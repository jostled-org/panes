use std::sync::Arc;

use super::types::{LayoutRuntime, strategy_ref};
use crate::error::{MutationError, PaneError, ViewportError};
use crate::focus::{self, FocusDirection};
use crate::focus_outcome::{FocusOutcome, FocusRejection};
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
                let saved = *self.viewport.saved_constraints.get(&pid).ok_or(
                    PaneError::InvalidViewport(ViewportError::NoSavedConstraints(pid)),
                )?;
                self.tree.set_constraints(pid, saved)?;
                self.viewport.saved_constraints.remove(&pid);
                self.viewport.collapsed.remove(&pid);
            }
            false => {
                let current = self.tree.panel_constraints(pid)?;
                self.tree.set_constraints(pid, fixed(0.0))?;
                self.viewport.saved_constraints.insert(pid, current);
                self.viewport.collapsed.insert(pid);
            }
        }
        self.invalidate_layout();
        Ok(())
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
    /// Returns `Applied` when focus moved, `Unchanged` when the panel
    /// was already focused, or `Rejected` when the panel is missing or
    /// the strategy rejected the request.
    pub fn focus(&mut self, pid: PanelId) -> FocusOutcome {
        let Some(strategy) =
            strategy_ref(&self.strategy_source, &self.breakpoints, self.active_bp_idx)
        else {
            return self.focus_no_strategy(pid);
        };
        let outcome = crate::strategy::try_apply_focus(
            strategy,
            &mut self.tree,
            &mut self.sequence,
            &mut self.viewport,
            pid,
        );
        // Strategy focus may modify constraints (visibility toggling).
        // Mark layout dirty so the dirty-state model stays consistent.
        if outcome.is_applied() && self.tree.is_dirty() {
            self.invalidate_layout();
        }
        outcome
    }

    fn focus_no_strategy(&mut self, pid: PanelId) -> FocusOutcome {
        let exists = self.tree.node_for_panel(pid).is_some();
        let already = matches!(self.viewport.focus, Some(prev) if prev == pid);
        match (exists, already) {
            (false, _) => FocusOutcome::Rejected(FocusRejection::PanelNotFound),
            (true, true) => FocusOutcome::Unchanged,
            (true, false) => {
                self.set_focus_unchecked(pid);
                FocusOutcome::Applied
            }
        }
    }

    /// Swap the focused panel with the next panel in the sequence (wrapping).
    ///
    /// Returns `Ok(())` when the swap succeeds or is a documented no-op.
    /// Slotted strategies remain a no-op because move is unsupported there.
    pub fn swap_next(&mut self) -> Result<(), PaneError> {
        self.swap_by(1)
    }

    /// Swap the focused panel with the previous panel in the sequence (wrapping).
    ///
    /// Returns `Ok(())` when the swap succeeds or is a documented no-op.
    /// Slotted strategies remain a no-op because move is unsupported there.
    pub fn swap_prev(&mut self) -> Result<(), PaneError> {
        self.swap_by(-1)
    }

    fn swap_by(&mut self, delta: isize) -> Result<(), PaneError> {
        let (pid, idx) = match (
            self.viewport.focus,
            self.viewport.focus.and_then(|c| self.sequence.index_of(c)),
        ) {
            (Some(pid), Some(idx)) => (pid, idx),
            _ => return Ok(()),
        };
        let len = self.sequence.len();
        match len <= 1 {
            true => return Ok(()),
            false => {}
        }
        let target = ((idx as isize + delta).rem_euclid(len as isize)) as usize;
        match self.move_panel(pid, target) {
            Ok(_) | Err(PaneError::InvalidMutation(MutationError::MoveNotSupported)) => Ok(()),
            Err(err) => Err(err),
        }
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
    /// Returns the spatial candidate (if any) paired with the focus outcome.
    /// When no candidate exists or no panel is focused, returns `(None, Unchanged)`.
    /// When a candidate is found but the strategy rejects it, returns
    /// `(Some(target), Rejected(reason))`.
    ///
    /// Returns `Err(SpatialNavUnsupported)` for strategies where spatial
    /// navigation is meaningless (ActivePanel, Window). Use
    /// `focus_next`/`focus_prev` instead.
    pub fn focus_direction(
        &mut self,
        layout: &ResolvedLayout,
        direction: FocusDirection,
    ) -> Result<(Option<PanelId>, FocusOutcome), PaneError> {
        self.check_spatial_nav()?;
        let Some(focused) = self.focused() else {
            return Ok((None, FocusOutcome::Unchanged));
        };
        let Some(target) = focus::find_nearest(layout, focused, &self.sequence, direction) else {
            return Ok((None, FocusOutcome::Unchanged));
        };
        let outcome = self.focus(target);
        Ok((Some(target), outcome))
    }

    /// Move focus to the nearest panel in a spatial direction, using the
    /// most recently resolved layout.
    ///
    /// Same semantics as [`focus_direction`](Self::focus_direction), but uses
    /// the cached layout from the most recent `resolve()` call.
    /// Returns `(None, Unchanged)` when no layout has been resolved.
    ///
    /// Returns `Err(SpatialNavUnsupported)` for strategies where spatial
    /// navigation is meaningless (ActivePanel, Window). Use
    /// `focus_next`/`focus_prev` instead.
    pub fn focus_direction_current(
        &mut self,
        direction: FocusDirection,
    ) -> Result<(Option<PanelId>, FocusOutcome), PaneError> {
        self.check_spatial_nav()?;
        let Some(layout) = self.previous.as_ref().map(Arc::clone) else {
            return Ok((None, FocusOutcome::Unchanged));
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
