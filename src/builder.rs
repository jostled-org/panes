use std::sync::Arc;

use crate::decoration::DecorationRole;
use crate::error::{PaneError, TreeError};
use crate::node::{NodeId, PanelId};
use crate::panel::Constraints;
use crate::strategy::{CardSpan, GridColumnMode};
use crate::tree::LayoutTree;
use crate::validate::{check_f32_non_negative, float_invalid_to_constraint};

/// Reject NaN, negative, or infinite gap values.
fn validate_gap(value: f32) -> Result<(), PaneError> {
    check_f32_non_negative(value)
        .map_err(|e| PaneError::InvalidConstraint(float_invalid_to_constraint("gap", e)))
}

/// Sentinel PanelId returned when a `ContainerCtx` operation fails.
fn sentinel_panel() -> PanelId {
    PanelId::from_raw(u32::MAX)
}

/// Push a panel allocation result into a child list, or store the error.
///
/// Returns sentinel immediately when the error slot is already occupied,
/// ignoring the (already-evaluated) result.
fn try_alloc(
    children: &mut Vec<NodeId>,
    error: &mut Option<PaneError>,
    alloc: impl FnOnce() -> Result<(PanelId, NodeId), PaneError>,
) -> PanelId {
    if error.is_some() {
        return sentinel_panel();
    }
    match alloc() {
        Ok((pid, nid)) => {
            children.push(nid);
            pid
        }
        Err(e) => {
            *error = Some(e);
            sentinel_panel()
        }
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

    /// Set the root to a grid container with the given configuration.
    pub fn grid(&mut self, grid: Grid, f: impl FnOnce(&mut GridCtx)) -> Result<(), PaneError> {
        self.require_no_root()?;
        crate::preset::validate_grid_columns(grid.columns)?;
        validate_gap(grid.gap)?;

        let children = collect_grid_children(&mut self.tree, f)?;
        let nid = self
            .tree
            .add_grid(grid.columns, grid.gap, grid.auto_rows, children)?;
        self.tree.set_root(nid);
        self.root_set = true;
        Ok(())
    }

    /// Set how many panels are visible at once in the active window.
    /// Returns an error if `panel_count` is zero.
    pub fn set_window_panel_count(&mut self, panel_count: usize) -> Result<(), PaneError> {
        self.tree.set_window_panel_count(panel_count)
    }

    /// Validate the tree and produce a [`Layout`](crate::layout::Layout).
    pub fn build(self) -> Result<crate::layout::Layout, PaneError> {
        match self.root_set {
            false => return Err(PaneError::InvalidTree(TreeError::RootNotSet)),
            true => {}
        }
        self.tree.validate()?;
        Ok(crate::layout::Layout::from_tree(self.tree))
    }

    fn require_no_root(&self) -> Result<(), PaneError> {
        match self.root_set {
            true => Err(PaneError::InvalidTree(TreeError::RootAlreadySet)),
            false => Ok(()),
        }
    }
}

impl Default for LayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Closure context passed into `row()` / `col()` closures for accumulating children.
///
/// Each method adds a child node (panel, nested container, or grid) to the
/// parent being built. Errors are deferred: the first failure is stored and
/// all subsequent operations become no-ops. The stored error surfaces when
/// the parent builder method returns.
pub struct ContainerCtx<'a> {
    tree: &'a mut LayoutTree,
    children: Vec<NodeId>,
    error: Option<PaneError>,
}

