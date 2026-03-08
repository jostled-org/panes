use std::sync::Arc;

use crate::compiler::{CompileResult, compile, compute_layout};
use crate::diff::{self, LayoutDiff};
use crate::error::PaneError;
use crate::focus::{self, FocusDirection};
use crate::layout::Layout;
use crate::node::{Node, PanelId};
use crate::panel::fixed;
use crate::rect::Rect;
use crate::resolver::{self, ResolveScratch, ResolvedLayout};
use crate::sequence::PanelSequence;
use crate::strategy::{Direction, StrategyKind};
use crate::tree::LayoutTree;
use crate::viewport::ViewportState;

/// Where to place the new panel relative to the focused panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    /// New panel goes before focused (left or above).
    Before,
    /// New panel goes after focused (right or below).
    #[default]
    After,
}

/// Result of a single resolve call: the resolved layout and its diff against the previous frame.
pub struct Frame {
    layout: Arc<ResolvedLayout>,
    diff: LayoutDiff,
}

impl Frame {
    /// The resolved layout for this frame.
    pub fn layout(&self) -> &ResolvedLayout {
        &self.layout
    }

    /// The diff between this frame and the previous one.
    pub fn diff(&self) -> &LayoutDiff {
        &self.diff
    }
}

/// Stateful layout wrapper that tracks tree, viewport, and frame history.
pub struct LayoutRuntime {
    tree: LayoutTree,
    viewport: ViewportState,
    previous: Option<Arc<ResolvedLayout>>,
    cached_compile: Option<CompileResult>,
    cached_kinds: Option<resolver::KindIndex>,
    rects_buf: Option<Vec<Option<Rect>>>,
    diff_scratch: diff::DiffScratch,
    resolve_scratch: ResolveScratch,
    strategy: Option<StrategyKind>,
    sequence: PanelSequence,
}

impl LayoutRuntime {
    /// Create a runtime from an existing tree (legacy path, no strategy).
    pub fn new(tree: LayoutTree) -> Self {
        Self {
            tree,
            viewport: ViewportState::default(),
            previous: None,
            cached_compile: None,
            cached_kinds: None,
            rects_buf: None,
            diff_scratch: diff::DiffScratch::default(),
            resolve_scratch: ResolveScratch::default(),
            strategy: None,
            sequence: PanelSequence::default(),
        }
    }

    /// Create a runtime from a strategy and initial panel kinds.
    pub fn from_strategy(strategy: StrategyKind, kinds: &[Arc<str>]) -> Result<Self, PaneError> {
        let mut sequence = PanelSequence::default();
        let mut viewport = ViewportState::default();
        let tree = crate::strategy::build_initial(&strategy, kinds, &mut sequence, &mut viewport)?;
        Ok(Self {
            tree,
            viewport,
            previous: None,
            cached_compile: None,
            cached_kinds: None,
            rects_buf: None,
            diff_scratch: diff::DiffScratch::default(),
            resolve_scratch: ResolveScratch::default(),
            strategy: Some(strategy),
            sequence,
        })
    }

    /// Create a runtime from a pre-built tree and a strategy.
    /// Populates the sequence by looking up each kind in the tree.
    pub fn from_tree_and_strategy(
        tree: LayoutTree,
        strategy: StrategyKind,
        kinds: &[Arc<str>],
    ) -> Result<Self, PaneError> {
        let mut sequence = PanelSequence::default();
        for kind in kinds {
            for &pid in tree.panels_by_kind(kind) {
                sequence.push(pid);
            }
        }
        let focus = sequence.get(0);
        Ok(Self {
            tree,
            viewport: ViewportState {
                focus,
                ..ViewportState::default()
            },
            previous: None,
            cached_compile: None,
            cached_kinds: None,
            rects_buf: None,
            diff_scratch: diff::DiffScratch::default(),
            resolve_scratch: ResolveScratch::default(),
            strategy: Some(strategy),
            sequence,
        })
    }

    /// Immutable access to the underlying tree.
    pub fn tree(&self) -> &LayoutTree {
        &self.tree
    }

    /// Mutable access to the underlying tree for structural mutations.
    pub fn tree_mut(&mut self) -> &mut LayoutTree {
        &mut self.tree
    }

    /// Immutable access to the viewport state.
    pub fn viewport(&self) -> &ViewportState {
        &self.viewport
    }

