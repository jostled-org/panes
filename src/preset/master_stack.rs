use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::grow;
use crate::preset::{add_panels, col_style, collect_kinds, validate_f32_param, validate_kinds};

/// Builder for the master-stack preset layout.
pub struct MasterStack {
    kinds: Arc<[Arc<str>]>,
    master_ratio: f32,
    gap: f32,
}

impl MasterStack {
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
            _ => self.build_master_stack(),
        }
    }

    fn build_master_stack(&self) -> Result<Layout, PaneError> {
        let mut b = LayoutBuilder::new();
        let ratio = self.master_ratio;
        let master_kind = Arc::clone(&self.kinds[0]);
        let stack_style = col_style(1.0 - ratio, self.gap);

        b.row_gap(self.gap, |r| {
            r.panel_with(master_kind, grow(ratio));
            r.taffy_node(stack_style, |c| add_panels(c, &self.kinds[1..], grow(1.0)));
        })?;

        b.build()
    }
}

super::impl_preset!(
    MasterStack,
    runtime(kinds, |this| crate::strategy::StrategyKind::MasterStack {
        master_ratio: this.master_ratio,
        gap: this.gap,
    })
);
