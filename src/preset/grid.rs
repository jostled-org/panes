use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::preset::{collect_kinds, validate_grid_columns, validate_kinds};
use crate::strategy::GridColumnMode;

/// Builder for the grid preset layout.
pub struct Grid {
    cols: GridColumnMode,
    kinds: Arc<[Arc<str>]>,
    gap: f32,
}

impl Grid {
    pub(crate) fn new(cols: usize, kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self {
            cols: GridColumnMode::Fixed(cols),
            kinds: collect_kinds(kinds),
            gap: 0.0,
        }
    }

    /// Use responsive `repeat(auto-fill, minmax(min_width, 1fr))` columns.
    pub fn auto_fill(mut self, min_width: f32) -> Self {
        self.cols = GridColumnMode::AutoFill { min_width };
        self
    }

    /// Use responsive `repeat(auto-fit, minmax(min_width, 1fr))` columns.
    pub fn auto_fit(mut self, min_width: f32) -> Self {
        self.cols = GridColumnMode::AutoFit { min_width };
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        validate_grid_columns(self.cols)?;
        validate_kinds(&self.kinds)?;

        let mut b = LayoutBuilder::new();
        let style = super::simple_grid_style(self.cols, self.gap);

        b.row(|r| {
            r.taffy_node(style, |grid| {
                super::add_grow_panels(grid, &self.kinds);
            });
        })?;

        b.build()
    }
}

impl Grid {
    /// Consume the builder and produce a [`crate::runtime::LayoutRuntime`].
    pub fn into_runtime(self) -> Result<crate::runtime::LayoutRuntime, PaneError> {
        let strategy = match self.cols {
            GridColumnMode::Fixed(columns) => crate::strategy::StrategyKind::ColumnGrid {
                columns,
                gap: self.gap,
            },
            GridColumnMode::AutoFill { min_width } => {
                crate::strategy::StrategyKind::ColumnGridAutoFill {
                    min_width,
                    gap: self.gap,
                }
            }
            GridColumnMode::AutoFit { min_width } => {
                crate::strategy::StrategyKind::ColumnGridAutoFit {
                    min_width,
                    gap: self.gap,
                }
            }
        };
        crate::runtime::LayoutRuntime::from_strategy(strategy, &self.kinds)
    }
}

super::impl_preset!(Grid);
