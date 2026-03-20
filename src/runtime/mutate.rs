use std::sync::Arc;

use super::types::{LayoutRuntime, strategy_ref};
use crate::error::{MutationError, PaneError};
use crate::node::PanelId;
use crate::runtime::placement::Placement;

impl LayoutRuntime {
    /// Add a panel using the active strategy.
    ///
    /// With a strategy: inserts after focused (or at end if no focus),
    /// then rebuilds the tree through the strategy's preset builder.
    ///
    /// Without a strategy: hyprland-style split (auto-direction from
    /// aspect ratio, `grow(1.0)`, placed after focused).
    pub fn add_panel(&mut self, kind: Arc<str>) -> Result<PanelId, PaneError> {
        self.add_panel_with(kind, Placement::After)
    }

    /// Add a panel at an explicit position relative to focused.
    ///
    /// - `Placement::Before` — before focused
    /// - `Placement::After` — after focused (same as `add_panel`)
    /// - `Placement::End` — append to sequence end
    pub fn add_panel_with(
        &mut self,
        kind: Arc<str>,
        placement: Placement,
    ) -> Result<PanelId, PaneError> {
        match strategy_ref(&self.strategy_source, &self.breakpoints, self.active_bp_idx) {
            Some(strategy) => {
                let index = self.placement_to_index(placement);
                crate::strategy::apply_add(
                    strategy,
                    &mut self.tree,
                    &mut self.sequence,
                    &mut self.viewport,
                    kind,
                    index,
                )
            }
            None => {
                let direction = self.auto_direction();
                self.add_panel_adjacent_with(kind, direction, crate::panel::grow(1.0), placement)
            }
        }
    }

    /// Convert a Placement to a sequence index.
    pub(crate) fn placement_to_index(&self, placement: Placement) -> usize {
        match placement {
            Placement::Before => self
                .viewport
                .focus
                .and_then(|pid| self.sequence.index_of(pid))
                .unwrap_or(self.sequence.len()),
            Placement::After => self
                .viewport
                .focus
                .and_then(|pid| self.sequence.index_of(pid).map(|i| i + 1))
                .unwrap_or(self.sequence.len()),
            Placement::End => self.sequence.len(),
        }
    }

    /// Remove a panel using the active strategy. Returns the new focus panel.
    pub fn remove_panel(&mut self, pid: PanelId) -> Result<Option<PanelId>, PaneError> {
        let strategy = strategy_ref(&self.strategy_source, &self.breakpoints, self.active_bp_idx)
            .ok_or(PaneError::InvalidMutation(MutationError::NoStrategy))?;
        crate::strategy::apply_remove(
            strategy,
            &mut self.tree,
            &mut self.sequence,
            &mut self.viewport,
            pid,
        )
    }

    /// Move a panel to a new sequence index using the active strategy.
    pub fn move_panel(&mut self, pid: PanelId, new_index: usize) -> Result<PanelId, PaneError> {
        let strategy = strategy_ref(&self.strategy_source, &self.breakpoints, self.active_bp_idx)
            .ok_or(PaneError::InvalidMutation(MutationError::NoStrategy))?;
        crate::strategy::apply_move(
            strategy,
            &mut self.tree,
            &mut self.sequence,
            &mut self.viewport,
            pid,
            new_index,
        )
    }

    /// Change a dashboard card's column span and rebuild the grid.
    ///
    /// Returns an error if the runtime has no strategy or the strategy
    /// is not a dashboard variant.
    pub fn set_card_span(
        &mut self,
        pid: PanelId,
        span: crate::strategy::CardSpan,
    ) -> Result<(), PaneError> {
        let strategy = strategy_ref(&self.strategy_source, &self.breakpoints, self.active_bp_idx)
            .ok_or(PaneError::InvalidMutation(MutationError::NoStrategy))?;
        let new_strategy = crate::strategy::apply_set_card_span(
            strategy,
            &mut self.tree,
            &mut self.sequence,
            &mut self.viewport,
            pid,
            span,
        )?;
        match self.strategy_mut() {
            Some(s) => *s = new_strategy,
            None => {}
        }
        Ok(())
    }

    /// Resize a panel's share of its container by `delta` (fraction of container space).
    ///
    /// Positive delta gives the panel more space; negative gives it less.
    /// All siblings in the parent container must be panels with grow constraints.
    pub fn resize_boundary(&mut self, pid: PanelId, delta: f32) -> Result<(), PaneError> {
        crate::resize::resize_boundary(&mut self.tree, pid, delta)
    }
}
