use std::sync::Arc;

use crate::builder::{LayoutBuilder, gap};
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::grow;
use crate::preset::{collect_kinds, validate_f32_param, validate_kinds};

/// Builder for the master-stack preset layout.
pub struct MasterStack {
    kinds: Arc<[Arc<str>]>,
    master_ratio: f32,
    gap: f32,
}

impl MasterStack {
    pub(crate) fn new(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self {
            kinds: collect_kinds(kinds),
            master_ratio: 0.5,
            gap: 0.0,
        }
    }

    /// Set the master panel's share of the viewport.
    pub fn master_ratio(mut self, ratio: f32) -> Self {
        self.master_ratio = ratio;
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
        validate_f32_param("master_ratio", self.master_ratio)?;
        match self.kinds.len() {
            1 => super::build_single(Arc::clone(&self.kinds[0])),
            _ => self.build_master_stack(),
        }
    }

    fn build_master_stack(&self) -> Result<Layout, PaneError> {
        let mut b = LayoutBuilder::new();
        let gap_val = gap(self.gap);
        let ratio = self.master_ratio;
        let master_kind = Arc::clone(&self.kinds[0]);
        let stack_style = col_style(1.0 - ratio, self.gap);

        b.row(gap_val, |r| {
            r.panel(master_kind, grow(ratio))?;
            r.taffy_node(stack_style, |c| add_panels(c, &self.kinds[1..], grow(1.0)))
        })?;

        b.build()
    }
}

/// A column-direction taffy style with a specific grow factor and gap.
pub(crate) fn col_style(flex_grow: f32, gap_px: f32) -> taffy::Style {
    taffy::Style {
        flex_direction: taffy::FlexDirection::Column,
        flex_grow,
        flex_basis: taffy::Dimension::length(0.0),
        flex_shrink: 1.0,
        gap: taffy::Size {
            width: taffy::LengthPercentage::length(0.0),
            height: taffy::LengthPercentage::length(gap_px),
        },
        ..Default::default()
    }
}

/// A row-direction taffy style with a specific grow factor and gap.
pub(crate) fn row_style(flex_grow: f32, gap_px: f32) -> taffy::Style {
    taffy::Style {
        flex_direction: taffy::FlexDirection::Row,
        flex_grow,
        flex_basis: taffy::Dimension::length(0.0),
        flex_shrink: 1.0,
        gap: taffy::Size {
            width: taffy::LengthPercentage::length(gap_px),
            height: taffy::LengthPercentage::length(0.0),
        },
        ..Default::default()
    }
}

pub(crate) fn add_panels(
    ctx: &mut crate::ContainerCtx,
    kinds: &[Arc<str>],
    constraints: crate::Constraints,
) -> Result<(), PaneError> {
    for kind in kinds {
        ctx.panel(Arc::clone(kind), constraints)?;
    }
    Ok(())
}

super::impl_preset!(MasterStack);
