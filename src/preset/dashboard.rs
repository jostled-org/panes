use std::sync::Arc;

use taffy::prelude::fr;

use crate::builder::LayoutBuilder;
use crate::error::{ConstraintError, PaneError, TreeError};
use crate::layout::Layout;

/// Builder for the grid-based dashboard preset layout.
pub struct Dashboard {
    cards: Arc<[(Arc<str>, usize)]>,
    columns: usize,
    gap: f32,
}

impl Dashboard {
    pub(crate) fn new(cards: impl IntoIterator<Item = (impl Into<Arc<str>>, usize)>) -> Self {
        Self {
            cards: cards
                .into_iter()
                .map(|(k, span)| (k.into(), span))
                .collect(),
            columns: 4,
            gap: 0.0,
        }
    }

    /// Set the number of columns.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns;
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
            true => {
                return Err(PaneError::InvalidTree(TreeError::DashboardNoCards));
            }
            _ => {}
        }
        match self.columns {
            0 => {
                return Err(PaneError::InvalidTree(TreeError::DashboardNoColumns));
            }
            _ => {}
        }

        let mut b = LayoutBuilder::new();
        let grid_style = self.grid_root_style();

        b.row(|r| {
            r.taffy_node(grid_style, |grid| add_cards(grid, &self.cards));
        })?;

        b.build()
    }

    fn grid_root_style(&self) -> taffy::Style {
        let gap_len = taffy::LengthPercentage::length(self.gap);
        taffy::Style {
            display: taffy::Display::Grid,
            size: taffy::Size {
                width: taffy::Dimension::percent(1.0),
                height: taffy::Dimension::percent(1.0),
            },
            grid_template_columns: vec![fr(1.0); self.columns],
            grid_auto_rows: vec![fr(1.0)],
            gap: taffy::Size {
                width: gap_len,
                height: gap_len,
            },
            ..Default::default()
        }
    }
}

fn card_style(span: usize) -> Result<taffy::Style, PaneError> {
    let span_u16 = u16::try_from(span)
        .map_err(|_| PaneError::InvalidConstraint(ConstraintError::GridSpanOverflow(span)))?;
    Ok(taffy::Style {
        grid_column: taffy::Line {
            start: taffy::GridPlacement::Auto,
            end: taffy::GridPlacement::Span(span_u16),
        },
        ..Default::default()
    })
}

fn add_cards(ctx: &mut crate::ContainerCtx, cards: &[(Arc<str>, usize)]) {
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
        let spans: Arc<[usize]> = self.cards.iter().map(|(_, s)| *s).collect();
        let kinds: Vec<Arc<str>> = self.cards.iter().map(|(k, _)| Arc::clone(k)).collect();
        let strategy = crate::strategy::StrategyKind::Dashboard {
            columns: self.columns,
            gap: self.gap,
            spans,
        };
        crate::runtime::LayoutRuntime::from_strategy(strategy, &kinds)
    }
}

super::impl_preset!(Dashboard);
