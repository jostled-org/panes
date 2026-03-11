use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::fixed;
use crate::preset::{row_style, validate_f32_param};

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

    /// Set the header height.
    pub fn header_height(mut self, height: f32) -> Self {
        self.header_height = height;
        self
    }

    /// Set the footer height.
    pub fn footer_height(mut self, height: f32) -> Self {
        self.footer_height = height;
        self
    }

    /// Set the sidebar width.
    pub fn sidebar_width(mut self, width: f32) -> Self {
        self.sidebar_width = width;
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        validate_f32_param("header_height", self.header_height)?;
        validate_f32_param("footer_height", self.footer_height)?;
        validate_f32_param("sidebar_width", self.sidebar_width)?;

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
        let slots: Arc<[crate::strategy::SlotDef]> = vec![
            crate::strategy::SlotDef {
                kind: Arc::clone(&self.header),
                constraints: crate::panel::fixed(self.header_height),
            },
            crate::strategy::SlotDef {
                kind: Arc::clone(&self.left),
                constraints: crate::panel::fixed(self.sidebar_width),
            },
            crate::strategy::SlotDef {
                kind: Arc::clone(&self.main),
                constraints: crate::panel::grow(1.0),
            },
            crate::strategy::SlotDef {
                kind: Arc::clone(&self.right),
                constraints: crate::panel::fixed(self.sidebar_width),
            },
            crate::strategy::SlotDef {
                kind: Arc::clone(&self.footer),
                constraints: crate::panel::fixed(self.footer_height),
            },
        ]
        .into();
        let strategy = crate::strategy::StrategyKind::Slotted {
            slots,
            gap: self.gap,
            direction: crate::strategy::Direction::Vertical,
        };
        let kinds = [
            Arc::clone(&self.header),
            Arc::clone(&self.left),
            Arc::clone(&self.main),
            Arc::clone(&self.right),
            Arc::clone(&self.footer),
        ];
        // Build the real nested tree (header, [left|main|right] row, footer)
        // instead of the flat slotted builder which can't represent nesting.
        let tree = crate::tree::LayoutTree::from(self.build()?);
        Ok(crate::runtime::LayoutRuntime::from_tree_and_strategy(
            tree, strategy, &kinds,
        ))
    }
}

super::impl_preset!(HolyGrail);
