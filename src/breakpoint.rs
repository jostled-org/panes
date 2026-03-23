use std::sync::Arc;

use crate::compiler::CompileResult;
use crate::error::{PaneError, TreeError};
use crate::panel::fixed;
use crate::resolver;
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
        let active_idx = 0;
        let breakpoints: Box<[BreakpointEntry]> = self.breakpoints.into();
        LayoutRuntime::from_adaptive(&self.panels, breakpoints, active_idx)
    }
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
    cached_compile: &mut Option<CompileResult>,
    cached_kinds: &mut Option<resolver::KindIndex>,
) -> Result<Box<[Arc<str>]>, PaneError> {
    let kinds = crate::strategy::collect_kinds_from_sequence(tree, sequence);

    let strategy = &breakpoints[new_idx].strategy;
    let new_tree = crate::strategy::build_tree_for_strategy(strategy, &kinds)?;

    let mut new_seq = PanelSequence::default();
    crate::strategy::populate_sequence_by_kinds(&new_tree, &kinds, &mut new_seq);

    *tree = new_tree;
    *sequence = new_seq;
    *cached_compile = None;
    *cached_kinds = None;

    Ok(kinds)
}

/// Restore focus and collapsed state after a breakpoint switch.
pub(crate) fn restore_breakpoint_viewport(
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    strategy: Option<&StrategyKind>,
    focused_kind: Option<Arc<str>>,
    collapsed_kinds: &[Arc<str>],
) -> Result<(), PaneError> {
    let focus_pid = focused_kind
        .and_then(|kind| tree.panels_by_kind(&kind).first().copied())
        .or_else(|| sequence.get(0));
    viewport.focus = focus_pid;
    match (focus_pid, strategy) {
        (Some(pid), Some(s)) => {
            crate::strategy::try_apply_focus(s, tree, sequence, viewport, pid);
        }
        _ => {}
    }

    for kind in collapsed_kinds {
        if let Some(&pid) = tree.panels_by_kind(kind).first() {
            let Ok(current) = tree.panel_constraints(pid) else {
                continue;
            };
            viewport.saved_constraints.insert(pid, current);
            tree.set_constraints(pid, fixed(0.0))?;
            viewport.collapsed.insert(pid);
        }
    }
    Ok(())
}
