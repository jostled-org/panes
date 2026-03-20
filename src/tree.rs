use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;

use crate::error::{PaneError, TreeError};
use crate::node::Node;
use crate::node::PanelId;
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
    dirty: bool,
    live_count: usize,
    window_size: usize,
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
            dirty: true,
            live_count: 0,
            window_size: 1,
        }
    }

    /// Allocate a new node in the arena, returning its `NodeId`.
    fn alloc(&mut self, node: Node) -> Result<NodeId, PaneError> {
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

    /// Add a panel node. Returns the generated `PanelId` and the arena `NodeId`.
    pub fn add_panel(
        &mut self,
        kind: impl Into<Arc<str>>,
        constraints: Constraints,
    ) -> Result<(PanelId, NodeId), PaneError> {
        constraints.validate()?;
        let pid = self.panel_gen.next_id()?;
        let kind: Arc<str> = kind.into();
        let node = Node::Panel {
            id: pid,
            kind: Arc::clone(&kind),
            constraints,
        };
        let nid = self.alloc(node)?;
        self.kind_index.entry(kind).or_default().push(pid);
        self.panel_to_node.insert(pid, nid);
        self.dirty = true;
        Ok((pid, nid))
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
        let id = self.alloc(Node::Row { gap, children })?;
        Self::record_children_from(&mut self.parent_map, &self.nodes, id);
        self.dirty = true;
        Ok(id)
    }

    /// Add a column container with the given gap and children.
    pub fn add_col(&mut self, gap: f32, children: Vec<NodeId>) -> Result<NodeId, PaneError> {
        let id = self.alloc(Node::Col { gap, children })?;
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
    pub fn window_size(&self) -> usize {
        self.window_size
    }

    /// Set the active window size. Returns an error if `size` is zero.
    pub fn set_window_size(&mut self, size: usize) -> Result<(), PaneError> {
        match size {
            0 => Err(PaneError::InvalidTree(TreeError::WindowSizeZero)),
            _ => {
                self.window_size = size;
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

    /// Detach a node from its parent container. Returns the parent id.
    pub(crate) fn detach(&mut self, node_id: NodeId) -> Option<NodeId> {
        let parent_id = self.parent_map.remove(&node_id)?;
        if let Some(children) = self.children_mut(parent_id) {
            children.retain(|&c| c != node_id);
        }
        Some(parent_id)
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

    /// Remove a panel from the tree entirely.
    pub fn remove_panel(&mut self, pid: PanelId) -> Result<(), PaneError> {
        let nid = self.resolve_panel(pid)?;

        // Remove from parent's children
        self.detach(nid);

        // Remove kind index entry
        let kind = match self.node(nid) {
            Some(Node::Panel { kind, .. }) => Arc::clone(kind),
            _ => return Err(PaneError::PanelNotFound(pid)),
        };
        self.remove_from_kind_index(&kind, pid);

        // Remove from panel-to-node map and arena
        self.panel_to_node.remove(&pid);
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

        let target_container = self
            .panel_to_node
            .get(&anchor)
            .and_then(|&anid| self.parent_map.get(&anid).copied())
            .ok_or(PaneError::PanelNotFound(anchor))?;

        // Detach from current parent
        self.detach(nid);

        // Find insertion index in target container
        let idx = self.anchor_index(target_container, anchor, offset)?;

        self.dirty = true;
        self.insert_child_at(target_container, idx, nid)
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
        self.validate_parents(root_id, &live)
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
    fn validate_parents(&self, root_id: NodeId, live: &FxHashSet<NodeId>) -> Result<(), PaneError> {
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

impl Default for LayoutTree {
    fn default() -> Self {
        Self::new()
    }
}
