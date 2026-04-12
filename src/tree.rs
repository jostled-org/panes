use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;
use std::{cell::Cell, thread_local};

use crate::decoration::{DecorationMeta, DecorationRole};
use crate::error::{MutationError, PaneError, TreeError};
use crate::node::Node;
use crate::node::PanelId;
use crate::strategy::{CardSpan, GridColumnMode};
use crate::{Constraints, NodeId, PanelIdGenerator};

/// Relative position for inserting or moving nodes within a container.
#[derive(Debug, Clone, Copy)]
pub enum Position {
    /// Insert after the given panel.
    After(PanelId),
    /// Insert before the given panel.
    Before(PanelId),
}

impl Position {
    fn anchor_and_offset(self) -> (PanelId, usize) {
        match self {
            Self::After(a) => (a, 1),
            Self::Before(a) => (a, 0),
        }
    }
}

/// Arena-based mutable layout tree.
pub struct LayoutTree {
    nodes: Vec<Option<Node>>,
    free_list: Vec<NodeId>,
    root: Option<NodeId>,
    panel_gen: PanelIdGenerator,
    kind_index: FxHashMap<Arc<str>, Vec<PanelId>>,
    panel_to_node: FxHashMap<PanelId, NodeId>,
    parent_map: FxHashMap<NodeId, NodeId>,
    /// Decoration metadata keyed by the decoration panel's `PanelId`.
    decorations: FxHashMap<PanelId, DecorationMeta>,
    dirty: bool,
    live_count: usize,
    window_panel_count: usize,
}

thread_local! {
    static DEBUG_NODE_ALLOC_FAILURE_REMAINING: Cell<usize> = const { Cell::new(usize::MAX) };
}

#[doc(hidden)]
pub struct DebugNodeAllocFailureGuard;

impl Drop for DebugNodeAllocFailureGuard {
    fn drop(&mut self) {
        DEBUG_NODE_ALLOC_FAILURE_REMAINING.with(|remaining| remaining.set(usize::MAX));
    }
}

