use std::sync::Arc;

use crate::builder::{LayoutBuilder, gap};
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::{fixed, grow};
use crate::preset::validate_f32_param;

pub struct Sidebar {
    sidebar_kind: Arc<str>,
    content_kind: Arc<str>,
    sidebar_width: f32,
    gap: f32,
}

impl Sidebar {
    pub(crate) fn new(
        sidebar_kind: impl Into<Arc<str>>,
        content_kind: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            sidebar_kind: sidebar_kind.into(),
            content_kind: content_kind.into(),
            sidebar_width: 20.0,
            gap: 0.0,
        }
    }

    pub fn sidebar_width(mut self, width: f32) -> Self {
        self.sidebar_width = width;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn build(&self) -> Result<Layout, PaneError> {
        validate_f32_param("sidebar_width", self.sidebar_width)?;

        let mut b = LayoutBuilder::new();
        let sidebar_kind = Arc::clone(&self.sidebar_kind);
        let content_kind = Arc::clone(&self.content_kind);
        let width = self.sidebar_width;

        b.row(gap(self.gap), |r| {
            r.panel(sidebar_kind, fixed(width))?;
            r.panel(content_kind, grow(1.0))?;
            Ok(())
        })?;

        b.build()
    }
}

super::impl_preset!(Sidebar);
