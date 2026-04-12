use std::sync::Arc;

use crate::error::{PaneError, TreeError};
use crate::panel::fixed;
use crate::runtime::LayoutRuntime;
use crate::sequence::PanelSequence;
use crate::strategy::StrategyKind;
use crate::strategy::builder::Strategy;
use crate::tree::LayoutTree;
use crate::viewport::ViewportState;

/// A breakpoint entry mapping a minimum viewport width to a strategy.
#[derive(Debug, Clone)]
pub struct BreakpointEntry {
    pub(crate) min_width: u32,
    pub(crate) strategy: StrategyKind,
}

impl BreakpointEntry {
    /// The minimum viewport width (in pixels) that activates this breakpoint.
    pub fn min_width(&self) -> u32 {
        self.min_width
    }

    /// The strategy used at this breakpoint.
    pub fn strategy(&self) -> &StrategyKind {
        &self.strategy
    }
}

/// Builder for adaptive layouts that switch strategies at width breakpoints.
pub struct AdaptiveBuilder {
    panels: Box<[Arc<str>]>,
    breakpoints: Vec<BreakpointEntry>,
}

impl AdaptiveBuilder {
    pub(crate) fn new(panels: Box<[Arc<str>]>) -> Self {
        Self {
            panels,
            breakpoints: Vec::new(),
        }
    }

    /// Add a breakpoint: when viewport width >= `min_width`, use this strategy.
    pub fn at(mut self, min_width: u32, strategy: impl Into<Strategy>) -> Self {
        let strategy: Strategy = strategy.into();
        self.breakpoints.push(BreakpointEntry {
            min_width,
            strategy: strategy.kind,
        });
        self
    }

    /// Build the adaptive runtime. Requires at least one breakpoint.
    pub fn into_runtime(mut self) -> Result<LayoutRuntime, PaneError> {
        match self.breakpoints.is_empty() {
            true => return Err(PaneError::InvalidTree(TreeError::NoBreakpoints)),
            false => {}
        }
        self.breakpoints.sort_by_key(|bp| bp.min_width);
        validate_breakpoint_widths(&self.breakpoints)?;
        let active_idx = 0;
        let breakpoints: Box<[BreakpointEntry]> = self.breakpoints.into();
        LayoutRuntime::from_adaptive(&self.panels, breakpoints, active_idx)
    }
}

fn validate_breakpoint_widths(breakpoints: &[BreakpointEntry]) -> Result<(), PaneError> {
    for pair in breakpoints.windows(2) {
        if pair[0].min_width == pair[1].min_width {
            return Err(PaneError::InvalidTree(
                TreeError::DuplicateBreakpointWidth {
                    width: pair[1].min_width,
                },
            ));
        }
    }

    Ok(())
}

/// Find the breakpoint index whose `min_width` is the largest that doesn't
/// exceed `width`. Breakpoints must be sorted ascending by `min_width`.
///
/// Width is truncated to an integer (799.9 → 799) for pixel-aligned
/// thresholds. Negative, NaN, or infinite widths saturate per Rust's
/// `as` cast semantics; `resolve()` validates dimensions before this
/// function is called.
pub(crate) fn select_breakpoint(breakpoints: &[BreakpointEntry], width: f32) -> usize {
    let w = width as u32;
    let idx = breakpoints.partition_point(|bp| bp.min_width <= w);
    idx.saturating_sub(1)
}

/// Rebuild tree + sequence for a new breakpoint. Returns collected kinds
/// for focus restoration.
///
/// The strategy is borrowed from `breakpoints[new_idx]` — no clone needed.
pub(crate) fn rebuild_for_breakpoint(
    breakpoints: &[BreakpointEntry],
    new_idx: usize,
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
) -> Result<Box<[Arc<str>]>, PaneError> {
    let kinds = crate::strategy::collect_kinds_from_sequence(tree, sequence);

    let strategy = &breakpoints[new_idx].strategy;
    let new_tree = crate::strategy::build_tree_for_strategy(strategy, &kinds)?;

    let mut new_seq = PanelSequence::default();
    crate::strategy::populate_sequence_by_kinds(&new_tree, &kinds, &mut new_seq);

    *tree = new_tree;
    *sequence = new_seq;

    Ok(kinds)
}

/// Restore focus and collapsed state after a breakpoint switch.
///
/// Uses sequence indices for deterministic restore when panels share a kind.
pub(crate) fn restore_breakpoint_viewport(
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    strategy: Option<&StrategyKind>,
    focused_seq_idx: Option<usize>,
    collapsed_seq_indices: &[usize],
) -> Result<(), PaneError> {
    let focus_pid = focused_seq_idx
        .and_then(|idx| sequence.get(idx))
        .or_else(|| sequence.get(0));
    viewport.focus = focus_pid;
    match (focus_pid, strategy) {
        (Some(pid), Some(s)) => {
            crate::strategy::try_apply_focus(s, tree, sequence, viewport, pid);
        }
        _ => {}
    }

    for &idx in collapsed_seq_indices {
        let Some(pid) = sequence.get(idx) else {
            continue;
        };
        let Ok(current) = tree.panel_constraints(pid) else {
            continue;
        };
        viewport.saved_constraints.insert(pid, current);
        tree.set_constraints(pid, fixed(0.0))?;
        viewport.collapsed.insert(pid);
    }
    Ok(())
}
