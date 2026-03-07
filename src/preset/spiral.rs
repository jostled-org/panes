use std::sync::Arc;

use crate::builder::{LayoutBuilder, gap};
use crate::error::PaneError;
use crate::layout::Layout;
use crate::preset::dwindle::build_recursive;
use crate::preset::{collect_kinds, validate_f32_param, validate_kinds};

/// Builder for the spiral preset layout.
pub struct Spiral {
    kinds: Arc<[Arc<str>]>,
    ratio: f32,
    gap: f32,
}

impl Spiral {
    pub(crate) fn new(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self {
            kinds: collect_kinds(kinds),
            ratio: 0.5,
            gap: 0.0,
        }
    }

    /// Set the split ratio.
    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio;
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
        validate_f32_param("ratio", self.ratio)?;

        let mut b = LayoutBuilder::new();
        let kinds = &self.kinds;
        let ratio = self.ratio;
        let gap_px = self.gap;

        b.row(gap(gap_px), |r| {
            build_recursive(r, kinds, 0, ratio, gap_px, true)
        })?;

        b.build()
    }
}

super::impl_preset!(Spiral);
