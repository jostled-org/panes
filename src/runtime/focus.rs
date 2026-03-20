use std::sync::Arc;

use super::types::{LayoutRuntime, strategy_ref};
use crate::error::{MutationError, PaneError, ViewportError};
use crate::focus::{self, FocusDirection};
use crate::node::PanelId;
use crate::panel::fixed;
use crate::resolver::ResolvedLayout;

impl LayoutRuntime {
    /// Toggle a panel's collapsed state.
    ///
    /// Collapsing saves the current constraints and sets the panel to fixed(0.0).
    /// Uncollapsing restores the saved constraints.
    pub fn toggle_collapsed(&mut self, pid: PanelId) -> Result<(), PaneError> {
        match self.viewport.collapsed.contains(&pid) {
            true => {
                let saved = self.viewport.saved_constraints.remove(&pid).ok_or(
                    PaneError::InvalidViewport(ViewportError::NoSavedConstraints(pid)),
                )?;
                self.tree.set_constraints(pid, saved)?;
                self.viewport.collapsed.remove(&pid);
                Ok(())
            }
            false => {
                let current = self.tree.panel_constraints(pid)?;
                self.viewport.saved_constraints.insert(pid, current);
                self.tree.set_constraints(pid, fixed(0.0))?;
                self.viewport.collapsed.insert(pid);
                Ok(())
            }
        }
    }

    /// Shift the scroll offset by a delta.
    pub fn scroll_by(&mut self, delta: f32) -> Result<(), PaneError> {
        crate::validate::check_f32_finite(delta)
            .map_err(|_| PaneError::InvalidViewport(ViewportError::ScrollNotFinite))?;
        self.viewport.scroll_offset += delta;
        Ok(())
    }

    /// Set the scroll offset to an absolute value.
    pub fn scroll_to(&mut self, offset: f32) -> Result<(), PaneError> {
        crate::validate::check_f32_finite(offset)
            .map_err(|_| PaneError::InvalidViewport(ViewportError::ScrollNotFinite))?;
        self.viewport.scroll_offset = offset;
        Ok(())
    }

    /// Set focus to a panel without strategy validation.
    ///
    /// Unlike [`focus`](Self::focus), this bypasses strategy-specific focus
    /// logic (e.g. updating tab visibility in `ActivePanel` layouts).
    /// Use when you need raw focus control outside the strategy system.
    pub fn set_focus_unchecked(&mut self, pid: PanelId) {
        self.viewport.focus = Some(pid);
    }

    /// Set focus to a specific panel.
    ///
    /// Returns `true` if focus was set, `false` if `pid` is not in the
    /// sequence (strategy path) or not a known panel.
    pub fn focus(&mut self, pid: PanelId) -> bool {
        let Some(strategy) =
            strategy_ref(&self.strategy_source, &self.breakpoints, self.active_bp_idx)
        else {
            self.set_focus_unchecked(pid);
            return true;
        };
        crate::strategy::try_apply_focus(
            strategy,
            &mut self.tree,
            &mut self.sequence,
            &mut self.viewport,
            pid,
        )
    }

    /// Swap the focused panel with the next panel in the sequence (wrapping).
    /// No-op if there is no focus or fewer than two panels.
    pub fn swap_next(&mut self) {
        self.swap_by(1);
    }

    /// Swap the focused panel with the previous panel in the sequence (wrapping).
    /// No-op if there is no focus or fewer than two panels.
    pub fn swap_prev(&mut self) {
        self.swap_by(-1);
    }

    fn swap_by(&mut self, delta: isize) {
        let (pid, idx) = match (
            self.viewport.focus,
            self.viewport.focus.and_then(|c| self.sequence.index_of(c)),
        ) {
            (Some(pid), Some(idx)) => (pid, idx),
            _ => return,
        };
        let len = self.sequence.len();
        match len <= 1 {
            true => return,
            false => {}
        }
        let target = ((idx as isize + delta).rem_euclid(len as isize)) as usize;
        // MoveNotSupported is expected for slotted strategies (swap is a
        // no-op). Other errors cannot occur: rem_euclid guarantees bounds
        // and rebuild cannot fail with len > 1.
        let _ = self.move_panel(pid, target);
    }

    /// Move focus to the next panel in the sequence.
    /// No-op if the sequence is empty.
    pub fn focus_next(&mut self) {
        self.focus_by(1);
    }

    /// Move focus to the previous panel in the sequence.
    /// No-op if the sequence is empty.
    pub fn focus_prev(&mut self) {
        self.focus_by(-1);
    }

    fn focus_by(&mut self, delta: isize) {
        let target = match (
            self.viewport.focus,
            self.viewport.focus.and_then(|c| self.sequence.index_of(c)),
        ) {
            (Some(_), Some(idx)) => {
                let len = self.sequence.len().max(1);
                let next_idx = ((idx as isize + delta).rem_euclid(len as isize)) as usize;
                self.sequence.get(next_idx)
            }
            _ => self.sequence.get(0),
        };
        if let Some(pid) = target {
            self.focus(pid);
        }
    }

    /// Move focus to the nearest panel in a spatial direction.
    ///
    /// Returns `Ok(Some(target))` when focus moved, `Ok(None)` when no
    /// candidate exists in that direction or no panel is focused.
    ///
    /// Returns `Err(SpatialNavUnsupported)` for strategies where spatial
    /// navigation is meaningless (ActivePanel, Window). Use
    /// `focus_next`/`focus_prev` instead.
    pub fn focus_direction(
        &mut self,
        layout: &ResolvedLayout,
        direction: FocusDirection,
    ) -> Result<Option<PanelId>, PaneError> {
        self.check_spatial_nav()?;
        let Some(focused) = self.focused() else {
            return Ok(None);
        };
        let Some(target) = focus::find_nearest(layout, focused, &self.sequence, direction) else {
            return Ok(None);
        };
        self.focus(target);
        Ok(Some(target))
    }

    /// Move focus to the nearest panel in a spatial direction, using the
    /// most recently resolved layout.
    ///
    /// Returns `Ok(Some(target))` when focus moved, `Ok(None)` when no
    /// layout has been resolved, no panel is focused, or no candidate exists.
    ///
    /// Returns `Err(SpatialNavUnsupported)` for strategies where spatial
    /// navigation is meaningless (ActivePanel, Window). Use
    /// `focus_next`/`focus_prev` instead.
    pub fn focus_direction_current(
        &mut self,
        direction: FocusDirection,
    ) -> Result<Option<PanelId>, PaneError> {
        self.check_spatial_nav()?;
        let Some(layout) = self.previous.as_ref().map(Arc::clone) else {
            return Ok(None);
        };
        self.focus_direction(&layout, direction)
    }

    fn check_spatial_nav(&self) -> Result<(), PaneError> {
        match self.strategy() {
            Some(s) if !s.supports_spatial_nav() => Err(PaneError::InvalidMutation(
                MutationError::SpatialNavUnsupported,
            )),
            _ => Ok(()),
        }
    }
}
