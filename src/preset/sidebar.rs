use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::fixed;
use crate::preset::validate_f32_param;
use crate::strategy::builder::Strategy;

/// Builder for the sidebar preset layout.
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

    crate::macros::builder_setters!(
        /// Set the sidebar width.
        sidebar_width(width: f32);
        /// Set the gap between panels.
        gap(gap: f32)
    );

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        validate_f32_param("sidebar_width", self.sidebar_width)?;
        validate_f32_param("gap", self.gap)?;

        let mut b = LayoutBuilder::new();
        let sidebar_kind = Arc::clone(&self.sidebar_kind);
        let content_kind = Arc::clone(&self.content_kind);
        let width = self.sidebar_width;

        b.row_gap(self.gap, |r| {
            r.panel_with(sidebar_kind, fixed(width));
            r.panel(content_kind);
        })?;

        b.build()
    }
}

impl Sidebar {
    /// Consume the builder and produce a [`crate::runtime::LayoutRuntime`].
    pub fn into_runtime(self) -> Result<crate::runtime::LayoutRuntime, PaneError> {
        Strategy::sidebar()
            .sidebar_width(self.sidebar_width)
            .gap(self.gap)
            .with_panels(self.sidebar_kind, self.content_kind)
            .into_runtime()
    }
}

super::impl_preset!(Sidebar);
