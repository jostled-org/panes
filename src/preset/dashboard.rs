use std::sync::Arc;

use taffy::prelude::TaffyGridLine;

use crate::builder::LayoutBuilder;
use crate::error::{ConstraintError, PaneError, TreeError};
use crate::layout::Layout;
use crate::strategy::{CardSpan, GridColumnMode};

/// Builder for the grid-based dashboard preset layout.
pub struct Dashboard {
    cards: Arc<[(Arc<str>, CardSpan)]>,
    columns: GridColumnMode,
    gap: f32,
}

impl Dashboard {
    pub(crate) fn new(
        cards: impl IntoIterator<Item = (impl Into<Arc<str>>, impl Into<CardSpan>)>,
    ) -> Self {
        Self {
            cards: cards
                .into_iter()
                .map(|(k, span)| (k.into(), span.into()))
                .collect(),
            columns: GridColumnMode::Fixed(4),
            gap: 0.0,
        }
    }

    /// Set a fixed number of columns.
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

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        match self.cards.is_empty() {
            true => return Err(PaneError::InvalidTree(TreeError::DashboardNoCards)),
            false => {}
        }
        validate_dashboard_columns(self.columns)?;

        let mut b = LayoutBuilder::new();
        let grid_style = super::simple_grid_style(self.columns, self.gap);

        b.row(|r| {
            r.taffy_node(grid_style, |grid| add_cards(grid, &self.cards));
        })?;

        b.build()
    }
}

/// Dashboard-specific column validation (uses DashboardNoColumns error).
fn validate_dashboard_columns(columns: GridColumnMode) -> Result<(), PaneError> {
    match columns {
        GridColumnMode::Fixed(0) => Err(PaneError::InvalidTree(TreeError::DashboardNoColumns)),
        GridColumnMode::AutoFill { min_width } | GridColumnMode::AutoFit { min_width }
            if !(min_width > 0.0 && min_width.is_finite()) =>
        {
            Err(PaneError::InvalidTree(TreeError::DashboardMinWidthInvalid))
        }
        _ => Ok(()),
    }
}

fn card_style(span: CardSpan) -> Result<taffy::Style, PaneError> {
    let grid_column = match span {
        CardSpan::FullWidth => taffy::Line {
            start: taffy::GridPlacement::from_line_index(1),
            end: taffy::GridPlacement::from_line_index(-1),
        },
        CardSpan::Columns(n) => {
            let span_u16 = u16::try_from(n)
                .map_err(|_| PaneError::InvalidConstraint(ConstraintError::GridSpanOverflow(n)))?;
            taffy::Line {
                start: taffy::GridPlacement::Auto,
                end: taffy::GridPlacement::Span(span_u16),
            }
        }
    };
    Ok(taffy::Style {
        grid_column,
        ..Default::default()
    })
}

fn add_cards(ctx: &mut crate::ContainerCtx, cards: &[(Arc<str>, CardSpan)]) {
    for (kind, span) in cards {
        match card_style(*span) {
            Ok(style) => {
                ctx.taffy_node(style, |inner| {
                    inner.panel(Arc::clone(kind));
                });
            }
            Err(e) => {
                ctx.set_error(e);
                return;
            }
        }
    }
}

impl Dashboard {
    /// Consume the builder and produce a [`crate::runtime::LayoutRuntime`].
    pub fn into_runtime(self) -> Result<crate::runtime::LayoutRuntime, PaneError> {
        let spans: Arc<[CardSpan]> = self.cards.iter().map(|(_, s)| *s).collect();
        let kinds: Vec<Arc<str>> = self.cards.iter().map(|(k, _)| Arc::clone(k)).collect();
        let strategy = self.columns.to_dashboard_strategy(self.gap, spans);
        crate::runtime::LayoutRuntime::from_strategy(strategy, &kinds)
    }
}

super::impl_preset!(Dashboard);