impl LayoutTree {
    /// Create an empty tree with no nodes.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            free_list: Vec::new(),
            root: None,
            panel_gen: PanelIdGenerator::new(),
            kind_index: FxHashMap::default(),
            panel_to_node: FxHashMap::default(),
            parent_map: FxHashMap::default(),
            decorations: FxHashMap::default(),
            dirty: true,
            live_count: 0,
            window_panel_count: 1,
        }
    }

    /// Allocate a new node in the arena, returning its `NodeId`.
    fn alloc(&mut self, node: Node) -> Result<NodeId, PaneError> {
        Self::debug_maybe_fail_node_alloc()?;
        let id = match self.free_list.pop() {
            Some(id) => {
                self.nodes[id.raw() as usize] = Some(node);
                id
            }
            None => {
                let id = NodeId::from_raw(
                    u32::try_from(self.nodes.len())
                        .map_err(|_| PaneError::InvalidTree(TreeError::ArenaOverflow))?,
                );
                self.nodes.push(Some(node));
                id
            }
        };
        self.live_count += 1;
        Ok(id)
    }

    fn debug_maybe_fail_node_alloc() -> Result<(), PaneError> {
        DEBUG_NODE_ALLOC_FAILURE_REMAINING.with(|remaining| match remaining.get() {
            usize::MAX => Ok(()),
            0 => Err(PaneError::InvalidTree(TreeError::ArenaOverflow)),
            value => {
                remaining.set(value - 1);
                Ok(())
            }
        })
    }

    #[doc(hidden)]
    pub fn debug_fail_nth_node_alloc(
        successful_allocations_before_failure: usize,
    ) -> DebugNodeAllocFailureGuard {
        DEBUG_NODE_ALLOC_FAILURE_REMAINING
            .with(|remaining| remaining.set(successful_allocations_before_failure));
        DebugNodeAllocFailureGuard
    }

    /// Shared panel allocation: validate, generate id, create node, alloc, register.
    fn alloc_panel(
        &mut self,
        kind: Arc<str>,
        constraints: Constraints,
    ) -> Result<(PanelId, NodeId), PaneError> {
        constraints.validate()?;
        let pid = self.panel_gen.next_id()?;
        let node = Node::Panel {
            id: pid,
            kind,
            constraints,
        };
        let nid = self.alloc(node)?;
        self.panel_to_node.insert(pid, nid);
        self.dirty = true;
        Ok((pid, nid))
    }

    /// Add a panel node. Returns the generated `PanelId` and the arena `NodeId`.
    ///
    /// The kind string must not be empty.
    pub fn add_panel(
        &mut self,
        kind: impl Into<Arc<str>>,
        constraints: Constraints,
    ) -> Result<(PanelId, NodeId), PaneError> {
        let kind: Arc<str> = kind.into();
        validate_kind(&kind)?;
        let (pid, nid) = self.alloc_panel(Arc::clone(&kind), constraints)?;
        self.kind_index.entry(kind).or_default().push(pid);
        Ok((pid, nid))
    }

    /// Add a decoration panel node. Like `add_panel` but stores decoration
    /// metadata and does NOT add the panel to the kind index.
    pub(crate) fn add_decoration_panel(
        &mut self,
        content_kind: Arc<str>,
        constraints: Constraints,
        role: DecorationRole,
    ) -> Result<(PanelId, NodeId), PaneError> {
        let (pid, nid) = self.alloc_panel(Arc::clone(&content_kind), constraints)?;
        self.decorations
            .insert(pid, DecorationMeta { role, content_kind });
        Ok((pid, nid))
    }

    /// Whether this panel is a decoration (tab bar, title bar).
    pub fn is_decoration(&self, pid: PanelId) -> bool {
        self.decorations.contains_key(&pid)
    }

    /// Decoration metadata for a panel, if it is a decoration.
    pub(crate) fn decoration_meta(&self, pid: PanelId) -> Option<&DecorationMeta> {
        self.decorations.get(&pid)
    }

    /// The decoration role for a panel, if it is a decoration.
    pub fn decoration_role(&self, pid: PanelId) -> Option<DecorationRole> {
        self.decorations.get(&pid).map(|m| m.role)
    }

    /// All decoration entries. Used by the resolver to propagate metadata.
    pub(crate) fn decoration_entries(
        &self,
    ) -> impl Iterator<Item = (PanelId, &DecorationMeta)> + '_ {
        self.decorations.iter().map(|(&pid, meta)| (pid, meta))
    }

    /// Record parent links for all children of a node in the arena.
    /// Takes split borrows to avoid conflicting with `&self.nodes`.
    fn record_children_from(
        parent_map: &mut FxHashMap<NodeId, NodeId>,
        nodes: &[Option<Node>],
        parent: NodeId,
    ) {
        let children = nodes
            .get(parent.raw() as usize)
            .and_then(|slot| slot.as_ref())
            .map(|n| n.children())
            .unwrap_or(&[]);
        for &child in children {
            parent_map.insert(child, parent);
        }
    }

    /// Add a row container with the given gap and children.
    pub fn add_row(&mut self, gap: f32, children: Vec<NodeId>) -> Result<NodeId, PaneError> {
        self.add_row_constrained(gap, None, children)
    }

    /// Add a row container with optional constraints, gap, and children.
    pub fn add_row_constrained(
        &mut self,
        gap: f32,
        constraints: Option<Constraints>,
        children: Vec<NodeId>,
    ) -> Result<NodeId, PaneError> {
        if let Some(ref c) = constraints {
            c.validate()?;
        }
        let id = self.alloc(Node::Row {
            gap,
            constraints,
            children,
        })?;
        Self::record_children_from(&mut self.parent_map, &self.nodes, id);
        self.dirty = true;
        Ok(id)
    }

    /// Add a column container with the given gap and children.
    pub fn add_col(&mut self, gap: f32, children: Vec<NodeId>) -> Result<NodeId, PaneError> {
        self.add_col_constrained(gap, None, children)
    }

    /// Add a column container with optional constraints, gap, and children.
    pub fn add_col_constrained(
        &mut self,
        gap: f32,
        constraints: Option<Constraints>,
        children: Vec<NodeId>,
    ) -> Result<NodeId, PaneError> {
        if let Some(ref c) = constraints {
            c.validate()?;
        }
        let id = self.alloc(Node::Col {
            gap,
            constraints,
            children,
        })?;
        Self::record_children_from(&mut self.parent_map, &self.nodes, id);
        self.dirty = true;
        Ok(id)
    }

    /// Add a raw Taffy passthrough node.
    pub fn add_taffy_node(
        &mut self,
        style: taffy::Style,
        children: Vec<NodeId>,
    ) -> Result<NodeId, PaneError> {
        let id = self.alloc(Node::TaffyPassthrough {
            style: Box::new(style),
            children: children.into_boxed_slice(),
        })?;
        Self::record_children_from(&mut self.parent_map, &self.nodes, id);
        self.dirty = true;
        Ok(id)
    }

    /// Add a grid container node directly.
    pub(crate) fn add_grid(
        &mut self,
        columns: GridColumnMode,
        gap: f32,
        auto_rows: bool,
        children: Vec<NodeId>,
    ) -> Result<NodeId, PaneError> {
        let id = self.alloc(Node::Grid {
            columns,
            gap,
            auto_rows,
            children,
        })?;
        Self::record_children_from(&mut self.parent_map, &self.nodes, id);
        self.dirty = true;
        Ok(id)
    }

    /// Add a grid item wrapper node with a column span.
    pub(crate) fn add_grid_item(
        &mut self,
        span: CardSpan,
        child: NodeId,
    ) -> Result<NodeId, PaneError> {
        let id = self.alloc(Node::GridItemWrapper { span, child })?;
        self.parent_map.insert(child, id);
        self.dirty = true;
        Ok(id)
    }

    /// Set the root node of the tree.
    pub fn set_root(&mut self, id: NodeId) {
        self.root = Some(id);
    }

    /// Return the root node id, if set.
    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    /// Look up a node by id.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes
            .get(id.raw() as usize)
            .and_then(|slot| slot.as_ref())
    }

    /// Whether the tree structure has changed since last compile.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clear the dirty flag after a successful compile.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// One past the highest issued `PanelId`. Used to size Vec-indexed storage.
    pub fn panel_id_high_water(&self) -> u32 {
        self.panel_gen.high_water()
    }

    /// Total slots in the node arena (including tombstones).
    pub fn arena_len(&self) -> usize {
        self.nodes.len()
    }

    /// Total number of live nodes in the arena.
    pub fn node_count(&self) -> usize {
        self.live_count
    }

    /// Total number of panel nodes in the tree.
    pub fn panel_count(&self) -> usize {
        self.panel_to_node.len()
    }

    /// How many panels the active window shows at once.
    /// Default is 1. Scrollable sets this to 2.
    pub fn window_panel_count(&self) -> usize {
        self.window_panel_count
    }

    /// Set how many panels are visible at once in the active window.
    /// Returns an error if `panel_count` is zero.
    pub fn set_window_panel_count(&mut self, panel_count: usize) -> Result<(), PaneError> {
        match panel_count {
            0 => Err(PaneError::InvalidTree(TreeError::WindowSizeZero)),
            _ => {
                self.window_panel_count = panel_count;
                Ok(())
            }
        }
    }

    /// Total number of distinct panel kinds.
    pub fn kind_count(&self) -> usize {
        self.kind_index.len()
    }

    /// All distinct panel kinds.
    pub fn kinds(&self) -> impl Iterator<Item = &Arc<str>> {
        self.kind_index.keys()
    }

    /// All `PanelId`s with the given kind.
    pub fn panels_by_kind(&self, kind: &str) -> &[PanelId] {
        match self.kind_index.get(kind) {
            Some(ids) => ids,
            None => &[],
        }
    }

    /// Update a panel's constraints.
    pub fn set_constraints(
        &mut self,
        pid: PanelId,
        constraints: Constraints,
    ) -> Result<(), PaneError> {
        constraints.validate()?;
        let nid = self.resolve_panel(pid)?;
        match self
            .nodes
            .get_mut(nid.raw() as usize)
            .and_then(|slot| slot.as_mut())
        {
            Some(Node::Panel { constraints: c, .. }) => {
                *c = constraints;
                self.dirty = true;
                Ok(())
            }
            _ => Err(PaneError::PanelNotFound(pid)),
        }
    }

    /// Update the `flex_grow` on a `TaffyPassthrough` node.
    pub(crate) fn set_node_flex_grow(&mut self, nid: NodeId, value: f32) -> Result<(), PaneError> {
        match self
            .nodes
            .get_mut(nid.raw() as usize)
            .and_then(|s| s.as_mut())
        {
            Some(Node::TaffyPassthrough { style, .. }) => {
                style.flex_grow = value;
                self.dirty = true;
                Ok(())
            }
            _ => Err(PaneError::NodeNotFound(nid)),
        }
    }

    /// Get a panel's current constraints.
    pub fn panel_constraints(&self, pid: PanelId) -> Result<Constraints, PaneError> {
        let nid = self.resolve_panel(pid)?;
        match self.node(nid) {
            Some(Node::Panel { constraints, .. }) => Ok(*constraints),
            _ => Err(PaneError::PanelNotFound(pid)),
        }
    }

    /// Get a panel's kind.
    pub fn panel_kind(&self, pid: PanelId) -> Result<&str, PaneError> {
        let nid = self.resolve_panel(pid)?;
        match self.node(nid) {
            Some(Node::Panel { kind, .. }) => Ok(kind),
            _ => Err(PaneError::PanelNotFound(pid)),
        }
    }

    /// Get a panel's kind as a cheap `Arc::clone` instead of allocating.
    pub fn panel_kind_arc(&self, pid: PanelId) -> Result<Arc<str>, PaneError> {
        let nid = self.resolve_panel(pid)?;
        match self.node(nid) {
            Some(Node::Panel { kind, .. }) => Ok(Arc::clone(kind)),
            _ => Err(PaneError::PanelNotFound(pid)),
        }
    }

    /// Get the children of a node. Returns an empty slice for leaf nodes.
    pub fn children(&self, id: NodeId) -> Result<&[NodeId], PaneError> {
        match self.node(id) {
            Some(node) => Ok(node.children()),
            None => Err(PaneError::NodeNotFound(id)),
        }
    }

    /// Get the parent of a node. Returns `None` for root nodes.
    pub fn parent(&self, id: NodeId) -> Result<Option<NodeId>, PaneError> {
        match self.node(id) {
            Some(_) => Ok(self.parent_map.get(&id).copied()),
            None => Err(PaneError::NodeNotFound(id)),
        }
    }

    /// Resolve a `PanelId` to its arena `NodeId`, if present.
    pub fn node_for_panel(&self, pid: PanelId) -> Option<NodeId> {
        self.panel_to_node.get(&pid).copied()
    }

    /// Resolve a `PanelId` to its arena `NodeId`.
    fn resolve_panel(&self, pid: PanelId) -> Result<NodeId, PaneError> {
        self.panel_to_node
            .get(&pid)
            .copied()
            .ok_or(PaneError::PanelNotFound(pid))
    }

    /// Mutable access to a container's children list.
    fn children_mut(&mut self, id: NodeId) -> Option<&mut Vec<NodeId>> {
        self.nodes
            .get_mut(id.raw() as usize)?
            .as_mut()?
            .children_mut()
    }

    /// Remove a `PanelId` from the kind index, dropping the entry if empty.
    fn remove_from_kind_index(&mut self, kind: &Arc<str>, pid: PanelId) {
        let is_empty = self
            .kind_index
            .get_mut(kind)
            .map(|ids| {
                ids.retain(|&p| p != pid);
                ids.is_empty()
            })
            .unwrap_or(false);
        if is_empty {
            self.kind_index.remove(kind);
        }
    }

    /// Drop an unattached non-panel node from the arena.
    pub(crate) fn remove_orphan_node(&mut self, node_id: NodeId) -> Result<(), PaneError> {
        match self.parent(node_id)? {
            Some(parent) => Err(PaneError::InvalidTree(TreeError::ParentChildMismatch {
                parent,
                child: node_id,
            })),
            None => self.drop_unattached_non_panel(node_id),
        }
    }

    fn drop_unattached_non_panel(&mut self, node_id: NodeId) -> Result<(), PaneError> {
        let node = self.node(node_id).ok_or(PaneError::NodeNotFound(node_id))?;
        match node {
            Node::Panel { .. } => Err(PaneError::NodeNotFound(node_id)),
            Node::Row { .. }
            | Node::Col { .. }
            | Node::Grid { .. }
            | Node::GridItemWrapper { .. }
            | Node::TaffyPassthrough { .. } => {
                let child_ids: Vec<_> = node.children().to_vec();
                for child_id in child_ids {
                    self.parent_map.remove(&child_id);
                }
                self.nodes[node_id.raw() as usize] = None;
                self.free_list.push(node_id);
                self.live_count = self.live_count.saturating_sub(1);
                self.dirty = true;
                Ok(())
            }
        }
    }

    /// Detach a node from its parent container. Returns the parent id.
    pub(crate) fn detach(&mut self, node_id: NodeId) -> Option<NodeId> {
        let parent_id = self.parent_map.remove(&node_id)?;
        if let Some(children) = self.children_mut(parent_id) {
            children.retain(|&c| c != node_id);
        }
        Some(parent_id)
    }

    /// Restore a child's parent link after rollback.
    pub(crate) fn restore_parent(&mut self, node_id: NodeId, parent_id: NodeId) {
        self.parent_map.insert(node_id, parent_id);
        self.dirty = true;
    }

    /// Insert a child into a container at the given index, updating parent_map.
    pub fn insert_child_at(
        &mut self,
        container: NodeId,
        idx: usize,
        child: NodeId,
    ) -> Result<(), PaneError> {
        let children = match self.children_mut(container) {
            Some(c) => c,
            None => return Err(PaneError::NodeNotFound(container)),
        };
        match idx > children.len() {
            true => Err(PaneError::InvalidTree(TreeError::InsertOutOfBounds {
                index: idx,
                len: children.len(),
            })),
            false => {
                children.insert(idx, child);
                self.parent_map.insert(child, container);
                self.dirty = true;
                Ok(())
            }
        }
    }

    /// Find the position index in a container for the given anchor panel.
    fn anchor_index(
        &self,
        container_id: NodeId,
        anchor: PanelId,
        offset: usize,
    ) -> Result<usize, PaneError> {
        let anchor_nid = self.resolve_panel(anchor)?;
        let children = self
            .children(container_id)
            .map_err(|_| PaneError::NodeNotFound(container_id))?;
        children
            .iter()
            .position(|&c| c == anchor_nid)
            .map(|i| i + offset)
            .ok_or(PaneError::NodeNotFound(anchor_nid))
    }

    /// Find the insertion index after moving a node out of its current parent.
    fn move_index(
        &self,
        node_id: NodeId,
        container_id: NodeId,
        anchor: PanelId,
        offset: usize,
    ) -> Result<usize, PaneError> {
        if self.parent(node_id)? != Some(container_id) {
            return self.anchor_index(container_id, anchor, offset);
        }

        let anchor_nid = self.resolve_panel(anchor)?;
        let children = self
            .children(container_id)
            .map_err(|_| PaneError::NodeNotFound(container_id))?;
        let mut insertion_index = 0;

        for &child_id in children {
            match (child_id == node_id, child_id == anchor_nid) {
                (true, _) => continue,
                (_, true) => return Ok(insertion_index + offset),
                (false, false) => insertion_index += 1,
            }
        }

        Err(PaneError::NodeNotFound(anchor_nid))
    }

    /// Remove a panel from the tree entirely.
    pub fn remove_panel(&mut self, pid: PanelId) -> Result<(), PaneError> {
        let nid = self.resolve_panel(pid)?;

        if self.root == Some(nid) {
            self.root = None;
        }

        // Remove from parent's children
        self.detach(nid);

        // Remove kind index entry
        let kind = match self.node(nid) {
            Some(Node::Panel { kind, .. }) => Arc::clone(kind),
            _ => return Err(PaneError::PanelNotFound(pid)),
        };
        self.remove_from_kind_index(&kind, pid);

        // Remove from panel-to-node map, decorations, and arena
        self.panel_to_node.remove(&pid);
        self.decorations.remove(&pid);
        self.nodes[nid.raw() as usize] = None;
        self.free_list.push(nid);
        self.live_count = self.live_count.saturating_sub(1);
        self.dirty = true;

        Ok(())
    }

    /// Move a panel to a new position (possibly in a different container).
    pub fn move_panel(&mut self, pid: PanelId, position: Position) -> Result<(), PaneError> {
        let nid = self.resolve_panel(pid)?;
        let (anchor, offset) = position.anchor_and_offset();
        let source_parent = self
            .parent(nid)?
            .ok_or(PaneError::InvalidMutation(MutationError::PanelNoParent))?;
        let source_index = self
            .children(source_parent)?
            .iter()
            .position(|&child_id| child_id == nid)
            .ok_or(PaneError::NodeNotFound(nid))?;

        let target_container = self
            .panel_to_node
            .get(&anchor)
            .and_then(|&anid| self.parent_map.get(&anid).copied())
            .ok_or(PaneError::PanelNotFound(anchor))?;

        let idx = self.move_index(nid, target_container, anchor, offset)?;

        // Detach from current parent
        self.detach(nid);

        self.dirty = true;
        match self.insert_child_at(target_container, idx, nid) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.insert_child_at(source_parent, source_index, nid)?;
                Err(error)
            }
        }
    }

    /// Check structural integrity of the tree.
    pub fn validate(&self) -> Result<(), PaneError> {
        let root_id = self
            .root
            .ok_or(PaneError::InvalidTree(TreeError::RootNotSet))?;

        self.node(root_id)
            .ok_or(PaneError::InvalidTree(TreeError::RootMissing(root_id)))?;

        let live: FxHashSet<NodeId> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| {
                slot.as_ref().map(|_| {
                    u32::try_from(i)
                        .map(NodeId::from_raw)
                        .map_err(|_| PaneError::InvalidTree(TreeError::ArenaIndexOverflow))
                })
            })
            .collect::<Result<_, _>>()?;

        self.validate_children(&live)?;
        self.validate_parent_links(root_id, &live)?;
        self.validate_reachability(root_id, &live)
    }

    /// Every child referenced by a container must exist in the arena.
    fn validate_children(&self, live: &FxHashSet<NodeId>) -> Result<(), PaneError> {
        for &nid in live {
            let Some(node) = self.node(nid) else { continue };
            for &child in node.children() {
                if !live.contains(&child) {
                    return Err(PaneError::InvalidTree(TreeError::MissingChild {
                        parent: nid,
                        child,
                    }));
                }
            }
        }
        Ok(())
    }

    /// Every non-root live node must have a parent that lists it as a child.
    fn validate_parent_links(
        &self,
        root_id: NodeId,
        live: &FxHashSet<NodeId>,
    ) -> Result<(), PaneError> {
        for &nid in live {
            if nid == root_id {
                continue;
            }
            let parent_id = self
                .parent_map
                .get(&nid)
                .copied()
                .ok_or(PaneError::InvalidTree(TreeError::NoParentEntry(nid)))?;
            let parent_children = self
                .node(parent_id)
                .ok_or(PaneError::InvalidTree(TreeError::ParentMissing {
                    parent: parent_id,
                    child: nid,
                }))?
                .children();
            if !parent_children.contains(&nid) {
                return Err(PaneError::InvalidTree(TreeError::ParentChildMismatch {
                    parent: parent_id,
                    child: nid,
                }));
            }
        }
        Ok(())
    }

    /// Every live node must be reachable from the root exactly once.
    fn validate_reachability(
        &self,
        root_id: NodeId,
        live: &FxHashSet<NodeId>,
    ) -> Result<(), PaneError> {
        let mut visited = FxHashSet::default();
        let mut child_owners = FxHashMap::default();
        let mut stack = vec![root_id];

        while let Some(node_id) = stack.pop() {
            if !visited.insert(node_id) {
                continue;
            }

            let node = self
                .node(node_id)
                .ok_or(PaneError::InvalidTree(TreeError::RootMissing(node_id)))?;

            for &child_id in node.children() {
                if let Some(first_parent) = child_owners.insert(child_id, node_id) {
                    return Err(PaneError::InvalidTree(
                        TreeError::ChildListedMultipleTimes {
                            child: child_id,
                            first_parent,
                            second_parent: node_id,
                        },
                    ));
                }

                stack.push(child_id);
            }
        }

        for &node_id in live {
            if !visited.contains(&node_id) {
                return Err(PaneError::InvalidTree(TreeError::DisconnectedNode(node_id)));
            }
        }

        Ok(())
    }

    /// Insert a node into a container at a position relative to an anchor.
    pub fn insert_node(
        &mut self,
        node_id: NodeId,
        container_id: NodeId,
        position: Position,
    ) -> Result<(), PaneError> {
        self.node(node_id).ok_or(PaneError::NodeNotFound(node_id))?;
        let (anchor, offset) = position.anchor_and_offset();
        let idx = self.anchor_index(container_id, anchor, offset)?;
        self.dirty = true;
        self.insert_child_at(container_id, idx, node_id)
    }

    /// Compile, compute, and resolve in one call.
    pub fn resolve(
        &self,
        width: f32,
        height: f32,
    ) -> Result<crate::resolver::ResolvedLayout, PaneError> {
        let mut result = crate::compiler::compile(self)?;
        crate::compiler::compute_layout(&mut result, width, height)?;
        crate::resolver::resolve(&result, self)
    }
}

fn validate_kind(kind: &str) -> Result<(), PaneError> {
    match kind.is_empty() {
        true => Err(PaneError::InvalidTree(TreeError::EmptyKind)),
        false => Ok(()),
    }
}

impl Default for LayoutTree {
    fn default() -> Self {
        Self::new()
    }
}
