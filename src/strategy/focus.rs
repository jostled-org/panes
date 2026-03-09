use crate::error::PaneError;
use crate::node::PanelId;
use crate::panel::{Constraints, fixed, grow};
use crate::sequence::PanelSequence;
use crate::tree::LayoutTree;
use crate::viewport::ViewportState;

use super::StrategyKind;

/// Try to focus a specific panel, mutating constraints as needed.
///
/// Returns `true` if focus was applied, `false` if `pid` is not in the
/// sequence or the panel is missing from the tree.
pub fn try_apply_focus(
    strategy: &StrategyKind,
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    pid: PanelId,
) -> bool {
    match sequence.index_of(pid) {
        Some(_) => {}
        None => return false,
    }

    match strategy {
        StrategyKind::ActivePanel { .. } => focus_active_panel(tree, viewport, pid),
        StrategyKind::Deck { .. } => focus_deck(tree, sequence, viewport, pid),
        StrategyKind::Window { size, .. } => focus_window(tree, sequence, viewport, pid, *size),
        _ => {
            viewport.focus = Some(pid);
            true
        }
    }
}

/// Set constraints on a panel if it exists in the tree.
/// Only call with known-valid constraints (fixed(0.0), grow(1.0)).
fn set_constraints_if_present(
    tree: &mut LayoutTree,
    pid: PanelId,
    constraints: Constraints,
) -> bool {
    tree.set_constraints(pid, constraints).is_ok()
}

fn focus_active_panel(tree: &mut LayoutTree, viewport: &mut ViewportState, pid: PanelId) -> bool {
    match viewport.focus {
        Some(prev) if prev == pid => return true,
        Some(prev) => {
            set_constraints_if_present(tree, prev, fixed(0.0));
        }
        None => {}
    }
    match set_constraints_if_present(tree, pid, grow(1.0)) {
        true => {
            viewport.focus = Some(pid);
            true
        }
        false => false,
    }
}

fn focus_deck(
    tree: &mut LayoutTree,
    sequence: &PanelSequence,
    viewport: &mut ViewportState,
    pid: PanelId,
) -> bool {
    let prev_is_stack = viewport
        .focus
        .and_then(|p| sequence.index_of(p).map(|i| (p, i)));
    match prev_is_stack {
        Some((prev, _)) if prev == pid => return true,
        Some((prev, i)) if i > 0 => {
            set_constraints_if_present(tree, prev, fixed(0.0));
            set_constraints_if_present(tree, pid, grow(1.0));
        }
        _ => focus_deck_full(tree, sequence, pid),
    }
    viewport.focus = Some(pid);
    true
}

/// Hide all non-target stack panels, show only `pid`.
fn focus_deck_full(tree: &mut LayoutTree, sequence: &PanelSequence, pid: PanelId) {
    for spid in sequence.iter().skip(1) {
        let c = if spid == pid { grow(1.0) } else { fixed(0.0) };
        set_constraints_if_present(tree, spid, c);
    }
}

fn focus_window(
    tree: &mut LayoutTree,
    sequence: &PanelSequence,
    viewport: &mut ViewportState,
    pid: PanelId,
    size: usize,
) -> bool {
    let index = match sequence.index_of(pid) {
        Some(i) => i,
        None => return false,
    };
    let ws = viewport.window_start;
    let in_window = index >= ws && index < ws + size;

    match in_window {
        true => {}
        false => {
            let len = sequence.len();
            let raw_start = window_start_for_index(index, ws, size);
            viewport.window_start = raw_start.min(len.saturating_sub(size));
            apply_window_constraints_best_effort(tree, sequence, viewport.window_start, size);
        }
    }

    viewport.focus = Some(pid);
    true
}

/// Best-effort window constraints: skips panels that are missing from the tree.
fn apply_window_constraints_best_effort(
    tree: &mut LayoutTree,
    sequence: &PanelSequence,
    start: usize,
    size: usize,
) {
    let _ = apply_window_constraints(tree, sequence, start, size);
}

fn window_start_for_index(index: usize, current_start: usize, size: usize) -> usize {
    match index < current_start {
        true => index,
        false => index.saturating_sub(size - 1),
    }
}

/// Set window visibility constraints: panels in [start, start+size) get grow(1.0),
/// all others get fixed(0.0).
pub(super) fn apply_window_constraints(
    tree: &mut LayoutTree,
    sequence: &PanelSequence,
    start: usize,
    size: usize,
) -> Result<(), PaneError> {
    for (i, pid) in sequence.iter().enumerate() {
        let visible = i >= start && i < start + size;
        let constraint = match visible {
            true => grow(1.0),
            false => fixed(0.0),
        };
        tree.set_constraints(pid, constraint)?;
    }
    Ok(())
}
