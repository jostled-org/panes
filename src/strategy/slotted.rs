use std::sync::Arc;

use crate::error::PaneError;

use super::builder::BoundStrategy;
use super::holy_grail::HolyGrailStrategy;
use super::sidebar::SidebarStrategy;

/// Builder for slotted strategies (sidebar, holy-grail).
#[derive(Debug, Clone)]
pub struct SlottedStrategy {
    gap: f32,
    sidebar_width: f32,
    header_height: f32,
    footer_height: f32,
}

impl Default for SlottedStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl SlottedStrategy {
    /// Create a new slotted strategy builder with default values.
    pub fn new() -> Self {
        Self {
            gap: 0.0,
            sidebar_width: 20.0,
            header_height: 1.0,
            footer_height: 1.0,
        }
    }

    /// Set the sidebar width.
    pub fn sidebar_width(mut self, width: f32) -> Self {
        self.sidebar_width = width;
        self
    }

    /// Set the header height (holy-grail only).
    pub fn header_height(mut self, height: f32) -> Self {
        self.header_height = height;
        self
    }

    /// Set the footer height (holy-grail only).
    pub fn footer_height(mut self, height: f32) -> Self {
        self.footer_height = height;
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Bind sidebar panels: fixed-width sidebar + grow content.
    pub fn with_sidebar_panels(
        self,
        sidebar: impl Into<Arc<str>>,
        content: impl Into<Arc<str>>,
    ) -> BoundStrategy {
        SidebarStrategy::new(self.gap, self.sidebar_width).with_panels(sidebar, content)
    }

    /// Bind holy-grail panels: header, footer, left sidebar, main, right sidebar.
    pub fn with_holy_grail_panels(
        self,
        header: impl Into<Arc<str>>,
        footer: impl Into<Arc<str>>,
        left: impl Into<Arc<str>>,
        main: impl Into<Arc<str>>,
        right: impl Into<Arc<str>>,
    ) -> Result<BoundStrategy, PaneError> {
        HolyGrailStrategy::new(
            self.gap,
            self.sidebar_width,
            self.header_height,
            self.footer_height,
        )
        .with_panels(header, footer, left, main, right)
    }
}
