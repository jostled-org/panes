use std::sync::Arc;

use super::GridColumnMode;
use super::builder::{BoundStrategy, Strategy};
use super::dashboard::DashboardStrategy;

/// Builder for grid strategies. Deprecated — use [`DashboardStrategy`] instead.
#[deprecated(since = "0.12.0", note = "use Strategy::dashboard() instead")]
#[derive(Debug, Clone)]
pub struct ColumnGridStrategy {
    columns: GridColumnMode,
    gap: f32,
}

#[allow(deprecated)]
impl ColumnGridStrategy {
    /// Create a new grid strategy with the given column count.
    pub(crate) fn new(columns: usize) -> Self {
        Self {
            columns: GridColumnMode::Fixed(columns),
            gap: 0.0,
        }
    }

    /// Use responsive `repeat(auto-fill, minmax(min_width, 1fr))` columns.
    pub fn auto_fill(mut self, min_width: f32) -> Self {
        self.columns = GridColumnMode::AutoFill { min_width };
        self
    }

    /// Use responsive `repeat(auto-fit, minmax(min_width, 1fr))` columns.
    pub fn auto_fit(mut self, min_width: f32) -> Self {
        self.columns = GridColumnMode::AutoFit { min_width };
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Convert to a generic [`Strategy`].
    pub fn build(self) -> Strategy {
        self.into_dashboard().build()
    }

    /// Bind panels directly.
    pub fn with_panels(
        self,
        panels: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> BoundStrategy {
        self.into_dashboard().with_panels(panels)
    }

    fn into_dashboard(self) -> DashboardStrategy {
        DashboardStrategy {
            columns: self.columns,
            gap: self.gap,
        }
    }
}

#[allow(deprecated)]
impl From<ColumnGridStrategy> for Strategy {
    fn from(builder: ColumnGridStrategy) -> Self {
        builder.build()
    }
}
