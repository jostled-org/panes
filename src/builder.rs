use crate::error::PaneError;
use crate::node::{NodeId, PanelId};
use crate::panel::Constraints;
use crate::tree::LayoutTree;

/// Reject NaN, negative, or infinite gap values.
fn validate_gap(value: f32) -> Result<(), PaneError> {
    match value {
        v if v.is_nan() => Err(PaneError::InvalidConstraint("gap is NaN".into())),
        v if v < 0.0 => Err(PaneError::InvalidConstraint("gap is negative".into())),
        v if v.is_infinite() => Err(PaneError::InvalidConstraint("gap is infinite".into())),
        _ => Ok(()),
    }
}

/// Sentinel PanelId returned when a `ContainerCtx` operation fails.
fn sentinel_panel() -> PanelId {
    PanelId::from_raw(u32::MAX)
}

/// Ergonomic builder for constructing layouts.
///
/// Users create panels, arrange them in `row()`/`col()` containers via closures,
/// then call `build()` to get a `Layout` ready for resolution.
pub struct LayoutBuilder {
    tree: LayoutTree,
    root_set: bool,
}

impl LayoutBuilder {
    /// Create an empty builder with no root set.
    pub fn new() -> Self {
        Self {
            tree: LayoutTree::new(),
            root_set: false,
        }
    }

    /// Create a panel with `grow(1.0)` default constraints.
    pub fn panel(&mut self, kind: impl Into<std::sync::Arc<str>>) -> Result<PanelId, PaneError> {
        self.panel_with(kind, crate::panel::grow(1.0))
    }

    /// Create a panel with explicit constraints.
    pub fn panel_with(
        &mut self,
        kind: impl Into<std::sync::Arc<str>>,
        constraints: Constraints,
    ) -> Result<PanelId, PaneError> {
        let (pid, _nid) = self.tree.add_panel(kind, constraints)?;
        Ok(pid)
    }

    /// Set the root to a row container with zero gap.
    pub fn row(&mut self, f: impl FnOnce(&mut ContainerCtx)) -> Result<(), PaneError> {
        self.row_gap(0.0, f)
    }

    /// Set the root to a row container with the specified gap.
    pub fn row_gap(
        &mut self,
        gap: f32,
        f: impl FnOnce(&mut ContainerCtx),
    ) -> Result<(), PaneError> {
        self.require_no_root()?;
        validate_gap(gap)?;
        let children = collect_children(&mut self.tree, f)?;
        let nid = self.tree.add_row(gap, children)?;
        self.tree.set_root(nid);
        self.root_set = true;
        Ok(())
    }

    /// Set the root to a column container with zero gap.
    pub fn col(&mut self, f: impl FnOnce(&mut ContainerCtx)) -> Result<(), PaneError> {
        self.col_gap(0.0, f)
    }

    /// Set the root to a column container with the specified gap.
    pub fn col_gap(
        &mut self,
        gap: f32,
        f: impl FnOnce(&mut ContainerCtx),
    ) -> Result<(), PaneError> {
        self.require_no_root()?;
        validate_gap(gap)?;
        let children = collect_children(&mut self.tree, f)?;
        let nid = self.tree.add_col(gap, children)?;
        self.tree.set_root(nid);
        self.root_set = true;
        Ok(())
    }

    /// Consume the builder, validate the tree, and return a `Layout`.
    /// Set how many panels the active window shows at once.
    pub fn set_window_size(&mut self, size: usize) {
        self.tree.set_window_size(size);
    }

    /// Validate the tree and produce a [`Layout`](crate::layout::Layout).
    pub fn build(self) -> Result<crate::layout::Layout, PaneError> {
        if !self.root_set {
            return Err(PaneError::InvalidTree("root is not set".into()));
        }
        self.tree.validate()?;
        Ok(crate::layout::Layout::from_tree(self.tree))
    }

    fn require_no_root(&self) -> Result<(), PaneError> {
        match self.root_set {
            true => Err(PaneError::InvalidTree("root already set".into())),
            false => Ok(()),
        }
    }
}

