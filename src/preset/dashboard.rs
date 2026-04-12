use std::sync::Arc;

use crate::builder::Grid;
use crate::error::{PaneError, TreeError};
use crate::layout::Layout;
use crate::strategy::{CardSpan, GridColumnMode};

/// Builder for the grid-based dashboard preset layout.
pub struct Dashboard {
    cards: Arc<[(Arc<str>, CardSpan)]>,
    columns: GridColumnMode,
    gap: f32,
    auto_rows: bool,
}

impl Dashboard {
    pub(crate) fn new(
        cards: impl IntoIterator<Item = (impl Into<Arc<str>>, impl Into<CardSpan>)>,
    ) -> Self {
        Self {
            cards: cards
                .into_iter()
                .map(|(k, span)| (k.into(), span.into()))
                .collect(),
            columns: GridColumnMode::Fixed(4),
            gap: 0.0,
            auto_rows: false,
        }
    }

    crate::macros::builder_mapped_setters!(
        /// Set a fixed number of columns.
        columns(columns: usize) -> columns = GridColumnMode::Fixed(columns);
        /// Use responsive `repeat(auto-fill, minmax(min_width, 1fr))` columns.
        auto_fill(min_width: f32) -> columns = GridColumnMode::AutoFill { min_width };
        /// Use responsive `repeat(auto-fit, minmax(min_width, 1fr))` columns.
        auto_fit(min_width: f32) -> columns = GridColumnMode::AutoFit { min_width }
    );

    crate::macros::builder_setters!(
        /// Set the gap between panels.
        gap(gap: f32)
    );

    crate::macros::builder_flag_setters!(
        /// Use `grid-auto-rows: auto` so rows size to their tallest card.
        auto_rows -> auto_rows = true
    );

    /// Build a [`Grid`] config from the current dashboard settings.
    fn to_grid(&self) -> Grid {
        Grid::from_column_mode(self.columns)
            .gap(self.gap)
            .apply_auto_rows(self.auto_rows)
    }

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        match self.cards.is_empty() {
            true => return Err(PaneError::InvalidTree(TreeError::DashboardNoCards)),
            false => {}
        }

        let cards = Arc::clone(&self.cards);
        Layout::build_grid(self.to_grid(), |g| {
            for (kind, span) in cards.iter() {
                g.panel_span(Arc::clone(kind), *span);
            }
        })
    }
}

impl Dashboard {
    /// Consume the builder and produce a [`crate::runtime::LayoutRuntime`].
    pub fn into_runtime(self) -> Result<crate::runtime::LayoutRuntime, PaneError> {
        let spans: Arc<[CardSpan]> = self.cards.iter().map(|(_, s)| *s).collect();
        let kinds: Box<[Arc<str>]> = self.cards.iter().map(|(k, _)| Arc::clone(k)).collect();
        let strategy = self
            .columns
            .to_dashboard_strategy(self.gap, spans, self.auto_rows);
        crate::runtime::LayoutRuntime::from_strategy(strategy, &kinds)
    }
}

super::impl_preset!(Dashboard);
