use std::sync::Arc;

use crate::builder::{LayoutBuilder, gap};
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::grow;
use crate::preset::{
    add_active_hidden_panels, collect_kinds, validate_active, validate_f32_param, validate_kinds,
};

/// Builder for the tabbed preset layout.
pub struct Tabbed {
    kinds: Arc<[Arc<str>]>,
    active: usize,
    tab_height: f32,
    gap: f32,
}

impl Tabbed {
    pub(crate) fn new(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self {
            kinds: collect_kinds(kinds),
            active: 0,
            tab_height: 1.0,
            gap: 0.0,
        }
    }

    /// Set which panel index is active (visible).
    pub fn active(mut self, index: usize) -> Self {
        self.active = index;
        self
    }

    /// Set the tab bar height.
    pub fn tab_height(mut self, height: f32) -> Self {
        self.tab_height = height;
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
        validate_active(self.active, self.kinds.len())?;
        validate_f32_param("tab_height", self.tab_height)?;

        let mut b = LayoutBuilder::new();
        let gap_px = self.gap;
        let tab_height = self.tab_height;
        let active = self.active;
        let tab_bar_style = tab_bar_fixed_style(tab_height);

        b.col(gap(gap_px), |outer| {
            outer.taffy_node(tab_bar_style, |bar| add_tab_panels(bar, &self.kinds))?;
            add_active_hidden_panels(outer, &self.kinds, active)
        })?;

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

fn add_tab_panels(ctx: &mut crate::ContainerCtx, kinds: &[Arc<str>]) -> Result<(), PaneError> {
    for kind in kinds {
        let tab_kind: Arc<str> = format!("{kind}_tab").into();
        ctx.panel(tab_kind, grow(1.0))?;
    }
    Ok(())
}

super::impl_preset!(Tabbed);
