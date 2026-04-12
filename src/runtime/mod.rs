mod adjacent;
mod breakpoint;
mod construct;
mod focus;
mod frame;
mod mutate;
mod overlay;
mod placement;
mod resolve;
mod sizing;
mod types;

pub use frame::Frame;
pub use placement::Placement;
pub use types::LayoutRuntime;

use std::sync::Arc;

use types::{strategy_ref, strategy_ref_mut};

use crate::breakpoint::BreakpointEntry;
use crate::node::{PanelId, PanelKey};
use crate::sequence::PanelSequence;
use crate::strategy::StrategyKind;
use crate::tree::LayoutTree;
use crate::viewport::ViewportState;

impl LayoutRuntime {
    /// The breakpoint entries, if this is an adaptive layout.
    pub fn breakpoints(&self) -> Option<&[BreakpointEntry]> {
        self.breakpoints.as_deref()
    }

    /// The index of the currently active breakpoint.
    pub fn active_breakpoint_index(&self) -> usize {
        self.active_bp_idx
    }

    /// Immutable access to the underlying tree.
    pub fn tree(&self) -> &LayoutTree {
        &self.tree
    }

    /// Mutable access to the underlying tree for structural mutations.
    ///
    /// Clears cached compile and kind data to prevent stale reads.
    pub fn tree_mut(&mut self) -> &mut LayoutTree {
        self.invalidate_topology();
        &mut self.tree
    }

    /// Immutable access to the viewport state.
    pub fn viewport(&self) -> &ViewportState {
        &self.viewport
    }

    /// The layout strategy, if this runtime was created via `from_strategy`.
    pub fn strategy(&self) -> Option<&StrategyKind> {
        strategy_ref(&self.strategy_source, &self.breakpoints, self.active_bp_idx)
    }

    /// Mutable reference to the active strategy for in-place updates.
    pub(crate) fn strategy_mut(&mut self) -> Option<&mut StrategyKind> {
        strategy_ref_mut(
            &mut self.strategy_source,
            &mut self.breakpoints,
            self.active_bp_idx,
        )
    }

    /// The panel sequence (logical order).
    pub fn sequence(&self) -> &PanelSequence {
        &self.sequence
    }

    /// The currently focused panel.
    pub fn focused(&self) -> Option<PanelId> {
        self.viewport.focus
    }

    /// The kind of the currently focused panel.
    pub fn focused_kind(&self) -> Option<&str> {
        let pid = self.viewport.focus?;
        self.tree.panel_kind(pid).ok()
    }

    /// The kind of the currently focused panel as an owned `Arc<str>`.
    pub fn focused_kind_arc(&self) -> Option<Arc<str>> {
        let pid = self.viewport.focus?;
        self.tree.panel_kind_arc(pid).ok()
    }

    /// Control whether resolve collects boundary segments for hit-testing.
    ///
    /// When `false`, `boundary_at_point` on the resulting layout always returns `None`,
    /// but resolve runs faster by skipping per-sibling layout lookups.
    pub fn set_collect_boundaries(&mut self, collect: bool) {
        self.resolve_scratch.collect_boundaries = collect;
    }

    /// The stable identity key for a panel, derived from its sequence position.
    ///
    /// Returns `None` if the panel is not in the sequence (e.g. decoration panels).
    pub fn panel_key(&self, pid: PanelId) -> Option<PanelKey> {
        self.sequence
            .index_of(pid)
            .map(|idx| PanelKey::from_raw(idx as u32))
    }

    /// The stable identity key of the currently focused panel.
    pub fn focused_panel_key(&self) -> Option<PanelKey> {
        self.viewport.focus.and_then(|pid| self.panel_key(pid))
    }

    /// Whether `pid` is a decorative panel (tab bar, title bar) for `content_pid`.
    pub fn is_decoration_for(&self, pid: PanelId, content_pid: PanelId) -> bool {
        let Some(meta) = self.tree.decoration_meta(pid) else {
            return false;
        };
        let Ok(content_kind) = self.tree.panel_kind(content_pid) else {
            return false;
        };
        meta.content_kind.as_ref() == content_kind
    }
}
