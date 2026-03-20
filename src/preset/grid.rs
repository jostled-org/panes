use std::sync::Arc;

use crate::error::PaneError;
use crate::layout::Layout;

/// Builder for the grid preset layout.
///
/// # Deprecated
/// Use [`Dashboard`](super::Dashboard) with span-1 cards instead.
#[deprecated(since = "0.12.0", note = "use Layout::dashboard() with span-1 cards")]
#[allow(deprecated)]
pub struct Grid(super::Columns);

#[allow(deprecated)]
impl Grid {
    pub(crate) fn new(cols: usize, kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self(super::Columns::new(cols, kinds))
    }

    /// Use responsive `repeat(auto-fill, minmax(min_width, 1fr))` columns.
    pub fn auto_fill(self, min_width: f32) -> Self {
        Self(self.0.auto_fill(min_width))
    }

    /// Use responsive `repeat(auto-fit, minmax(min_width, 1fr))` columns.
    pub fn auto_fit(self, min_width: f32) -> Self {
        Self(self.0.auto_fit(min_width))
    }

    /// Set the gap between panels.
    pub fn gap(self, gap: f32) -> Self {
        Self(self.0.gap(gap))
    }

    /// Consume the builder and produce a [`Layout`].
    pub fn build(&self) -> Result<Layout, PaneError> {
        self.0.build()
    }

    /// Consume the builder and produce a [`crate::runtime::LayoutRuntime`].
    pub fn into_runtime(self) -> Result<crate::runtime::LayoutRuntime, PaneError> {
        self.0.into_runtime()
    }
}

super::impl_preset!(Grid);
