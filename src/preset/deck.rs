use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::grow;
use crate::preset::{
    add_active_hidden_panels, col_style, collect_kinds, validate_active, validate_f32_param,
    validate_kinds, validate_share_param,
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

    crate::macros::builder_setters!(
        /// Set the master panel's share of the viewport.
        master_ratio(ratio: f32);
        /// Set which panel index is active (visible).
        active(index: usize);
        /// Set the gap between panels.
        gap(gap: f32)
    );

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        validate_kinds(&self.kinds)?;
        match self.kinds.len() > 1 {
            true => validate_active(self.active, self.kinds.len() - 1)?,
            false => {}
        }
        validate_share_param("master_ratio", self.master_ratio)?;
        validate_f32_param("gap", self.gap)?;

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

super::impl_preset!(
    Deck,
    runtime(kinds, |this| crate::strategy::StrategyKind::Deck {
        master_ratio: this.master_ratio,
        gap: this.gap,
    })
);
