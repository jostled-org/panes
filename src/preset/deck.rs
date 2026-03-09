use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::grow;
use crate::preset::master_stack::col_style;
use crate::preset::{
    add_active_hidden_panels, collect_kinds, validate_active, validate_f32_param, validate_kinds,
};

/// Builder for the deck preset layout.
pub struct Deck {
    kinds: Arc<[Arc<str>]>,
    master_ratio: f32,
    active: usize,
    gap: f32,
}

impl Deck {
    pub(crate) fn new(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self {
            kinds: collect_kinds(kinds),
            master_ratio: 0.5,
            active: 0,
            gap: 0.0,
        }
    }

    /// Set the master panel's share of the viewport.
    pub fn master_ratio(mut self, ratio: f32) -> Self {
        self.master_ratio = ratio;
        self
    }

    /// Set which panel index is active (visible).
    pub fn active(mut self, index: usize) -> Self {
        self.active = index;
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        validate_kinds(&self.kinds)?;
        if self.kinds.len() > 1 {
            validate_active(self.active, self.kinds.len() - 1)?;
        }
        validate_f32_param("master_ratio", self.master_ratio)?;

        let mut b = LayoutBuilder::new();
        let ratio = self.master_ratio;
        let gap_px = self.gap;
        let master_kind = Arc::clone(&self.kinds[0]);
        let active = self.active;

        b.row_gap(gap_px, |r| {
            r.panel_with(master_kind, grow(ratio));
            r.taffy_node(col_style(1.0 - ratio, 0.0), |c| {
                add_active_hidden_panels(c, &self.kinds[1..], active);
            });
        })?;

        b.build()
    }
}

impl Deck {
    /// Consume the builder and produce a [`crate::runtime::LayoutRuntime`].
    pub fn into_runtime(self) -> Result<crate::runtime::LayoutRuntime, PaneError> {
        let strategy = crate::strategy::StrategyKind::Deck {
            master_ratio: self.master_ratio,
            gap: self.gap,
        };
        crate::runtime::LayoutRuntime::from_strategy(strategy, &self.kinds)
    }
}

super::impl_preset!(Deck);
