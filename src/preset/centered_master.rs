use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::grow;
use crate::preset::{add_panels, col_style, collect_kinds, validate_f32_param, validate_kinds};

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

    crate::macros::builder_setters!(
        /// Set the master panel's share of the viewport.
        master_ratio(ratio: f32);
        /// Set the gap between panels.
        gap(gap: f32)
    );

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        validate_kinds(&self.kinds)?;
        validate_f32_param("master_ratio", self.master_ratio)?;
        validate_f32_param("gap", self.gap)?;
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

        b.row_gap(gap_px, |r| {
            r.taffy_node(col_style(side_ratio, gap_px), |c| {
                add_panels(c, &left_kinds, grow(1.0));
            });
            r.panel_with(master_kind, grow(ratio));
            r.taffy_node(col_style(side_ratio, gap_px), |c| {
                add_panels(c, &right_kinds, grow(1.0));
            });
        })?;

        b.build()
    }
}

type KindSlice = Box<[Arc<str>]>;

/// Split items into left and right lists by alternation.
/// Even indices (0, 2, 4...) go left, odd indices (1, 3, 5...) go right.
fn split_alternating(items: &[Arc<str>]) -> (KindSlice, KindSlice) {
    let cap = items.len().div_ceil(2);
    let mut left = Vec::with_capacity(cap);
    let mut right = Vec::with_capacity(cap);
    for (i, item) in items.iter().enumerate() {
        match i % 2 {
            0 => left.push(Arc::clone(item)),
            _ => right.push(Arc::clone(item)),
        }
    }
    (left.into_boxed_slice(), right.into_boxed_slice())
}

super::impl_preset!(
    CenteredMaster,
    runtime(kinds, |this| {
        crate::strategy::StrategyKind::CenteredMaster {
            master_ratio: this.master_ratio,
            gap: this.gap,
        }
    })
);