impl ContainerCtx<'_> {
    /// Place a pre-created panel (from [`LayoutBuilder::panel`]) into this container.
    pub fn add(&mut self, pid: PanelId) {
        match self.error {
            Some(_) => return,
            None => {}
        }
        match self.tree.node_for_panel(pid) {
            Some(nid) => self.children.push(nid),
            None => self.error = Some(PaneError::PanelNotFound(pid)),
        }
    }

    /// Add a panel child with `grow(1.0)` default constraints. Returns its [`PanelId`].
    pub fn panel(&mut self, kind: impl Into<std::sync::Arc<str>>) -> PanelId {
        self.panel_with(kind, crate::panel::grow(1.0))
    }

    /// Add a panel child with explicit constraints. Returns its [`PanelId`].
    pub fn panel_with(
        &mut self,
        kind: impl Into<std::sync::Arc<str>>,
        constraints: Constraints,
    ) -> PanelId {
        let tree = &mut *self.tree;
        try_alloc(&mut self.children, &mut self.error, move || {
            tree.add_panel(kind, constraints)
        })
    }

    /// Create a decoration panel (tab bar, title bar) linked to a content kind.
    ///
    /// Decoration panels have geometry and PanelIds but do not appear in
    /// kind-based lookups (`by_kind`, `panels()`).
    pub(crate) fn decoration_panel(
        &mut self,
        content_kind: Arc<str>,
        constraints: Constraints,
        role: DecorationRole,
    ) -> PanelId {
        let tree = &mut *self.tree;
        try_alloc(&mut self.children, &mut self.error, move || {
            tree.add_decoration_panel(content_kind, constraints, role)
        })
    }

    /// Nest a row container with zero gap. Children are built in the closure.
    pub fn row(&mut self, f: impl FnOnce(&mut ContainerCtx)) {
        self.row_gap(0.0, f);
    }

    /// Nest a row container with the specified gap between children.
    pub fn row_gap(&mut self, gap: f32, f: impl FnOnce(&mut ContainerCtx)) {
        self.container_with_gap(gap, f, |tree, c| tree.add_row(gap, c));
    }

    /// Nest a row container with explicit constraints and zero gap.
    pub fn row_with(&mut self, constraints: Constraints, f: impl FnOnce(&mut ContainerCtx)) {
        self.row_gap_with(0.0, constraints, f);
    }

    /// Nest a row container with explicit constraints and the specified gap.
    pub fn row_gap_with(
        &mut self,
        gap: f32,
        constraints: Constraints,
        f: impl FnOnce(&mut ContainerCtx),
    ) {
        self.container_with_gap(gap, f, |tree, c| {
            tree.add_row_constrained(gap, Some(constraints), c)
        });
    }

    /// Nest a column container with zero gap. Children are built in the closure.
    pub fn col(&mut self, f: impl FnOnce(&mut ContainerCtx)) {
        self.col_gap(0.0, f);
    }

    /// Nest a column container with the specified gap between children.
    pub fn col_gap(&mut self, gap: f32, f: impl FnOnce(&mut ContainerCtx)) {
        self.container_with_gap(gap, f, |tree, c| tree.add_col(gap, c));
    }

    /// Nest a column container with explicit constraints and zero gap.
    pub fn col_with(&mut self, constraints: Constraints, f: impl FnOnce(&mut ContainerCtx)) {
        self.col_gap_with(0.0, constraints, f);
    }

    /// Nest a column container with explicit constraints and the specified gap.
    pub fn col_gap_with(
        &mut self,
        gap: f32,
        constraints: Constraints,
        f: impl FnOnce(&mut ContainerCtx),
    ) {
        self.container_with_gap(gap, f, |tree, c| {
            tree.add_col_constrained(gap, Some(constraints), c)
        });
    }

    /// Nest a grid container. Grid items are built in the closure via [`GridCtx`].
    pub fn grid(&mut self, grid: Grid, f: impl FnOnce(&mut GridCtx)) {
        build_nested_grid(self.tree, &mut self.children, &mut self.error, grid, f);
    }

    /// Insert a raw Taffy node with a custom style. Escape hatch for layouts
    /// that cannot be expressed through row/col/grid primitives.
    pub fn taffy_node(&mut self, style: taffy::Style, f: impl FnOnce(&mut ContainerCtx)) {
        self.add_container(f, |tree, c| tree.add_taffy_node(style, c));
    }

    fn container_with_gap(
        &mut self,
        gap: f32,
        f: impl FnOnce(&mut ContainerCtx),
        build: impl FnOnce(&mut LayoutTree, Vec<NodeId>) -> Result<NodeId, PaneError>,
    ) {
        match validate_gap(gap) {
            Err(e) => self.set_error(e),
            Ok(()) => self.add_container(f, build),
        }
    }

    fn add_container(
        &mut self,
        f: impl FnOnce(&mut ContainerCtx),
        build: impl FnOnce(&mut LayoutTree, Vec<NodeId>) -> Result<NodeId, PaneError>,
    ) {
        match self.error {
            Some(_) => return,
            None => {}
        }
        let children = match collect_children(self.tree, f) {
            Ok(c) => c,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        match build(self.tree, children) {
            Ok(nid) => self.children.push(nid),
            Err(e) => self.error = Some(e),
        }
    }

    /// Store an error in the deferred error slot.
    /// Subsequent operations will no-op.
    pub(crate) fn set_error(&mut self, err: PaneError) {
        match self.error {
            None => self.error = Some(err),
            Some(_) => {}
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

// ---------------------------------------------------------------------------
// Grid configuration and context
// ---------------------------------------------------------------------------

/// Configuration for a CSS Grid container node.
///
/// Describes column mode, gap, and row sizing. Constructed via named
/// constructors (`columns`, `auto_fill`, `auto_fit`) and builder methods.
#[derive(Debug, Clone, Copy)]
pub struct Grid {
    pub(crate) columns: GridColumnMode,
    pub(crate) gap: f32,
    pub(crate) auto_rows: bool,
}

impl Grid {
    /// Fixed number of equal-width columns.
    pub fn columns(n: usize) -> Self {
        Self::from_column_mode(GridColumnMode::Fixed(n))
    }

    /// Responsive columns via `repeat(auto-fill, minmax(min_width, 1fr))`.
    pub fn auto_fill(min_width: f32) -> Self {
        Self::from_column_mode(GridColumnMode::AutoFill { min_width })
    }

    /// Responsive columns via `repeat(auto-fit, minmax(min_width, 1fr))`.
    pub fn auto_fit(min_width: f32) -> Self {
        Self::from_column_mode(GridColumnMode::AutoFit { min_width })
    }

    /// Construct from an existing [`GridColumnMode`] directly.
    pub(crate) fn from_column_mode(columns: GridColumnMode) -> Self {
        Self {
            columns,
            gap: 0.0,
            auto_rows: false,
        }
    }

    /// Set the gap between grid items.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Use `grid-auto-rows: auto` so rows size to their tallest item.
    pub fn auto_rows(mut self) -> Self {
        self.auto_rows = true;
        self
    }

    /// Conditionally enable auto rows based on a runtime flag.
    pub(crate) fn apply_auto_rows(mut self, enabled: bool) -> Self {
        self.auto_rows = enabled;
        self
    }
}

/// Closure context passed into `grid()` closures for accumulating grid children.
///
/// Supports plain panels, panels with column spans, and nested row/col/grid
/// containers. Errors are deferred: the first failure is stored and all
/// subsequent operations become no-ops. The stored error surfaces when the
/// parent builder method returns.
pub struct GridCtx<'a> {
    tree: &'a mut LayoutTree,
    children: Vec<NodeId>,
    error: Option<PaneError>,
}

impl GridCtx<'_> {
    /// Add a grid item panel with `grow(1.0)` default constraints. Returns its [`PanelId`].
    pub fn panel(&mut self, kind: impl Into<Arc<str>>) -> PanelId {
        self.panel_with(kind, crate::panel::grow(1.0))
    }

    /// Add a grid item panel with explicit constraints. Returns its [`PanelId`].
    pub fn panel_with(&mut self, kind: impl Into<Arc<str>>, constraints: Constraints) -> PanelId {
        let tree = &mut *self.tree;
        try_alloc(&mut self.children, &mut self.error, move || {
            tree.add_panel(kind, constraints)
        })
    }

    /// Add a grid item panel with a column span and `grow(1.0)` default constraints.
    /// Returns its [`PanelId`].
    pub fn panel_span(&mut self, kind: impl Into<Arc<str>>, span: CardSpan) -> PanelId {
        self.panel_with_span(kind, crate::panel::grow(1.0), span)
    }

    /// Add a grid item panel with explicit constraints and a column span.
    /// Returns its [`PanelId`].
    pub fn panel_with_span(
        &mut self,
        kind: impl Into<Arc<str>>,
        constraints: Constraints,
        span: CardSpan,
    ) -> PanelId {
        match self.error {
            Some(_) => return sentinel_panel(),
            None => {}
        }
        match crate::preset::validate_grid_span(span) {
            Err(e) => {
                self.error = Some(e);
                return sentinel_panel();
            }
            Ok(()) => {}
        }
        let (pid, panel_nid) = match self.tree.add_panel(kind, constraints) {
            Ok(pair) => pair,
            Err(e) => {
                self.error = Some(e);
                return sentinel_panel();
            }
        };
        match self.tree.add_grid_item(span, panel_nid) {
            Ok(wrapper_nid) => {
                self.children.push(wrapper_nid);
                pid
            }
            Err(e) => self.fail_grid_item_wrapper(pid, e),
        }
    }

    fn fail_grid_item_wrapper(&mut self, pid: PanelId, error: PaneError) -> PanelId {
        let rollback = self.tree.remove_panel(pid);
        self.error = Some(match rollback {
            Ok(()) => error,
            Err(remove_error) => remove_error,
        });
        sentinel_panel()
    }

    /// Nest a row container with zero gap. Children are built in the closure.
    pub fn row(&mut self, f: impl FnOnce(&mut ContainerCtx)) {
        self.row_gap(0.0, f);
    }

    /// Nest a row container with the specified gap between children.
    pub fn row_gap(&mut self, gap: f32, f: impl FnOnce(&mut ContainerCtx)) {
        self.add_container_with_gap(gap, f, |tree, g, c| tree.add_row(g, c));
    }

    /// Nest a column container with zero gap. Children are built in the closure.
    pub fn col(&mut self, f: impl FnOnce(&mut ContainerCtx)) {
        self.col_gap(0.0, f);
    }

    /// Nest a column container with the specified gap between children.
    pub fn col_gap(&mut self, gap: f32, f: impl FnOnce(&mut ContainerCtx)) {
        self.add_container_with_gap(gap, f, |tree, g, c| tree.add_col(g, c));
    }

    /// Nest a row container with explicit constraints and the specified gap.
    pub fn row_gap_with(
        &mut self,
        gap: f32,
        constraints: Constraints,
        f: impl FnOnce(&mut ContainerCtx),
    ) {
        self.add_container_with_gap(gap, f, |tree, g, c| {
            tree.add_row_constrained(g, Some(constraints), c)
        });
    }

    /// Nest a column container with explicit constraints and the specified gap.
    pub fn col_gap_with(
        &mut self,
        gap: f32,
        constraints: Constraints,
        f: impl FnOnce(&mut ContainerCtx),
    ) {
        self.add_container_with_gap(gap, f, |tree, g, c| {
            tree.add_col_constrained(g, Some(constraints), c)
        });
    }

    /// Nest a grid container. Grid items are built in the closure via a new [`GridCtx`].
    pub fn grid(&mut self, grid: Grid, f: impl FnOnce(&mut GridCtx)) {
        build_nested_grid(self.tree, &mut self.children, &mut self.error, grid, f);
    }

    fn add_container_with_gap(
        &mut self,
        gap: f32,
        f: impl FnOnce(&mut ContainerCtx),
        build: impl FnOnce(&mut LayoutTree, f32, Vec<NodeId>) -> Result<NodeId, PaneError>,
    ) {
        match self.error {
            Some(_) => return,
            None => {}
        }
        match validate_gap(gap) {
            Err(e) => {
                self.error = Some(e);
                return;
            }
            Ok(()) => {}
        }
        let children = match collect_children(self.tree, f) {
            Ok(c) => c,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        match build(self.tree, gap, children) {
            Ok(nid) => self.children.push(nid),
            Err(e) => self.error = Some(e),
        }
    }
}

/// Validate grid config, collect children, and insert a grid node.
///
/// Shared by `ContainerCtx::grid` and `GridCtx::grid` to avoid duplication.
fn build_nested_grid(
    tree: &mut LayoutTree,
    children: &mut Vec<NodeId>,
    error: &mut Option<PaneError>,
    grid: Grid,
    f: impl FnOnce(&mut GridCtx),
) {
    match *error {
        Some(_) => return,
        None => {}
    }
    match crate::preset::validate_grid_columns(grid.columns) {
        Err(e) => {
            *error = Some(e);
            return;
        }
        Ok(()) => {}
    }
    match validate_gap(grid.gap) {
        Err(e) => {
            *error = Some(e);
            return;
        }
        Ok(()) => {}
    }
    let grid_children = match collect_grid_children(tree, f) {
        Ok(c) => c,
        Err(e) => {
            *error = Some(e);
            return;
        }
    };
    match tree.add_grid(grid.columns, grid.gap, grid.auto_rows, grid_children) {
        Ok(nid) => children.push(nid),
        Err(e) => *error = Some(e),
    }
}

/// Run a closure with a fresh `GridCtx`, collecting the children it produces.
fn collect_grid_children(
    tree: &mut LayoutTree,
    f: impl FnOnce(&mut GridCtx),
) -> Result<Vec<NodeId>, PaneError> {
    let mut ctx = GridCtx {
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
