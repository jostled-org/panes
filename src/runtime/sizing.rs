use crate::error::PaneError;
use crate::node::PanelId;
use crate::panel::Constraints;
use crate::validate::{check_f32_non_negative, float_invalid_to_constraint};

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
        check_f32_non_negative(width).map_err(|err| {
            PaneError::InvalidConstraint(float_invalid_to_constraint("width", err))
        })?;
        check_f32_non_negative(height).map_err(|err| {
            PaneError::InvalidConstraint(float_invalid_to_constraint("height", err))
        })?;

        self.tree
            .node_for_panel(pid)
            .ok_or(PaneError::PanelNotFound(pid))?;

        let idx = pid.raw() as usize;
        if idx >= self.panel_sizes.len() {
            self.panel_sizes.resize(idx + 1, None);
        }
        let new_val = Some((width, height));
        match self.panel_sizes[idx] == new_val {
            true => {}
            false => {
                self.panel_sizes[idx] = new_val;
                self.invalidate_layout();
            }
        }
        Ok(())
    }

    /// Update a panel's constraints without invalidating kind caches.
    ///
    /// Unlike `tree_mut().set_constraints()`, this marks only layout dirty,
    /// preserving cached kind indices and sorted kind keys across the next resolve.
    pub fn set_constraints(
        &mut self,
        pid: PanelId,
        constraints: Constraints,
    ) -> Result<(), PaneError> {
        self.tree.set_constraints(pid, constraints)?;
        self.invalidate_layout();
        Ok(())
    }

    /// Clear a panel's intrinsic size, reverting to constraint-based layout.
    pub fn clear_panel_size(&mut self, pid: PanelId) -> Result<(), PaneError> {
        self.tree
            .node_for_panel(pid)
            .ok_or(PaneError::PanelNotFound(pid))?;

        let idx = pid.raw() as usize;
        match self.panel_sizes.get_mut(idx) {
            Some(slot @ Some(_)) => {
                *slot = None;
                self.invalidate_layout();
            }
            _ => {}
        }
        Ok(())
    }
}
