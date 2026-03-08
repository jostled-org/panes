use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::{fixed, grow};
use crate::preset::{collect_kinds, validate_active, validate_kinds};

/// Builder for the scrollable preset layout.
///
/// NIRI-style scrolling: shows two panels side by side, filling the viewport.
/// The `active` index is the focused panel. The window position is derived
/// so that the focused panel is always visible.
pub struct Scrollable {
    kinds: Arc<[Arc<str>]>,
    active: usize,
    gap: f32,
}

impl Scrollable {
    pub(crate) fn new(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self {
            kinds: collect_kinds(kinds),
            active: 0,
            gap: 0.0,
        }
    }

    /// Set the focused panel index. The visible window is derived from focus.
    pub fn active(mut self, index: usize) -> Self {
        self.active = index;
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
        validate_active(self.active, self.kinds.len())?;

        match self.kinds.len() {
            1 => super::build_single(Arc::clone(&self.kinds[0])),
            _ => self.build_scroll(),
        }
    }

    fn build_scroll(&self) -> Result<Layout, PaneError> {
        let mut b = LayoutBuilder::new();
        let window_start = window_start_from_focus(self.active, self.kinds.len(), 2);
        let gap_px = self.gap;
        let kinds = &self.kinds;

        b.row_gap(gap_px, |r| add_scroll_panels(r, kinds, window_start))?;

        b.build()
    }
}

/// Derive the window start so that `focus` is visible in a window of `size` panels.
fn window_start_from_focus(focus: usize, len: usize, size: usize) -> usize {
    let start = (focus + 1).saturating_sub(size);
    start.min(len.saturating_sub(size))
}

/// Show panels at `window` and `window + 1`; hide everything else.
fn add_scroll_panels(ctx: &mut crate::ContainerCtx, kinds: &[Arc<str>], window: usize) {
    for (i, kind) in kinds.iter().enumerate() {
        let visible = i == window || i == window + 1;
        let constraint = match visible {
            true => grow(1.0),
            false => fixed(0.0),
        };
        ctx.panel_with(Arc::clone(kind), constraint);
    }
}

impl Scrollable {
    /// Consume the builder and produce a [`crate::runtime::LayoutRuntime`].
    pub fn into_runtime(self) -> Result<crate::runtime::LayoutRuntime, PaneError> {
        let strategy = crate::strategy::StrategyKind::Window {
            size: 2,
            gap: self.gap,
        };
        let kinds: Vec<Arc<str>> = self.kinds.to_vec();
        crate::runtime::LayoutRuntime::from_strategy(strategy, &kinds)
    }
}

super::impl_preset!(Scrollable);
