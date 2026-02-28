use std::sync::Arc;

use taffy::prelude::fr;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::grow;

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

    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn build(&self) -> Result<Layout, PaneError> {
        match self.cards.is_empty() {
            true => {
                return Err(PaneError::InvalidTree(
                    "dashboard requires at least one card".into(),
                ));
            }
            _ => {}
        }
        match self.columns {
            0 => {
                return Err(PaneError::InvalidTree(
                    "dashboard columns must be at least 1".into(),
                ));
            }
            _ => {}
        }

        let mut b = LayoutBuilder::new();
        let grid_style = self.grid_root_style();
        let cards = self.cards.to_vec();

        b.row(crate::builder::gap(0.0), |r| {
            r.taffy_node(grid_style, |grid| add_cards(grid, &cards))
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
    let span_u16 = u16::try_from(span).map_err(|_| {
        PaneError::InvalidConstraint(format!("grid span {span} exceeds u16 max").into())
    })?;
    Ok(taffy::Style {
        grid_column: taffy::Line {
            start: taffy::GridPlacement::Auto,
            end: taffy::GridPlacement::Span(span_u16),
        },
        ..Default::default()
    })
}

fn add_cards(ctx: &mut crate::ContainerCtx, cards: &[(Arc<str>, usize)]) -> Result<(), PaneError> {
    for (kind, span) in cards {
        let style = card_style(*span)?;
        ctx.taffy_node(style, |inner| {
            inner.panel(Arc::clone(kind), grow(1.0))?;
            Ok(())
        })?;
    }
    Ok(())
}

super::impl_preset!(Dashboard);
