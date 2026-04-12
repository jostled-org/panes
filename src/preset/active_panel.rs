use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::decoration::DecorationRole;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::fixed;
use crate::preset::{
    add_active_hidden_panels, collect_kinds, validate_active, validate_f32_param, validate_kinds,
};
use crate::strategy::ActivePanelVariant;

/// Shared builder for active-panel preset layouts (tabbed, stacked).
///
/// Both variants share the same field set, validation, and runtime wiring.
/// The layout structure differs only in how decoration panels are arranged:
/// tabbed uses a single row-direction tab bar, stacked uses per-panel title
/// bars.
pub struct ActivePanelPreset {
    kinds: Arc<[Arc<str>]>,
    active: usize,
    bar_height: f32,
    gap: f32,
    variant: ActivePanelVariant,
}

/// Tabbed active-panel preset: tab bar above content panels.
pub type Tabbed = ActivePanelPreset;
/// Stacked active-panel preset: per-panel title bars above content.
pub type Stacked = ActivePanelPreset;

impl ActivePanelPreset {
    pub(crate) fn new_tabbed(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self {
            kinds: collect_kinds(kinds),
            active: 0,
            bar_height: 1.0,
            gap: 0.0,
            variant: ActivePanelVariant::Tabbed,
        }
    }

    pub(crate) fn new_stacked(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self {
            kinds: collect_kinds(kinds),
            active: 0,
            bar_height: 1.0,
            gap: 0.0,
            variant: ActivePanelVariant::Stacked,
        }
    }

    crate::macros::builder_setters!(
        /// Set which panel index is active (visible).
        active(index: usize);
        /// Set the bar height (tab bar or title bar).
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
        let active = self.active;

        match self.variant {
            ActivePanelVariant::Tabbed => {
                let tab_bar_style = tab_bar_fixed_style(self.bar_height);
                b.col_gap(gap_px, |outer| {
                    outer.taffy_node(tab_bar_style, |bar| {
                        add_tab_panels(bar, &self.kinds);
                    });
                    add_active_hidden_panels(outer, &self.kinds, active);
                })?;
            }
            ActivePanelVariant::Stacked => {
                let title_h = self.bar_height;
                b.col_gap(gap_px, |c| {
                    add_stacked_panels(c, &self.kinds, active, title_h);
                })?;
            }
            ActivePanelVariant::Monocle => {
                b.col_gap(gap_px, |c| {
                    add_active_hidden_panels(c, &self.kinds, active);
                })?;
            }
        }

        b.build()
    }
}

/// Row-direction style with fixed height for the tab bar.
fn tab_bar_fixed_style(height: f32) -> taffy::Style {
    taffy::Style {
        flex_direction: taffy::FlexDirection::Row,
        flex_grow: 0.0,
        flex_basis: taffy::Dimension::length(height),
        flex_shrink: 0.0,
        ..Default::default()
    }
}

fn add_tab_panels(ctx: &mut crate::ContainerCtx, kinds: &[Arc<str>]) {
    for kind in kinds {
        ctx.decoration_panel(
            Arc::clone(kind),
            crate::panel::grow(1.0),
            DecorationRole::Tab,
        );
    }
}

fn add_stacked_panels(
    ctx: &mut crate::ContainerCtx,
    kinds: &[Arc<str>],
    active: usize,
    title_height: f32,
) {
    for kind in kinds {
        ctx.decoration_panel(Arc::clone(kind), fixed(title_height), DecorationRole::Title);
    }
    add_active_hidden_panels(ctx, kinds, active);
}

super::impl_preset!(
    ActivePanelPreset,
    runtime(kinds, |this| crate::strategy::StrategyKind::ActivePanel {
        variant: this.variant,
        bar_height: this.bar_height,
    })
);
