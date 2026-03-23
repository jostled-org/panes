use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::fixed;
use crate::preset::{
    add_active_hidden_panels, collect_kinds, validate_active, validate_f32_param, validate_kinds,
};

/// Builder for the stacked preset layout.
pub struct Stacked {
    kinds: Arc<[Arc<str>]>,
    active: usize,
    bar_height: f32,
    gap: f32,
}

impl Stacked {
    pub(crate) fn new(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self {
            kinds: collect_kinds(kinds),
            active: 0,
            bar_height: 1.0,
            gap: 0.0,
        }
    }

    crate::macros::builder_setters!(
        /// Set which panel index is active (visible).
        active(index: usize);
        /// Set the title bar height.
        bar_height(height: f32);
        /// Set the gap between panels.
        gap(gap: f32)
    );

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        validate_kinds(&self.kinds)?;
        validate_active(self.active, self.kinds.len())?;
        validate_f32_param("bar_height", self.bar_height)?;
        validate_f32_param("gap", self.gap)?;

        let mut b = LayoutBuilder::new();
        let gap_px = self.gap;
        let title_h = self.bar_height;
        let active = self.active;

        b.col_gap(gap_px, |c| {
            add_stacked_panels(c, &self.kinds, active, title_h);
        })?;

        b.build()
    }
}

fn add_stacked_panels(
    ctx: &mut crate::ContainerCtx,
    kinds: &[Arc<str>],
    active: usize,
    title_height: f32,
) {
    for kind in kinds {
        let title_kind: Arc<str> = format!("{kind}_title").into();
        ctx.panel_with(title_kind, fixed(title_height));
    }
    add_active_hidden_panels(ctx, kinds, active);
}

super::impl_preset!(
    Stacked,
    runtime(kinds, |this| crate::strategy::StrategyKind::ActivePanel {
        variant: crate::strategy::ActivePanelVariant::Stacked,
        bar_height: this.bar_height,
    })
);