    /// Toggle a panel's collapsed state.
    ///
    /// Collapsing saves the current constraints and sets the panel to fixed(0.0).
    /// Uncollapsing restores the saved constraints.
    pub fn toggle_collapsed(&mut self, pid: PanelId) -> Result<(), PaneError> {
        match self.viewport.collapsed.contains(&pid) {
            true => {
                let saved = self
                    .viewport
                    .saved_constraints
                    .remove(&pid)
                    .ok_or_else(|| {
                        PaneError::InvalidViewport(
                            format!("no saved constraints for panel {pid}").into(),
                        )
                    })?;
                self.tree.set_constraints(pid, saved)?;
                self.viewport.collapsed.remove(&pid);
                Ok(())
            }
            false => {
                let current = self.tree.panel_constraints(pid)?;
                self.viewport.saved_constraints.insert(pid, current);
                self.tree.set_constraints(pid, fixed(0.0))?;
                self.viewport.collapsed.insert(pid);
                Ok(())
            }
        }
    }

    /// Shift the scroll offset by a delta.
    pub fn scroll_by(&mut self, delta: f32) {
        self.viewport.scroll_offset += delta;
    }

    /// Set the scroll offset to an absolute value.
    pub fn scroll_to(&mut self, offset: f32) {
        self.viewport.scroll_offset = offset;
    }

    /// Set the active panel.
    pub fn set_active(&mut self, pid: PanelId) {
        self.viewport.focus = Some(pid);
    }

    /// Get the currently active panel, if any.
    pub fn active_panel(&self) -> Option<PanelId> {
        self.viewport.focus
    }

    /// The layout strategy, if this runtime was created via `from_strategy`.
    pub fn strategy(&self) -> Option<&StrategyKind> {
        self.strategy.as_ref()
    }

    /// The panel sequence (logical order).
    pub fn sequence(&self) -> &PanelSequence {
        &self.sequence
    }

    /// The currently focused panel.
    pub fn focused(&self) -> Option<PanelId> {
        self.viewport.focus
    }

    /// The kind of the currently focused panel.
    pub fn focused_kind(&self) -> Option<&str> {
        let pid = self.viewport.focus?;
        self.tree.panel_kind(pid).ok()
    }

    /// Whether `pid` is a decorative panel (tab bar, title bar) for `content_pid`.
    pub fn is_decoration_for(&self, pid: PanelId, content_pid: PanelId) -> bool {
        let (Ok(dec_kind), Ok(content_kind)) =
            (self.tree.panel_kind(pid), self.tree.panel_kind(content_pid))
        else {
            return false;
        };
        let base = dec_kind
            .strip_suffix("_tab")
            .or_else(|| dec_kind.strip_suffix("_title"));
        matches!(base, Some(b) if b == content_kind)
    }

    /// Add a panel using the active strategy.
    pub fn add_panel(&mut self, kind: Arc<str>) -> Result<PanelId, PaneError> {
        let strategy = self
            .strategy
            .as_ref()
            .ok_or_else(|| PaneError::InvalidMutation("no strategy set".into()))?
            .clone();
        crate::strategy::apply_add(
            &strategy,
            &mut self.tree,
            &mut self.sequence,
            &mut self.viewport,
            kind,
        )
    }

    /// Remove a panel using the active strategy. Returns the new focus panel.
    pub fn remove_panel(&mut self, pid: PanelId) -> Result<Option<PanelId>, PaneError> {
        let strategy = self
            .strategy
            .as_ref()
            .ok_or_else(|| PaneError::InvalidMutation("no strategy set".into()))?
            .clone();
        crate::strategy::apply_remove(
            &strategy,
            &mut self.tree,
            &mut self.sequence,
            &mut self.viewport,
            pid,
        )
    }

    /// Move a panel to a new sequence index using the active strategy.
    pub fn move_panel(&mut self, pid: PanelId, new_index: usize) -> Result<PanelId, PaneError> {
        let strategy = self
            .strategy
            .as_ref()
            .ok_or_else(|| PaneError::InvalidMutation("no strategy set".into()))?
            .clone();
        crate::strategy::apply_move(
            &strategy,
            &mut self.tree,
            &mut self.sequence,
            &mut self.viewport,
            pid,
            new_index,
        )
    }

    /// Set focus to a specific panel using the active strategy.
    pub fn focus(&mut self, pid: PanelId) -> Result<(), PaneError> {
        match &self.strategy {
            Some(strategy) => {
                let strategy = strategy.clone();
                crate::strategy::apply_focus(
                    &strategy,
                    &mut self.tree,
                    &mut self.sequence,
                    &mut self.viewport,
                    pid,
                )
            }
            None => {
                self.viewport.focus = Some(pid);
                Ok(())
            }
        }
    }

