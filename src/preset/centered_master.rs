use std::sync::Arc;

use crate::builder::{LayoutBuilder, gap};
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::grow;
use crate::preset::master_stack::{add_panels, col_style};
use crate::preset::{collect_kinds, validate_f32_param, validate_kinds};

/// Builder for the centered-master preset layout.
pub struct CenteredMaster {
    kinds: Arc<[Arc<str>]>,
    master_ratio: f32,
    gap: f32,
}

impl CenteredMaster {
    pub(crate) fn new(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self {
            kinds: collect_kinds(kinds),
            master_ratio: 0.5,
            gap: 0.0,
        }
    }

    /// Set the master panel's share of the viewport.
    pub fn master_ratio(mut self, ratio: f32) -> Self {
        self.master_ratio = ratio;
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        validate_kinds(&self.kinds)?;
        validate_f32_param("master_ratio", self.master_ratio)?;
        match self.kinds.len() {
            1 => super::build_single(Arc::clone(&self.kinds[0])),
            _ => self.build_centered(),
        }
    }

    fn build_centered(&self) -> Result<Layout, PaneError> {
        let (left_kinds, right_kinds) = split_alternating(&self.kinds[1..]);
        let mut b = LayoutBuilder::new();
        let ratio = self.master_ratio;
        let side_ratio = (1.0 - ratio) / 2.0;
        let gap_px = self.gap;
        let master_kind = Arc::clone(&self.kinds[0]);

        b.row(gap(gap_px), |r| {
            r.taffy_node(col_style(side_ratio, gap_px), |c| {
                add_panels(c, &left_kinds, grow(1.0))
            })?;
            r.panel(master_kind, grow(ratio))?;
            r.taffy_node(col_style(side_ratio, gap_px), |c| {
                add_panels(c, &right_kinds, grow(1.0))
            })
        })?;

        b.build()
    }
}

/// Split items into left and right lists by alternation.
/// Even indices (0, 2, 4...) go left, odd indices (1, 3, 5...) go right.
fn split_alternating(items: &[Arc<str>]) -> (Vec<Arc<str>>, Vec<Arc<str>>) {
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (i, item) in items.iter().enumerate() {
        match i % 2 {
            0 => left.push(Arc::clone(item)),
            _ => right.push(Arc::clone(item)),
        }
    }
    (left, right)
}

impl CenteredMaster {
    /// Consume the builder and produce a [`crate::runtime::LayoutRuntime`].
    pub fn into_runtime(self) -> Result<crate::runtime::LayoutRuntime, PaneError> {
        let strategy = crate::strategy::StrategyKind::CenteredMaster {
            master_ratio: self.master_ratio,
            gap: self.gap,
        };
        let kinds: Vec<Arc<str>> = self.kinds.to_vec();
        crate::runtime::LayoutRuntime::from_strategy(strategy, &kinds)
    }
}

super::impl_preset!(CenteredMaster);
