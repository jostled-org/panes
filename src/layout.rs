use std::sync::Arc;

use crate::builder::{ContainerCtx, LayoutBuilder};
use crate::compiler::compile;
use crate::error::PaneError;
use crate::preset::PresetInfo;
use crate::resolver::ResolvedLayout;
use crate::tree::LayoutTree;

/// An immutable, validated layout ready for resolution.
pub struct Layout {
    tree: LayoutTree,
}

impl std::fmt::Debug for Layout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Layout")
            .field("panel_count", &self.tree.panel_count())
            .finish()
    }
}

impl Layout {
    /// Create a `Layout` from a validated tree. Called by `LayoutBuilder::build()`.
    pub(crate) fn from_tree(tree: LayoutTree) -> Self {
        Self { tree }
    }

    /// Borrow the underlying tree for read-only traversal.
    pub fn tree(&self) -> &LayoutTree {
        &self.tree
    }

    /// How many panels the active window shows at once.
    pub fn window_size(&self) -> usize {
        self.tree.window_size()
    }

    /// Compile, compute, and resolve the layout at the given viewport size.
    pub fn resolve(&self, width: f32, height: f32) -> Result<ResolvedLayout, PaneError> {
        self.tree.resolve(width, height)
    }

    // -- Convenience constructors --

    /// Build a row layout from a closure.
    pub fn build_row(f: impl FnOnce(&mut ContainerCtx)) -> Result<Self, PaneError> {
        let mut b = LayoutBuilder::new();
        b.row(f)?;
        b.build()
    }

    /// Build a column layout from a closure.
    pub fn build_col(f: impl FnOnce(&mut ContainerCtx)) -> Result<Self, PaneError> {
        let mut b = LayoutBuilder::new();
        b.col(f)?;
        b.build()
    }

    /// Build a row layout with gap from a closure.
    pub fn build_row_gap(gap: f32, f: impl FnOnce(&mut ContainerCtx)) -> Result<Self, PaneError> {
        let mut b = LayoutBuilder::new();
        b.row_gap(gap, f)?;
        b.build()
    }

    /// Build a column layout with gap from a closure.
    pub fn build_col_gap(gap: f32, f: impl FnOnce(&mut ContainerCtx)) -> Result<Self, PaneError> {
        let mut b = LayoutBuilder::new();
        b.col_gap(gap, f)?;
        b.build()
    }

