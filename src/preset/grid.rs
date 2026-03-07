use std::sync::Arc;

use crate::builder::{LayoutBuilder, gap};
use crate::error::PaneError;
use crate::layout::Layout;
use crate::preset::master_stack::row_style;
use crate::preset::{collect_kinds, validate_kinds};

pub struct Grid {
    cols: usize,
    kinds: Arc<[Arc<str>]>,
    gap: f32,
}

impl Grid {
    pub(crate) fn new(cols: usize, kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self {
            cols,
            kinds: collect_kinds(kinds),
            gap: 0.0,
        }
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn build(&self) -> Result<Layout, PaneError> {
        match self.cols {
            0 => {
                return Err(PaneError::InvalidTree(
                    "grid columns must be at least 1".into(),
                ));
            }
            _ => {}
        }
        validate_kinds(&self.kinds)?;

        let mut b = LayoutBuilder::new();
        let gap_px = self.gap;

        b.col(gap(gap_px), |outer| {
            for chunk in self.kinds.chunks(self.cols) {
                outer.taffy_node(row_style(1.0, gap_px), |r| super::add_grow_panels(r, chunk))?;
            }
            Ok(())
        })?;

        b.build()
    }
}

super::impl_preset!(Grid);
