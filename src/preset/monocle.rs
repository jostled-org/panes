use std::sync::Arc;

use crate::builder::{LayoutBuilder, gap};
use crate::error::PaneError;
use crate::layout::Layout;
use crate::preset::{add_active_hidden_panels, collect_kinds, validate_active, validate_kinds};

pub struct Monocle {
    kinds: Arc<[Arc<str>]>,
    active: usize,
}

impl Monocle {
    pub(crate) fn new(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self {
            kinds: collect_kinds(kinds),
            active: 0,
        }
    }

    pub fn active(mut self, index: usize) -> Self {
        self.active = index;
        self
    }

    pub fn build(&self) -> Result<Layout, PaneError> {
        validate_kinds(&self.kinds)?;
        validate_active(self.active, self.kinds.len())?;

        let mut b = LayoutBuilder::new();
        let active = self.active;

        b.col(gap(0.0), |c| {
            add_active_hidden_panels(c, &self.kinds, active)
        })?;

        b.build()
    }
}

super::impl_preset!(Monocle);
