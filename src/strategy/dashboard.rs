use std::sync::Arc;

use super::builder::{BoundStrategy, Strategy};
use super::{CardSpan, GridColumnMode, StrategyKind};

/// Builder for dashboard strategies.
#[derive(Debug, Clone)]
pub struct DashboardStrategy {
    pub(crate) columns: GridColumnMode,
    pub(crate) gap: f32,
    pub(crate) auto_rows: bool,
}

impl DashboardStrategy {
    /// Set a fixed number of columns.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = GridColumnMode::Fixed(columns);
        self
    }

    /// Use responsive `repeat(auto-fill, minmax(min_width, 1fr))` columns.
    pub fn auto_fill(mut self, min_width: f32) -> Self {
        self.columns = GridColumnMode::AutoFill { min_width };
        self
    }

    /// Use responsive `repeat(auto-fit, minmax(min_width, 1fr))` columns.
    pub fn auto_fit(mut self, min_width: f32) -> Self {
        self.columns = GridColumnMode::AutoFit { min_width };
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Use `grid-auto-rows: auto` so rows size to their tallest card.
    pub fn auto_rows(mut self) -> Self {
        self.auto_rows = true;
        self
    }

    /// Bind cards with explicit column spans.
    pub fn with_cards(
        self,
        cards: impl IntoIterator<Item = (impl Into<Arc<str>>, impl Into<CardSpan>)>,
    ) -> BoundStrategy {
        let cards: Vec<(Arc<str>, CardSpan)> = cards
            .into_iter()
            .map(|(k, s)| (k.into(), s.into()))
            .collect();
        let panels: Box<[Arc<str>]> = cards.iter().map(|(k, _)| Arc::clone(k)).collect();
        let spans: Arc<[CardSpan]> = cards.iter().map(|(_, s)| *s).collect();
        let kind = self.to_strategy_kind(spans);
        BoundStrategy::new(kind, panels, None)
    }

    /// Bind panels with all spans defaulting to 1.
    pub fn with_panels(
        self,
        panels: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> BoundStrategy {
        let panels: Box<[Arc<str>]> = panels.into_iter().map(Into::into).collect();
        let spans: Arc<[CardSpan]> = vec![CardSpan::Columns(1); panels.len()].into();
        let kind = self.to_strategy_kind(spans);
        BoundStrategy::new(kind, panels, None)
    }

    /// Convert to a generic [`Strategy`] with no cards bound yet.
    /// The resulting strategy will have empty spans — bind panels via
    /// [`Strategy::with_panels`] (all spans default to 1).
    pub fn build(self) -> Strategy {
        let spans: Arc<[CardSpan]> = Arc::from([]);
        Strategy::from_kind(self.to_strategy_kind(spans))
    }

    pub(crate) fn to_strategy_kind(&self, spans: Arc<[CardSpan]>) -> StrategyKind {
        self.columns
            .to_dashboard_strategy(self.gap, spans, self.auto_rows)
    }
}

impl From<DashboardStrategy> for Strategy {
    fn from(builder: DashboardStrategy) -> Self {
        builder.build()
    }
}
