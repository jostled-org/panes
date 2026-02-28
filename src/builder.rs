use crate::error::PaneError;
use crate::node::{NodeId, PanelId};
use crate::panel::Constraints;
use crate::tree::LayoutTree;

/// Newtype for gap values. Validated when consumed by `row()`/`col()`.
#[derive(Debug, Clone, Copy)]
pub struct Gap(pub(crate) f32);

/// Create a gap value for use in `row()` and `col()` containers.
pub fn gap(value: f32) -> Gap {
    Gap(value)
}

/// Reject NaN, negative, or infinite gap values.
fn validate_gap(value: f32) -> Result<(), PaneError> {
    match value {
        v if v.is_nan() => Err(PaneError::InvalidConstraint("gap is NaN".into())),
        v if v < 0.0 => Err(PaneError::InvalidConstraint("gap is negative".into())),
        v if v.is_infinite() => Err(PaneError::InvalidConstraint("gap is infinite".into())),
        _ => Ok(()),
    }
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
    pub fn new() -> Self {
        Self {
            tree: LayoutTree::new(),
            root_set: false,
        }
    }

    /// Create a panel, returning its `PanelId`. Hides the internal `NodeId`.
    pub fn panel(
        &mut self,
        kind: impl Into<std::sync::Arc<str>>,
        constraints: Constraints,
    ) -> Result<PanelId, PaneError> {
        let (pid, _nid) = self.tree.add_panel(kind, constraints)?;
        Ok(pid)
    }

    /// Set the root to a row container built by the closure.
    pub fn row(
        &mut self,
        gap: Gap,
        f: impl FnOnce(&mut ContainerCtx) -> Result<(), PaneError>,
    ) -> Result<(), PaneError> {
        self.require_no_root()?;
        validate_gap(gap.0)?;
        let children = collect_children(&mut self.tree, f)?;
        let nid = self.tree.add_row(gap.0, children)?;
        self.tree.set_root(nid);
        self.root_set = true;
        Ok(())
    }

    /// Set the root to a column container built by the closure.
    pub fn col(
        &mut self,
        gap: Gap,
        f: impl FnOnce(&mut ContainerCtx) -> Result<(), PaneError>,
    ) -> Result<(), PaneError> {
        self.require_no_root()?;
        validate_gap(gap.0)?;
        let children = collect_children(&mut self.tree, f)?;
        let nid = self.tree.add_col(gap.0, children)?;
        self.tree.set_root(nid);
        self.root_set = true;
        Ok(())
    }

    /// Consume the builder, validate the tree, and return a `Layout`.
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
pub struct ContainerCtx<'a> {
    tree: &'a mut LayoutTree,
    children: Vec<NodeId>,
}

impl ContainerCtx<'_> {
    /// Place a pre-created panel into this container.
    pub fn add(&mut self, pid: PanelId) -> Result<(), PaneError> {
        let nid = self
            .tree
            .node_for_panel(pid)
            .ok_or(PaneError::PanelNotFound(pid))?;
        self.children.push(nid);
        Ok(())
    }

    /// Create a panel inline and place it in this container.
    pub fn panel(
        &mut self,
        kind: impl Into<std::sync::Arc<str>>,
        constraints: Constraints,
    ) -> Result<PanelId, PaneError> {
        let (pid, nid) = self.tree.add_panel(kind, constraints)?;
        self.children.push(nid);
        Ok(pid)
    }

    /// Create a nested row container.
    pub fn row(
        &mut self,
        gap: Gap,
        f: impl FnOnce(&mut ContainerCtx) -> Result<(), PaneError>,
    ) -> Result<(), PaneError> {
        validate_gap(gap.0)?;
        let inner_children = collect_children(self.tree, f)?;
        let nid = self.tree.add_row(gap.0, inner_children)?;
        self.children.push(nid);
        Ok(())
    }

    /// Create a nested column container.
    pub fn col(
        &mut self,
        gap: Gap,
        f: impl FnOnce(&mut ContainerCtx) -> Result<(), PaneError>,
    ) -> Result<(), PaneError> {
        validate_gap(gap.0)?;
        let inner_children = collect_children(self.tree, f)?;
        let nid = self.tree.add_col(gap.0, inner_children)?;
        self.children.push(nid);
        Ok(())
    }

    /// Escape hatch: insert a raw Taffy node with a custom style.
    pub fn taffy_node(
        &mut self,
        style: taffy::Style,
        f: impl FnOnce(&mut ContainerCtx) -> Result<(), PaneError>,
    ) -> Result<(), PaneError> {
        let inner_children = collect_children(self.tree, f)?;
        let nid = self.tree.add_taffy_node(style, inner_children)?;
        self.children.push(nid);
        Ok(())
    }
}

/// Run a closure with a fresh `ContainerCtx`, collecting the children it produces.
fn collect_children(
    tree: &mut LayoutTree,
    f: impl FnOnce(&mut ContainerCtx) -> Result<(), PaneError>,
) -> Result<Vec<NodeId>, PaneError> {
    let mut ctx = ContainerCtx {
        tree,
        children: Vec::new(),
    };
    f(&mut ctx)?;
    Ok(ctx.children)
}