impl Default for LayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Closure context for building container children.
///
/// Errors are deferred: operations no-op after the first failure.
/// The error is surfaced when the parent collects children.
pub struct ContainerCtx<'a> {
    tree: &'a mut LayoutTree,
    children: Vec<NodeId>,
    error: Option<PaneError>,
}

impl ContainerCtx<'_> {
    /// Place a pre-created panel into this container.
    pub fn add(&mut self, pid: PanelId) {
        if self.error.is_some() {
            return;
        }
        match self.tree.node_for_panel(pid) {
            Some(nid) => self.children.push(nid),
            None => self.error = Some(PaneError::PanelNotFound(pid)),
        }
    }

    /// Create a panel inline with `grow(1.0)` default constraints.
    pub fn panel(&mut self, kind: impl Into<std::sync::Arc<str>>) -> PanelId {
        self.panel_with(kind, crate::panel::grow(1.0))
    }

    /// Create a panel inline with explicit constraints.
    pub fn panel_with(
        &mut self,
        kind: impl Into<std::sync::Arc<str>>,
        constraints: Constraints,
    ) -> PanelId {
        if self.error.is_some() {
            return sentinel_panel();
        }
        match self.tree.add_panel(kind, constraints) {
            Ok((pid, nid)) => {
                self.children.push(nid);
                pid
            }
            Err(e) => {
                self.error = Some(e);
                sentinel_panel()
            }
        }
    }

    /// Create a nested row container with zero gap.
    pub fn row(&mut self, f: impl FnOnce(&mut ContainerCtx)) {
        self.row_gap(0.0, f);
    }

    /// Create a nested row container with the specified gap.
    pub fn row_gap(&mut self, gap: f32, f: impl FnOnce(&mut ContainerCtx)) {
        if self.error.is_some() {
            return;
        }
        if let Err(e) = validate_gap(gap) {
            self.error = Some(e);
            return;
        }
        let children = match collect_children(self.tree, f) {
            Ok(c) => c,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        match self.tree.add_row(gap, children) {
            Ok(nid) => self.children.push(nid),
            Err(e) => self.error = Some(e),
        }
    }

    /// Create a nested column container with zero gap.
    pub fn col(&mut self, f: impl FnOnce(&mut ContainerCtx)) {
        self.col_gap(0.0, f);
    }

    /// Create a nested column container with the specified gap.
    pub fn col_gap(&mut self, gap: f32, f: impl FnOnce(&mut ContainerCtx)) {
        if self.error.is_some() {
            return;
        }
        if let Err(e) = validate_gap(gap) {
            self.error = Some(e);
            return;
        }
        let children = match collect_children(self.tree, f) {
            Ok(c) => c,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        match self.tree.add_col(gap, children) {
            Ok(nid) => self.children.push(nid),
            Err(e) => self.error = Some(e),
        }
    }

    /// Escape hatch: insert a raw Taffy node with a custom style.
    pub fn taffy_node(&mut self, style: taffy::Style, f: impl FnOnce(&mut ContainerCtx)) {
        if self.error.is_some() {
            return;
        }
        let children = match collect_children(self.tree, f) {
            Ok(c) => c,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        match self.tree.add_taffy_node(style, children) {
            Ok(nid) => self.children.push(nid),
            Err(e) => self.error = Some(e),
        }
    }

    /// Store an error in the deferred error slot.
    /// Subsequent operations will no-op.
    pub(crate) fn set_error(&mut self, err: PaneError) {
        if self.error.is_none() {
            self.error = Some(err);
        }
    }
}

/// Run a closure with a fresh `ContainerCtx`, collecting the children it produces.
fn collect_children(
    tree: &mut LayoutTree,
    f: impl FnOnce(&mut ContainerCtx),
) -> Result<Vec<NodeId>, PaneError> {
    let mut ctx = ContainerCtx {
        tree,
        children: Vec::new(),
        error: None,
    };
    f(&mut ctx);
    match ctx.error {
        Some(e) => Err(e),
        None => Ok(ctx.children),
    }
}
