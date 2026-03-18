use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::preset::{add_active_hidden_panels, collect_kinds, validate_active, validate_kinds};

/// Builder for the monocle preset layout.
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

    /// Set which panel index is active (visible).
    pub fn active(mut self, index: usize) -> Self {
        self.active = index;
        self
    }

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        validate_kinds(&self.kinds)?;
        validate_active(self.active, self.kinds.len())?;

        let mut b = LayoutBuilder::new();
        let active = self.active;

        b.col(|c| {
            add_active_hidden_panels(c, &self.kinds, active);
        })?;

        b.build()
    }
}

super::impl_preset!(
    Monocle,
    runtime(kinds, |_this| crate::strategy::StrategyKind::ActivePanel {
        variant: crate::strategy::ActivePanelVariant::Monocle,
        bar_height: 0.0,
    })
);
