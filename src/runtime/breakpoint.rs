use super::types::{LayoutRuntime, strategy_ref};
use crate::breakpoint;
use crate::error::PaneError;

impl LayoutRuntime {
    pub(crate) fn maybe_switch_breakpoint(&mut self, width: f32) -> Result<(), PaneError> {
        let breakpoints = match self.breakpoints.as_ref() {
            Some(bp) => bp,
            None => return Ok(()),
        };
        let new_idx = breakpoint::select_breakpoint(breakpoints, width);
        match new_idx == self.active_bp_idx {
            true => Ok(()),
            false => self.apply_breakpoint_switch(new_idx),
        }
    }

    fn apply_breakpoint_switch(&mut self, new_idx: usize) -> Result<(), PaneError> {
        let Some(breakpoints) = self.breakpoints.as_ref() else {
            return Ok(());
        };

        let focused_kind = self.focused_kind_arc();
        let collapsed_kinds: Box<[_]> = self
            .viewport
            .collapsed
            .iter()
            .filter_map(|&pid| self.tree.panel_kind_arc(pid).ok())
            .collect();

        breakpoint::rebuild_for_breakpoint(
            breakpoints,
            new_idx,
            &mut self.tree,
            &mut self.sequence,
            &mut self.cached_compile,
            &mut self.cached_kinds,
            &mut self.cached_sorted_kind_keys,
        )?;

        self.viewport.collapsed.clear();
        self.viewport.saved_constraints.clear();

        // active_bp_idx must be updated before strategy() is called
        self.active_bp_idx = new_idx;

        breakpoint::restore_breakpoint_viewport(
            &mut self.tree,
            &mut self.sequence,
            &mut self.viewport,
            strategy_ref(&self.strategy_source, &self.breakpoints, self.active_bp_idx),
            focused_kind,
            &collapsed_kinds,
        )?;

        Ok(())
    }
}
