use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::grow;
use crate::preset::{validate_f32_param, validate_share_param};

/// Builder for the split preset layout.
pub struct Split {
    first: Arc<str>,
    second: Arc<str>,
    ratio: f32,
    gap: f32,
    is_vertical: bool,
}

impl Split {
    pub(crate) fn new(first: impl Into<Arc<str>>, second: impl Into<Arc<str>>) -> Self {
        Self {
            first: first.into(),
            second: second.into(),
            ratio: 0.5,
            gap: 0.0,
            is_vertical: false,
        }
    }

    crate::macros::builder_setters!(
        /// Set the split ratio.
        ratio(ratio: f32);
        /// Set the gap between panels.
        gap(gap: f32)
    );

    crate::macros::builder_flag_setters!(
        /// Use vertical split direction.
        vertical -> is_vertical = true
    );

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        validate_share_param("ratio", self.ratio)?;
        validate_f32_param("gap", self.gap)?;

        let mut b = LayoutBuilder::new();
        let first = Arc::clone(&self.first);
        let second = Arc::clone(&self.second);
        let ratio = self.ratio;

        let add_children = |ctx: &mut crate::ContainerCtx| {
            ctx.panel_with(first, grow(ratio));
            ctx.panel_with(second, grow(1.0 - ratio));
        };

        match self.is_vertical {
            true => b.col_gap(self.gap, add_children)?,
            false => b.row_gap(self.gap, add_children)?,
        }

        b.build()
    }
}

impl Split {
    /// Consume the builder and produce a [`crate::runtime::LayoutRuntime`].
    pub fn into_runtime(self) -> Result<crate::runtime::LayoutRuntime, PaneError> {
        let mut builder = crate::strategy::builder::Strategy::split()
            .ratio(self.ratio)
            .gap(self.gap);
        match self.is_vertical {
            true => {
                builder = builder.vertical();
            }
            false => {}
        }
        builder.with_panels(self.first, self.second).into_runtime()
    }
}

super::impl_preset!(Split);
