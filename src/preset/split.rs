use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::grow;
use crate::preset::validate_f32_param;

/// Builder for the split preset layout.
pub struct Split {
    first: Arc<str>,
    second: Arc<str>,
    ratio: f32,
    gap: f32,
    is_vertical: bool,
}

impl Split {
    pub(crate) fn new(first: impl Into<Arc<str>>, second: impl Into<Arc<str>>) -> Self {
        Self {
            first: first.into(),
            second: second.into(),
            ratio: 0.5,
            gap: 0.0,
            is_vertical: false,
        }
    }

    /// Set the split ratio.
    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio;
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Use vertical split direction.
    pub fn vertical(mut self) -> Self {
        self.is_vertical = true;
        self
    }

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        validate_f32_param("ratio", self.ratio)?;

        let mut b = LayoutBuilder::new();
        let first = Arc::clone(&self.first);
        let second = Arc::clone(&self.second);
        let ratio = self.ratio;

        let add_children = |ctx: &mut crate::ContainerCtx| {
            ctx.panel_with(first, grow(ratio));
            ctx.panel_with(second, grow(1.0 - ratio));
        };

        match self.is_vertical {
            true => b.col_gap(self.gap, add_children)?,
            false => b.row_gap(self.gap, add_children)?,
        }

        b.build()
    }
}

impl Split {
    /// Consume the builder and produce a [`crate::runtime::LayoutRuntime`].
    pub fn into_runtime(self) -> Result<crate::runtime::LayoutRuntime, PaneError> {
        let direction = match self.is_vertical {
            true => crate::strategy::Direction::Vertical,
            false => crate::strategy::Direction::Horizontal,
        };
        let strategy = crate::strategy::StrategyKind::Sequence {
            direction,
            gap: self.gap,
            ratio: Some(self.ratio),
        };
        let kinds = [Arc::clone(&self.first), Arc::clone(&self.second)];
        crate::runtime::LayoutRuntime::from_strategy(strategy, &kinds)
    }
}

super::impl_preset!(Split);
