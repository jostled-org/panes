use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::preset::collect_kinds;
use crate::preset::master_stack::col_style;
use crate::preset::validate_kinds;

/// Builder for the equal-columns preset layout.
pub struct Columns {
    count: usize,
    kinds: Arc<[Arc<str>]>,
    gap: f32,
}

impl Columns {
    pub(crate) fn new(count: usize, kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self {
            count,
            kinds: collect_kinds(kinds),
            gap: 0.0,
        }
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        match self.count {
            0 => {
                return Err(PaneError::InvalidTree(
                    "columns count must be at least 1".into(),
                ));
            }
            _ => {}
        }
        validate_kinds(&self.kinds)?;

        let buckets = distribute_round_robin(&self.kinds, self.count);
        let mut b = LayoutBuilder::new();
        let gap_px = self.gap;

        b.row_gap(gap_px, |outer| {
            for bucket in &buckets {
                outer.taffy_node(col_style(1.0, gap_px), |c| {
                    super::add_grow_panels(c, bucket);
                });
            }
        })?;

        b.build()
    }
}

/// Distribute items round-robin into `n` buckets.
fn distribute_round_robin(items: &[Arc<str>], n: usize) -> Vec<Vec<Arc<str>>> {
    let mut buckets: Vec<Vec<Arc<str>>> = (0..n).map(|_| Vec::new()).collect();
    for (i, kind) in items.iter().enumerate() {
        buckets[i % n].push(Arc::clone(kind));
    }
    buckets
}

impl Columns {
    /// Consume the builder and produce a [`crate::runtime::LayoutRuntime`].
    pub fn into_runtime(self) -> Result<crate::runtime::LayoutRuntime, PaneError> {
        let strategy = crate::strategy::StrategyKind::Sequence {
            direction: crate::strategy::Direction::Horizontal,
            gap: self.gap,
        };
        let kinds: Vec<Arc<str>> = self.kinds.to_vec();
        crate::runtime::LayoutRuntime::from_strategy(strategy, &kinds)
    }
}

super::impl_preset!(Columns);
