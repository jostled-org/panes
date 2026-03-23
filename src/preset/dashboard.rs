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
    auto_rows: bool,
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
            auto_rows: false,
        }
    }

    crate::macros::builder_mapped_setters!(
        /// Set a fixed number of columns.
        columns(columns: usize) -> columns = GridColumnMode::Fixed(columns);
        /// Use responsive `repeat(auto-fill, minmax(min_width, 1fr))` columns.
        auto_fill(min_width: f32) -> columns = GridColumnMode::AutoFill { min_width };
        /// Use responsive `repeat(auto-fit, minmax(min_width, 1fr))` columns.
        auto_fit(min_width: f32) -> columns = GridColumnMode::AutoFit { min_width }
    );

    crate::macros::builder_setters!(
        /// Set the gap between panels.
        gap(gap: f32)
    );

    crate::macros::builder_flag_setters!(
        /// Use `grid-auto-rows: auto` so rows size to their tallest card.
        auto_rows -> auto_rows = true
    );

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        match self.cards.is_empty() {
            true => return Err(PaneError::InvalidTree(TreeError::DashboardNoCards)),
            false => {}
        }
        validate_dashboard_columns(self.columns)?;
        crate::preset::validate_f32_param("gap", self.gap)?;

        let mut b = LayoutBuilder::new();
        let grid_style = super::simple_grid_style(self.columns, self.gap, self.auto_rows);

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
        let strategy = self
            .columns
            .to_dashboard_strategy(self.gap, spans, self.auto_rows);
        crate::runtime::LayoutRuntime::from_strategy(strategy, &kinds)
    }
}

super::impl_preset!(Dashboard);
