use std::sync::Arc;

use crate::error::PaneError;
use crate::layout::Layout;
use crate::preset::collect_kinds;
use crate::strategy::{CardSpan, GridColumnMode};

/// Builder for the equal-columns preset layout.
///
/// # Deprecated
/// Use [`Dashboard`](super::Dashboard) with span-1 cards instead.
#[deprecated(since = "0.12.0", note = "use Layout::dashboard() with span-1 cards")]
pub struct Columns {
    cols: GridColumnMode,
    kinds: Arc<[Arc<str>]>,
    gap: f32,
}

#[allow(deprecated)]
impl Columns {
    pub(crate) fn new(count: usize, kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self {
            cols: GridColumnMode::Fixed(count),
            kinds: collect_kinds(kinds),
            gap: 0.0,
        }
    }

    /// Use responsive `repeat(auto-fill, minmax(min_width, 1fr))` columns.
    pub fn auto_fill(mut self, min_width: f32) -> Self {
        self.cols = GridColumnMode::AutoFill { min_width };
        self
    }

    /// Use responsive `repeat(auto-fit, minmax(min_width, 1fr))` columns.
    pub fn auto_fit(mut self, min_width: f32) -> Self {
        self.cols = GridColumnMode::AutoFit { min_width };
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        self.as_dashboard().build()
    }

    /// Consume the builder and produce a [`crate::runtime::LayoutRuntime`].
    pub fn into_runtime(self) -> Result<crate::runtime::LayoutRuntime, PaneError> {
        self.as_dashboard().into_runtime()
    }

    fn as_dashboard(&self) -> super::Dashboard {
        let cards: Vec<(Arc<str>, CardSpan)> = self
            .kinds
            .iter()
            .map(|k| (Arc::clone(k), CardSpan::Columns(1)))
            .collect();
        let mut d = super::Dashboard::new(cards);
        d = match self.cols {
            GridColumnMode::Fixed(n) => d.columns(n),
            GridColumnMode::AutoFill { min_width } => d.auto_fill(min_width),
            GridColumnMode::AutoFit { min_width } => d.auto_fit(min_width),
        };
        d.gap(self.gap)
    }
}

super::impl_preset!(Columns);
