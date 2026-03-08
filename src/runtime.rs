use std::sync::Arc;

use crate::compiler::{CompileResult, compile, compute_layout};
use crate::diff::{self, LayoutDiff};
use crate::error::PaneError;
use crate::layout::Layout;
use crate::node::PanelId;
use crate::panel::fixed;
use crate::resolver::{self, ResolvedLayout};
use crate::sequence::PanelSequence;
use crate::strategy::StrategyKind;
use crate::tree::LayoutTree;
use crate::viewport::ViewportState;

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

    /// Resolve the layout at the given dimensions, producing a Frame with layout and diff.
    pub fn resolve(&mut self, width: f32, height: f32) -> Result<Frame, PaneError> {
        let mut result = match (self.tree.is_dirty(), self.cached_compile.take()) {
            (false, Some(cached)) => cached,
            _ => {
                self.tree.clear_dirty();
                compile(&self.tree)?
            }
        };

        compute_layout(&mut result, width, height)?;

        let mut layout = resolver::resolve(&result, &self.tree)?;

        self.cached_compile = Some(result);

        apply_scroll_offset(&mut layout, self.viewport.scroll_offset);

        let diff = match self.previous.as_deref() {
            Some(prev) => diff::diff(prev, &layout),
            None => diff::first_frame(&layout),
        };

        let layout = Arc::new(layout);
        self.previous = Some(Arc::clone(&layout));

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
