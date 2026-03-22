use crate::error::PaneError;
use crate::node::PanelId;

use super::types::LayoutRuntime;

impl LayoutRuntime {
    /// Set a panel's intrinsic size for the next resolve.
    ///
    /// Overrides constraint-based sizing until cleared via
    /// [`clear_panel_size`](Self::clear_panel_size).
    pub fn set_panel_size(
        &mut self,
        pid: PanelId,
        width: f32,
        height: f32,
    ) -> Result<(), PaneError> {
        self.tree
            .node_for_panel(pid)
            .ok_or(PaneError::PanelNotFound(pid))?;

        let idx = pid.raw() as usize;
        if idx >= self.panel_sizes.len() {
            self.panel_sizes.resize(idx + 1, None);
        }
        self.panel_sizes[idx] = Some((width, height));
        self.cached_compile = None;
        Ok(())
    }

    /// Clear a panel's intrinsic size, reverting to constraint-based layout.
    pub fn clear_panel_size(&mut self, pid: PanelId) -> Result<(), PaneError> {
        self.tree
            .node_for_panel(pid)
            .ok_or(PaneError::PanelNotFound(pid))?;

        let idx = pid.raw() as usize;
        if idx < self.panel_sizes.len() {
            self.panel_sizes[idx] = None;
        }
        self.cached_compile = None;
        Ok(())
    }
}
