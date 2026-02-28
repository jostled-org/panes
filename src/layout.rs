use std::sync::Arc;

use crate::builder::{LayoutBuilder, gap};
use crate::compiler::compile;
use crate::error::PaneError;
use crate::panel::grow;
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

    /// Compile, compute, and resolve the layout at the given viewport size.
    pub fn resolve(&self, width: f32, height: f32) -> Result<ResolvedLayout, PaneError> {
        self.tree.resolve(width, height)
    }

    /// Equal-grow panels in a row, zero gap.
    pub fn row(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Result<Self, PaneError> {
        let kinds: Vec<Arc<str>> = kinds.into_iter().map(Into::into).collect();
        let mut b = LayoutBuilder::new();
        b.row(gap(0.0), |r| {
            for kind in &kinds {
                r.panel(Arc::clone(kind), grow(1.0))?;
            }
            Ok(())
        })?;
        b.build()
    }

    /// Equal-grow panels in a column, zero gap.
    pub fn col(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Result<Self, PaneError> {
        let kinds: Vec<Arc<str>> = kinds.into_iter().map(Into::into).collect();
        let mut b = LayoutBuilder::new();
        b.col(gap(0.0), |r| {
            for kind in &kinds {
                r.panel(Arc::clone(kind), grow(1.0))?;
            }
            Ok(())
        })?;
        b.build()
    }

    // -- Preset constructors --

    pub fn master_stack(
        kinds: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> crate::preset::MasterStack {
        crate::preset::MasterStack::new(kinds)
    }

    pub fn centered_master(
        kinds: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> crate::preset::CenteredMaster {
        crate::preset::CenteredMaster::new(kinds)
    }

    pub fn monocle(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> crate::preset::Monocle {
        crate::preset::Monocle::new(kinds)
    }

    pub fn scrollable(
        kinds: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> crate::preset::Scrollable {
        crate::preset::Scrollable::new(kinds)
    }

    pub fn dwindle(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> crate::preset::Dwindle {
        crate::preset::Dwindle::new(kinds)
    }

    pub fn spiral(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> crate::preset::Spiral {
        crate::preset::Spiral::new(kinds)
    }

    pub fn columns(
        count: usize,
        kinds: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> crate::preset::Columns {
        crate::preset::Columns::new(count, kinds)
    }

    pub fn deck(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> crate::preset::Deck {
        crate::preset::Deck::new(kinds)
    }

    pub fn tabbed(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> crate::preset::Tabbed {
        crate::preset::Tabbed::new(kinds)
    }

    pub fn stacked(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> crate::preset::Stacked {
        crate::preset::Stacked::new(kinds)
    }

    pub fn sidebar(
        sidebar_kind: impl Into<Arc<str>>,
        content_kind: impl Into<Arc<str>>,
    ) -> crate::preset::Sidebar {
        crate::preset::Sidebar::new(sidebar_kind, content_kind)
    }

    pub fn holy_grail(
        header: impl Into<Arc<str>>,
        footer: impl Into<Arc<str>>,
        left: impl Into<Arc<str>>,
        main: impl Into<Arc<str>>,
        right: impl Into<Arc<str>>,
    ) -> crate::preset::HolyGrail {
        crate::preset::HolyGrail::new(header, footer, left, main, right)
    }

    pub fn dashboard(
        cards: impl IntoIterator<Item = (impl Into<Arc<str>>, usize)>,
    ) -> crate::preset::Dashboard {
        crate::preset::Dashboard::new(cards)
    }

    pub fn split(first: impl Into<Arc<str>>, second: impl Into<Arc<str>>) -> crate::preset::Split {
        crate::preset::Split::new(first, second)
    }

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
