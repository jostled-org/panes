use std::sync::Arc;

use super::GridColumnMode;
use super::builder::{BoundStrategy, Strategy};
use super::dashboard::DashboardStrategy;

/// Builder for columns strategy. Deprecated — use [`DashboardStrategy`] instead.
#[deprecated(since = "0.12.0", note = "use Strategy::dashboard() instead")]
#[derive(Debug, Clone)]
pub struct ColumnsStrategy {
    columns: GridColumnMode,
    gap: f32,
}

#[allow(deprecated)]
impl ColumnsStrategy {
    /// Create a new columns strategy with defaults.
    pub(crate) fn new() -> Self {
        Self {
            columns: GridColumnMode::Fixed(0),
            gap: 0.0,
        }
    }

    /// Set a fixed number of columns. When 0 (default), uses panel count.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = GridColumnMode::Fixed(columns);
        self
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

    fn into_dashboard(self) -> DashboardStrategy {
        DashboardStrategy {
            columns: self.columns,
            gap: self.gap,
        }
    }

    /// Bind panels directly.
    pub fn with_panels(
        self,
        panels: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> BoundStrategy {
        let panels: Vec<Arc<str>> = panels.into_iter().map(Into::into).collect();
        let resolved = match self.columns {
            GridColumnMode::Fixed(0) => GridColumnMode::Fixed(panels.len()),
            other => other,
        };
        let d = DashboardStrategy {
            columns: resolved,
            gap: self.gap,
        };
        d.with_panels(panels)
    }
}

#[allow(deprecated)]
impl From<ColumnsStrategy> for Strategy {
    fn from(builder: ColumnsStrategy) -> Self {
        builder.build()
    }
}
