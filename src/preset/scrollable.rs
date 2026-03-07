use std::sync::Arc;

use crate::builder::{LayoutBuilder, gap};
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::fixed;
use crate::preset::{collect_kinds, validate_f32_param, validate_kinds};

/// Builder for the scrollable preset layout.
pub struct Scrollable {
    kinds: Arc<[Arc<str>]>,
    col_width: f32,
    gap: f32,
}

impl Scrollable {
    pub(crate) fn new(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self {
            kinds: collect_kinds(kinds),
            col_width: 80.0,
            gap: 0.0,
        }
    }

    /// Set the column width.
    pub fn col_width(mut self, width: f32) -> Self {
        self.col_width = width;
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
        validate_f32_param("col_width", self.col_width)?;

        let mut b = LayoutBuilder::new();
        let col_width = self.col_width;
        let gap_px = self.gap;

        // Root is a TaffyPassthrough row with flex_shrink: 0 so children don't shrink
        let root_style = scrollable_root_style(gap_px);

        b.row(gap(0.0), |r| {
            r.taffy_node(root_style, |inner| {
                add_fixed_panels(inner, &self.kinds, col_width)
            })
        })?;

        b.build()
    }
}

fn scrollable_root_style(gap_px: f32) -> taffy::Style {
    taffy::Style {
        flex_direction: taffy::FlexDirection::Row,
        flex_grow: 1.0,
        flex_basis: taffy::Dimension::length(0.0),
        flex_shrink: 0.0,
        gap: taffy::Size {
            width: taffy::LengthPercentage::length(gap_px),
            height: taffy::LengthPercentage::length(0.0),
        },
        ..Default::default()
    }
}

fn add_fixed_panels(
    ctx: &mut crate::ContainerCtx,
    kinds: &[Arc<str>],
    col_width: f32,
) -> Result<(), PaneError> {
    for kind in kinds {
        ctx.panel(Arc::clone(kind), fixed(col_width))?;
    }
    Ok(())
}

super::impl_preset!(Scrollable);