    /// Equal-grow panels in a row, zero gap.
    pub fn row(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Result<Self, PaneError> {
        let kinds: Box<[Arc<str>]> = kinds.into_iter().map(Into::into).collect();
        let mut b = LayoutBuilder::new();
        b.row(|r| {
            for kind in &*kinds {
                r.panel(Arc::clone(kind));
            }
        })?;
        b.build()
    }

    /// Equal-grow panels in a column, zero gap.
    pub fn col(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Result<Self, PaneError> {
        let kinds: Box<[Arc<str>]> = kinds.into_iter().map(Into::into).collect();
        let mut b = LayoutBuilder::new();
        b.col(|r| {
            for kind in &*kinds {
                r.panel(Arc::clone(kind));
            }
        })?;
        b.build()
    }

    /// Panels in a row with explicit constraints per panel.
    pub fn row_with(
        panels: impl IntoIterator<Item = (impl Into<Arc<str>>, crate::panel::Constraints)>,
    ) -> Result<Self, PaneError> {
        let panels: Box<[_]> = panels.into_iter().map(|(k, c)| (k.into(), c)).collect();
        let mut b = LayoutBuilder::new();
        b.row(|r| {
            for (kind, constraints) in &*panels {
                r.panel_with(Arc::clone(kind), *constraints);
            }
        })?;
        b.build()
    }

    /// Panels in a column with explicit constraints per panel.
    pub fn col_with(
        panels: impl IntoIterator<Item = (impl Into<Arc<str>>, crate::panel::Constraints)>,
    ) -> Result<Self, PaneError> {
        let panels: Box<[_]> = panels.into_iter().map(|(k, c)| (k.into(), c)).collect();
        let mut b = LayoutBuilder::new();
        b.col(|c| {
            for (kind, constraints) in &*panels {
                c.panel_with(Arc::clone(kind), *constraints);
            }
        })?;
        b.build()
    }

    /// Return metadata for all built-in presets, sorted alphabetically by name.
    pub fn presets() -> &'static [PresetInfo] {
        &crate::preset::catalog::PRESETS
    }

    // -- Preset constructors --

    /// Create a [`MasterStack`](crate::preset::MasterStack) builder.
    pub fn master_stack(
        kinds: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> crate::preset::MasterStack {
        crate::preset::MasterStack::new(kinds)
    }

    /// Create a [`CenteredMaster`](crate::preset::CenteredMaster) builder.
    pub fn centered_master(
        kinds: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> crate::preset::CenteredMaster {
        crate::preset::CenteredMaster::new(kinds)
    }

    /// Create a [`Monocle`](crate::preset::Monocle) builder.
    pub fn monocle(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> crate::preset::Monocle {
        crate::preset::Monocle::new(kinds)
    }

    /// Create a [`Scrollable`](crate::preset::Scrollable) builder.
    pub fn scrollable(
        kinds: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> crate::preset::Scrollable {
        crate::preset::Scrollable::new(kinds)
    }

    /// Create a [`Dwindle`](crate::preset::Dwindle) builder.
    pub fn dwindle(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> crate::preset::Dwindle {
        crate::preset::Dwindle::new(kinds)
    }

    /// Create a [`Spiral`](crate::preset::Spiral) builder.
    pub fn spiral(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> crate::preset::Spiral {
        crate::preset::Spiral::new(kinds)
    }

    /// Create a [`Columns`](crate::preset::Columns) builder.
    ///
    /// # Deprecated
    /// Use [`Layout::dashboard`] with span-1 cards instead.
    #[deprecated(since = "0.12.0", note = "use Layout::dashboard() with span-1 cards")]
    #[allow(deprecated)]
    pub fn columns(
        count: usize,
        kinds: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> crate::preset::Columns {
        crate::preset::Columns::new(count, kinds)
    }

    /// Create a [`Deck`](crate::preset::Deck) builder.
    pub fn deck(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> crate::preset::Deck {
        crate::preset::Deck::new(kinds)
    }

    /// Create a [`Tabbed`](crate::preset::Tabbed) builder.
    pub fn tabbed(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> crate::preset::Tabbed {
        crate::preset::Tabbed::new(kinds)
    }

    /// Create a [`Stacked`](crate::preset::Stacked) builder.
    pub fn stacked(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> crate::preset::Stacked {
        crate::preset::Stacked::new(kinds)
    }

    /// Create a [`Sidebar`](crate::preset::Sidebar) builder.
    pub fn sidebar(
        sidebar_kind: impl Into<Arc<str>>,
        content_kind: impl Into<Arc<str>>,
    ) -> crate::preset::Sidebar {
        crate::preset::Sidebar::new(sidebar_kind, content_kind)
    }

    /// Create a [`HolyGrail`](crate::preset::HolyGrail) builder.
    pub fn holy_grail(
        header: impl Into<Arc<str>>,
        footer: impl Into<Arc<str>>,
        left: impl Into<Arc<str>>,
        main: impl Into<Arc<str>>,
        right: impl Into<Arc<str>>,
    ) -> crate::preset::HolyGrail {
        crate::preset::HolyGrail::new(header, footer, left, main, right)
    }

    /// Create a [`Dashboard`](crate::preset::Dashboard) builder.
    pub fn dashboard(
        cards: impl IntoIterator<Item = (impl Into<Arc<str>>, impl Into<crate::strategy::CardSpan>)>,
    ) -> crate::preset::Dashboard {
        crate::preset::Dashboard::new(cards)
    }

    /// Create a [`Split`](crate::preset::Split) builder.
    pub fn split(first: impl Into<Arc<str>>, second: impl Into<Arc<str>>) -> crate::preset::Split {
        crate::preset::Split::new(first, second)
    }

    /// Create an adaptive layout that switches strategies at width breakpoints.
    pub fn adaptive(
        panels: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> crate::breakpoint::AdaptiveBuilder {
        let panels: Vec<Arc<str>> = panels.into_iter().map(Into::into).collect();
        crate::breakpoint::AdaptiveBuilder::new(panels)
    }

    /// Create a [`Grid`](crate::preset::Grid) builder.
    ///
    /// # Deprecated
    /// Use [`Layout::dashboard`] with span-1 cards instead.
    #[deprecated(since = "0.12.0", note = "use Layout::dashboard() with span-1 cards")]
    #[allow(deprecated)]
    pub fn grid(
        cols: usize,
        kinds: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> crate::preset::Grid {
        crate::preset::Grid::new(cols, kinds)
    }
}

impl Layout {
    /// Parse a TOML configuration string into a `Layout`.
    #[cfg(feature = "toml")]
    pub fn from_toml(input: &str) -> Result<Self, crate::toml_parse::TomlError> {
        crate::toml_parse::parse(input)
    }

    /// Parse a TOML configuration string into a `LayoutRuntime`.
    ///
    /// Handles both single-strategy configs and adaptive configs with
    /// `[[layout.breakpoints]]`.
    #[cfg(feature = "toml")]
    pub fn from_toml_runtime(
        input: &str,
    ) -> Result<crate::runtime::LayoutRuntime, crate::toml_parse::TomlError> {
        crate::toml_parse::parse_runtime(input)
    }

    /// Read a TOML file from disk and parse it into a `Layout`.
    #[cfg(feature = "toml")]
    pub fn from_toml_file(
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, crate::toml_parse::TomlError> {
        let input = std::fs::read_to_string(path)?;
        crate::toml_parse::parse(&input)
    }

    /// Compile the layout tree into a Taffy tree ready for layout computation.
    pub fn compile(&self) -> Result<crate::compiler::CompileResult, PaneError> {
        compile(&self.tree)
    }
}

impl From<Layout> for LayoutTree {
    fn from(layout: Layout) -> Self {
        layout.tree
    }
}
