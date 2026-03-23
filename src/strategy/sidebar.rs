use std::sync::Arc;

use super::builder::BoundStrategy;
use super::{Direction, SlotDef, StrategyKind};

/// Builder for sidebar strategy: fixed-width sidebar + grow content.
#[derive(Debug, Clone)]
pub struct SidebarStrategy {
    gap: f32,
    sidebar_width: f32,
}

impl SidebarStrategy {
    /// Create from gap and sidebar width.
    pub(crate) fn new(gap: f32, sidebar_width: f32) -> Self {
        Self { gap, sidebar_width }
    }

    crate::macros::builder_setters!(
        /// Set the sidebar width.
        sidebar_width(width: f32);
        /// Set the gap between panels.
        gap(gap: f32)
    );

    /// Bind sidebar panels: fixed-width sidebar + grow content.
    pub fn with_panels(
        self,
        sidebar: impl Into<Arc<str>>,
        content: impl Into<Arc<str>>,
    ) -> BoundStrategy {
        let sidebar: Arc<str> = sidebar.into();
        let content: Arc<str> = content.into();
        let slots: Arc<[SlotDef]> = vec![
            SlotDef {
                kind: Arc::clone(&sidebar),
                constraints: crate::panel::fixed(self.sidebar_width),
            },
            SlotDef {
                kind: Arc::clone(&content),
                constraints: crate::panel::grow(1.0),
            },
        ]
        .into();
        let kind = StrategyKind::Slotted {
            slots,
            gap: self.gap,
            direction: Direction::Horizontal,
        };
        BoundStrategy::new(kind, Box::from([sidebar, content]), None)
    }
}
