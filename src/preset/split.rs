use std::sync::Arc;

use crate::builder::{LayoutBuilder, gap};
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::grow;
use crate::preset::validate_f32_param;

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

    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn vertical(mut self) -> Self {
        self.is_vertical = true;
        self
    }

    pub fn build(&self) -> Result<Layout, PaneError> {
        validate_f32_param("ratio", self.ratio)?;

        let mut b = LayoutBuilder::new();
        let first = Arc::clone(&self.first);
        let second = Arc::clone(&self.second);
        let ratio = self.ratio;
        let gap_val = gap(self.gap);

        let add_children = |ctx: &mut crate::ContainerCtx| {
            ctx.panel(first, grow(ratio))?;
            ctx.panel(second, grow(1.0 - ratio))?;
            Ok(())
        };

        match self.is_vertical {
            true => b.col(gap_val, add_children)?,
            false => b.row(gap_val, add_children)?,
        }

        b.build()
    }
}

super::impl_preset!(Split);
