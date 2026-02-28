use std::sync::Arc;

use crate::compiler::{compile, compute_layout, CompileResult};
use crate::diff::{self, LayoutDiff};
use crate::error::PaneError;
use crate::layout::Layout;
use crate::node::PanelId;
use crate::panel::fixed;
use crate::resolver::{self, ResolvedLayout};
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
}

impl LayoutRuntime {
    pub fn new(tree: LayoutTree) -> Self {
        Self {
            tree,
            viewport: ViewportState::default(),
            previous: None,
            cached_compile: None,
        }
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
        self.viewport.active_panel = Some(pid);
    }

    /// Get the currently active panel, if any.
    pub fn active_panel(&self) -> Option<PanelId> {
        self.viewport.active_panel
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
