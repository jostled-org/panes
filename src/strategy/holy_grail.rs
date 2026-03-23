use std::sync::Arc;

use crate::error::PaneError;

use super::builder::BoundStrategy;
use super::{Direction, SlotDef, StrategyKind};

/// Builder for holy-grail strategy: header, footer, left sidebar, main, right sidebar.
#[derive(Debug, Clone)]
pub struct HolyGrailStrategy {
    gap: f32,
    sidebar_width: f32,
    header_height: f32,
    footer_height: f32,
}

impl HolyGrailStrategy {
    /// Create from gap, sidebar width, header height, and footer height.
    pub(crate) fn new(
        gap: f32,
        sidebar_width: f32,
        header_height: f32,
        footer_height: f32,
    ) -> Self {
        Self {
            gap,
            sidebar_width,
            header_height,
            footer_height,
        }
    }

    crate::macros::builder_setters!(
        /// Set the sidebar width.
        sidebar_width(width: f32);
        /// Set the header height.
        header_height(height: f32);
        /// Set the footer height.
        footer_height(height: f32);
        /// Set the gap between panels.
        gap(gap: f32)
    );

    /// Bind holy-grail panels: header, footer, left sidebar, main, right sidebar.
    pub fn with_panels(
        self,
        header: impl Into<Arc<str>>,
        footer: impl Into<Arc<str>>,
        left: impl Into<Arc<str>>,
        main: impl Into<Arc<str>>,
        right: impl Into<Arc<str>>,
    ) -> Result<BoundStrategy, PaneError> {
        let header: Arc<str> = header.into();
        let footer: Arc<str> = footer.into();
        let left: Arc<str> = left.into();
        let main_kind: Arc<str> = main.into();
        let right: Arc<str> = right.into();

        let slots: Arc<[SlotDef]> = vec![
            SlotDef {
                kind: Arc::clone(&header),
                constraints: crate::panel::fixed(self.header_height),
            },
            SlotDef {
                kind: Arc::clone(&left),
                constraints: crate::panel::fixed(self.sidebar_width),
            },
            SlotDef {
                kind: Arc::clone(&main_kind),
                constraints: crate::panel::grow(1.0),
            },
            SlotDef {
                kind: Arc::clone(&right),
                constraints: crate::panel::fixed(self.sidebar_width),
            },
            SlotDef {
                kind: Arc::clone(&footer),
                constraints: crate::panel::fixed(self.footer_height),
            },
        ]
        .into();
        let kind = StrategyKind::Slotted {
            slots,
            gap: self.gap,
            direction: Direction::Vertical,
        };

        let layout = crate::preset::HolyGrail::new(
            Arc::clone(&header),
            Arc::clone(&footer),
            Arc::clone(&left),
            Arc::clone(&main_kind),
            Arc::clone(&right),
        )
        .header_height(self.header_height)
        .footer_height(self.footer_height)
        .sidebar_width(self.sidebar_width)
        .gap(self.gap)
        .build()?;

        let panels: Box<[Arc<str>]> = Box::from([header, left, main_kind, right, footer]);
        Ok(BoundStrategy::new(kind, panels, Some(layout)))
    }
}