    /// Move focus to the next panel in the sequence.
    pub fn focus_next(&mut self) -> Result<(), PaneError> {
        let next = match self.viewport.focus {
            Some(current) => {
                let idx = self.sequence.index_of(current).unwrap_or(0);
                let next_idx = (idx + 1) % self.sequence.len().max(1);
                self.sequence.get(next_idx)
            }
            None => self.sequence.get(0),
        };
        match next {
            Some(pid) => self.focus(pid),
            None => Ok(()),
        }
    }

    /// Move focus to the previous panel in the sequence.
    pub fn focus_prev(&mut self) -> Result<(), PaneError> {
        let prev = match self.viewport.focus {
            Some(current) => {
                let len = self.sequence.len().max(1);
                let idx = self.sequence.index_of(current).unwrap_or(0);
                let prev_idx = (idx + len - 1) % len;
                self.sequence.get(prev_idx)
            }
            None => self.sequence.get(0),
        };
        match prev {
            Some(pid) => self.focus(pid),
            None => Ok(()),
        }
    }

    /// Move focus to the nearest panel in a spatial direction.
    ///
    /// Returns `Ok(Some(target))` when focus moved, `Ok(None)` when no
    /// candidate exists in that direction or no panel is focused.
    pub fn focus_direction(
        &mut self,
        layout: &ResolvedLayout,
        direction: FocusDirection,
    ) -> Result<Option<PanelId>, PaneError> {
        let focused = match self.focused() {
            Some(pid) => pid,
            None => return Ok(None),
        };
        match focus::find_nearest(layout, focused, &self.sequence, direction) {
            Some(target) => {
                self.focus(target)?;
                Ok(Some(target))
            }
            None => Ok(None),
        }
    }

    /// Move focus to the nearest panel in a spatial direction, using the
    /// most recently resolved layout.
    ///
    /// Equivalent to [`focus_direction`](Self::focus_direction) but reads
    /// geometry from the cached layout so the caller doesn't need to pass it.
    /// Requires at least one prior [`resolve`](Self::resolve) call.
    pub fn focus_direction_current(
        &mut self,
        direction: FocusDirection,
    ) -> Result<Option<PanelId>, PaneError> {
        let layout = Arc::clone(self.previous.as_ref().ok_or_else(|| {
            PaneError::InvalidViewport("no resolved layout; call resolve() first".into())
        })?);
        self.focus_direction(&layout, direction)
    }

    /// Pick a split direction from the focused panel's aspect ratio.
    /// Splits the longer axis: wider → horizontal, taller → vertical.
    /// Falls back to horizontal if no layout is cached or no panel is focused.
    fn auto_direction(&self) -> Direction {
        let rect = self
            .viewport
            .focus
            .and_then(|pid| self.previous.as_ref()?.get(pid));
        match rect {
            Some(r) if r.h > r.w => Direction::Vertical,
            _ => Direction::Horizontal,
        }
    }

    /// Add a panel adjacent to the currently focused panel.
    ///
    /// Auto-picks the split direction from the focused panel's aspect ratio:
    /// wider panels split horizontal, taller panels split vertical. Falls
    /// back to horizontal if no layout has been resolved yet.
    ///
    /// Uses `grow(1.0)` constraints and [`Placement::After`].
    pub fn add_panel_adjacent(&mut self, kind: Arc<str>) -> Result<PanelId, PaneError> {
        let direction = self.auto_direction();
        self.add_panel_adjacent_with(kind, direction, crate::panel::grow(1.0), Placement::After)
    }

