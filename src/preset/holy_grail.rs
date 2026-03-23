use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::fixed;
use crate::preset::{row_style, validate_f32_param};
use crate::strategy::builder::Strategy;

/// Builder for the holy-grail preset layout.
pub struct HolyGrail {
    header: Arc<str>,
    footer: Arc<str>,
    left: Arc<str>,
    main: Arc<str>,
    right: Arc<str>,
    header_height: f32,
    footer_height: f32,
    sidebar_width: f32,
    gap: f32,
}

impl HolyGrail {
    pub(crate) fn new(
        header: impl Into<Arc<str>>,
        footer: impl Into<Arc<str>>,
        left: impl Into<Arc<str>>,
        main: impl Into<Arc<str>>,
        right: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            header: header.into(),
            footer: footer.into(),
            left: left.into(),
            main: main.into(),
            right: right.into(),
            header_height: 1.0,
            footer_height: 1.0,
            sidebar_width: 20.0,
            gap: 0.0,
        }
    }

    crate::macros::builder_setters!(
        /// Set the header height.
        header_height(height: f32);
        /// Set the footer height.
        footer_height(height: f32);
        /// Set the sidebar width.
        sidebar_width(width: f32);
        /// Set the gap between panels.
        gap(gap: f32)
    );

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        validate_f32_param("header_height", self.header_height)?;
        validate_f32_param("footer_height", self.footer_height)?;
        validate_f32_param("sidebar_width", self.sidebar_width)?;
        validate_f32_param("gap", self.gap)?;

        let mut b = LayoutBuilder::new();
        let gap_px = self.gap;
        let header = Arc::clone(&self.header);
        let footer = Arc::clone(&self.footer);
        let left = Arc::clone(&self.left);
        let main_kind = Arc::clone(&self.main);
        let right = Arc::clone(&self.right);
        let sw = self.sidebar_width;
        let hh = self.header_height;
        let fh = self.footer_height;

        b.col_gap(gap_px, |outer| {
            outer.panel_with(header, fixed(hh));
            outer.taffy_node(row_style(1.0, gap_px), |mid| {
                build_middle(mid, left, main_kind, right, sw);
            });
            outer.panel_with(footer, fixed(fh));
        })?;

        b.build()
    }
}

fn build_middle(
    ctx: &mut crate::ContainerCtx,
    left: Arc<str>,
    main_kind: Arc<str>,
    right: Arc<str>,
    sidebar_width: f32,
) {
    ctx.panel_with(left, fixed(sidebar_width));
    ctx.panel(main_kind);
    ctx.panel_with(right, fixed(sidebar_width));
}

impl HolyGrail {
    /// Consume the builder and produce a [`crate::runtime::LayoutRuntime`].
    pub fn into_runtime(self) -> Result<crate::runtime::LayoutRuntime, PaneError> {
        Strategy::holy_grail()
            .header_height(self.header_height)
            .footer_height(self.footer_height)
            .sidebar_width(self.sidebar_width)
            .gap(self.gap)
            .with_panels(self.header, self.footer, self.left, self.main, self.right)?
            .into_runtime()
    }
}

super::impl_preset!(HolyGrail);