    /// Add a panel adjacent to the currently focused panel with full control.
    ///
    /// This is strategy-independent: it works directly on tree topology.
    /// `placement` controls whether the new panel appears before or after
    /// the focused panel. If `direction` matches the parent container's
    /// axis, the new panel is inserted as a sibling. If it conflicts, the
    /// focused panel is wrapped in a new sub-container oriented along
    /// `direction`, and the new panel is added beside it.
    pub fn add_panel_adjacent_with(
        &mut self,
        kind: Arc<str>,
        direction: Direction,
        constraints: crate::Constraints,
        placement: Placement,
    ) -> Result<PanelId, PaneError> {
        let focused = self
            .focused()
            .ok_or_else(|| PaneError::InvalidMutation("no focused panel".into()))?;
        let focused_nid = self
            .tree
            .node_for_panel(focused)
            .ok_or(PaneError::PanelNotFound(focused))?;
        let parent_id = self
            .tree
            .parent(focused_nid)?
            .ok_or_else(|| PaneError::InvalidMutation("focused panel has no parent".into()))?;

        let (new_pid, new_nid) = self.tree.add_panel(kind, constraints)?;

        let parent_axis = match self.tree.node(parent_id) {
            Some(Node::Col { .. }) => Direction::Vertical,
            Some(Node::TaffyPassthrough { style, .. })
                if matches!(
                    style.flex_direction,
                    taffy::FlexDirection::Column | taffy::FlexDirection::ColumnReverse
                ) =>
            {
                Direction::Vertical
            }
            Some(Node::Row { .. }) | Some(Node::TaffyPassthrough { .. }) => Direction::Horizontal,
            Some(Node::Panel { .. }) | None => {
                return Err(PaneError::InvalidMutation(
                    "parent is not a container".into(),
                ));
            }
        };

        let focused_idx = self
            .tree
            .children(parent_id)?
            .iter()
            .position(|&c| c == focused_nid)
            .ok_or(PaneError::PanelNotFound(focused))?;

        let insert_idx = match placement {
            Placement::Before => focused_idx,
            Placement::After => focused_idx + 1,
        };

        match (parent_axis == direction, direction, placement) {
            (true, _, _) => {
                self.tree.insert_child_at(parent_id, insert_idx, new_nid)?;
            }
            (false, Direction::Horizontal, Placement::Before) => {
                self.tree.detach(focused_nid);
                let c = self.tree.add_row(0.0, vec![new_nid, focused_nid])?;
                self.tree.insert_child_at(parent_id, focused_idx, c)?;
            }
            (false, Direction::Horizontal, Placement::After) => {
                self.tree.detach(focused_nid);
                let c = self.tree.add_row(0.0, vec![focused_nid, new_nid])?;
                self.tree.insert_child_at(parent_id, focused_idx, c)?;
            }
            (false, Direction::Vertical, Placement::Before) => {
                self.tree.detach(focused_nid);
                let c = self.tree.add_col(0.0, vec![new_nid, focused_nid])?;
                self.tree.insert_child_at(parent_id, focused_idx, c)?;
            }
            (false, Direction::Vertical, Placement::After) => {
                self.tree.detach(focused_nid);
                let c = self.tree.add_col(0.0, vec![focused_nid, new_nid])?;
                self.tree.insert_child_at(parent_id, focused_idx, c)?;
            }
        }

        let seq_idx = match (self.sequence.index_of(focused), placement) {
            (Some(idx), Placement::Before) => idx,
            (Some(idx), Placement::After) => idx + 1,
            (None, _) => self.sequence.len(),
        };
        self.sequence.insert(seq_idx, new_pid);

        self.viewport.focus = Some(new_pid);
        Ok(new_pid)
    }

    /// Resolve the layout at the given dimensions, producing a Frame with layout and diff.
    pub fn resolve(&mut self, width: f32, height: f32) -> Result<Frame, PaneError> {
        let tree_dirty = self.tree.is_dirty();

        let mut result = match (tree_dirty, self.cached_compile.take()) {
            (false, Some(cached)) => cached,
            _ => {
                self.tree.clear_dirty();
                compile(&self.tree)?
            }
        };

        compute_layout(&mut result, width, height)?;

        let mut layout = match (tree_dirty, self.cached_kinds.take()) {
            (false, Some(kinds)) => resolver::resolve_with_cached_kinds(
                &result,
                &self.tree,
                kinds,
                &mut self.resolve_scratch,
                self.rects_buf.take(),
            )?,
            _ => resolver::resolve(&result, &self.tree)?,
        };

        self.cached_kinds = Some(Arc::clone(layout.kinds_arc()));
        self.cached_compile = Some(result);

        apply_scroll_offset(&mut layout, self.viewport.scroll_offset);

        let prev_arc = self.previous.take();
        let diff = match (tree_dirty, prev_arc.as_deref()) {
            (_, None) => diff::first_frame(&layout),
            (false, Some(prev)) => diff::diff_same_panels(prev, &layout),
            (true, Some(prev)) => diff::diff_reuse(prev, &layout, &mut self.diff_scratch),
        };

        let layout = Arc::new(layout);
        self.previous = Some(Arc::clone(&layout));

        // Reclaim the previous frame's rects buffer if no other consumers hold a reference.
        if let Some(Ok(mut prev_layout)) = prev_arc.map(Arc::try_unwrap) {
            self.rects_buf = Some(prev_layout.take_rects());
        }

        Ok(Frame { layout, diff })
    }
}

impl From<LayoutTree> for LayoutRuntime {
    fn from(tree: LayoutTree) -> Self {
        Self::new(tree)
    }
}

/// Shift all resolved rect x-positions by the negative scroll offset.
fn apply_scroll_offset(layout: &mut ResolvedLayout, offset: f32) {
    match offset.abs() < f32::EPSILON {
        true => {}
        false => layout.shift_x(-offset),
    }
}

impl From<Layout> for LayoutRuntime {
    fn from(layout: Layout) -> Self {
        Self::new(LayoutTree::from(layout))
    }
}
