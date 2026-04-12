use crate::error::PaneError;
use crate::focus_outcome::{FocusOutcome, FocusRejection};
use crate::node::PanelId;
use crate::panel::{Constraints, fixed, grow};
use crate::sequence::PanelSequence;
use crate::tree::LayoutTree;
use crate::viewport::ViewportState;

use super::StrategyKind;

/// Try to focus a specific panel, mutating constraints as needed.
///
/// Returns the outcome: `Applied` if focus moved, `Unchanged` if the
/// panel was already focused, or `Rejected` if the panel is not in
/// the sequence or strategy-level application failed.
pub fn try_apply_focus(
    strategy: &StrategyKind,
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    pid: PanelId,
) -> FocusOutcome {
    match sequence.index_of(pid) {
        Some(_) => {}
        None => return FocusOutcome::Rejected(FocusRejection::PanelNotFound),
    }

    match strategy {
        StrategyKind::ActivePanel { .. } => focus_active_panel(tree, viewport, pid),
        StrategyKind::Deck { .. } => focus_deck(tree, sequence, viewport, pid),
        StrategyKind::Window { panel_count, .. } => {
            focus_window(tree, sequence, viewport, pid, *panel_count)
        }
        _ => focus_simple(viewport, pid),
    }
}

fn focus_simple(viewport: &mut ViewportState, pid: PanelId) -> FocusOutcome {
    let unchanged = matches!(viewport.focus, Some(prev) if prev == pid);
    viewport.focus = Some(pid);
    match unchanged {
        true => FocusOutcome::Unchanged,
        false => FocusOutcome::Applied,
    }
}

fn panels_are_present(tree: &LayoutTree, panels: impl IntoIterator<Item = PanelId>) -> bool {
    panels
        .into_iter()
        .all(|pid| tree.node_for_panel(pid).is_some())
}

fn require_panels_present(
    tree: &LayoutTree,
    panels: impl IntoIterator<Item = PanelId>,
) -> Result<(), FocusRejection> {
    match panels_are_present(tree, panels) {
        true => Ok(()),
        false => Err(FocusRejection::StrategyRejected),
    }
}

/// Set constraints on a panel.
/// Only call with known-valid constraints (fixed(0.0), grow(1.0)).
fn set_constraints(
    tree: &mut LayoutTree,
    pid: PanelId,
    constraints: Constraints,
) -> Result<(), FocusRejection> {
    tree.set_constraints(pid, constraints)
        .map_err(|_| FocusRejection::StrategyRejected)
}

fn hide_prev_active_panel(
    tree: &mut LayoutTree,
    prev: PanelId,
    pid: PanelId,
) -> Result<(), FocusRejection> {
    require_panels_present(tree, [prev, pid])?;
    set_constraints(tree, prev, fixed(0.0))
}

fn focus_visible_deck_panel(
    tree: &mut LayoutTree,
    prev: PanelId,
    pid: PanelId,
) -> Result<(), FocusRejection> {
    require_panels_present(tree, [prev, pid])?;
    set_constraints(tree, prev, fixed(0.0))?;
    set_constraints(tree, pid, grow(1.0))
}

fn slide_window_to_include(
    tree: &mut LayoutTree,
    sequence: &PanelSequence,
    next_window_start: usize,
    panel_count: usize,
) -> Result<(), FocusRejection> {
    require_panels_present(tree, sequence.iter())?;
    apply_window_constraints(tree, sequence, next_window_start, panel_count)
        .map_err(|_| FocusRejection::StrategyRejected)
}

enum FocusPrep {
    Unchanged,
    Ready,
}

fn prepare_active_panel_focus(
    tree: &mut LayoutTree,
    focused: Option<PanelId>,
    pid: PanelId,
) -> Result<FocusPrep, FocusRejection> {
    match focused {
        Some(prev) if prev == pid => Ok(FocusPrep::Unchanged),
        Some(prev) => {
            hide_prev_active_panel(tree, prev, pid)?;
            Ok(FocusPrep::Ready)
        }
        None => Ok(FocusPrep::Ready),
    }
}

fn prepare_deck_focus(
    tree: &mut LayoutTree,
    sequence: &PanelSequence,
    focused: Option<PanelId>,
    pid: PanelId,
) -> Result<FocusPrep, FocusRejection> {
    match focused.and_then(|panel| sequence.index_of(panel).map(|index| (panel, index))) {
        Some((prev, _)) if prev == pid => Ok(FocusPrep::Unchanged),
        Some((prev, index)) if index > 0 => {
            focus_visible_deck_panel(tree, prev, pid)?;
            Ok(FocusPrep::Ready)
        }
        _ => {
            focus_deck_full(tree, sequence, pid)?;
            Ok(FocusPrep::Ready)
        }
    }
}

fn update_window_focus(
    tree: &mut LayoutTree,
    sequence: &PanelSequence,
    viewport: &mut ViewportState,
    index: usize,
    panel_count: usize,
) -> Result<(), FocusRejection> {
    let ws = viewport.window_start;
    match index >= ws && index < ws + panel_count {
        true => Ok(()),
        false => {
            let len = sequence.len();
            let raw_start = window_start_for_index(index, ws, panel_count);
            let next_window_start = raw_start.min(len.saturating_sub(panel_count));
            slide_window_to_include(tree, sequence, next_window_start, panel_count)?;
            viewport.window_start = next_window_start;
            Ok(())
        }
    }
}

fn focus_active_panel(
    tree: &mut LayoutTree,
    viewport: &mut ViewportState,
    pid: PanelId,
) -> FocusOutcome {
    match prepare_active_panel_focus(tree, viewport.focus, pid) {
        Ok(FocusPrep::Unchanged) => return FocusOutcome::Unchanged,
        Ok(FocusPrep::Ready) => {}
        Err(reason) => return FocusOutcome::Rejected(reason),
    }

    if let Err(reason) = require_panels_present(tree, [pid]) {
        return FocusOutcome::Rejected(reason);
    }

    match set_constraints(tree, pid, grow(1.0)) {
        Ok(()) => {
            viewport.focus = Some(pid);
            FocusOutcome::Applied
        }
        Err(reason) => FocusOutcome::Rejected(reason),
    }
}

fn focus_deck(
    tree: &mut LayoutTree,
    sequence: &PanelSequence,
    viewport: &mut ViewportState,
    pid: PanelId,
) -> FocusOutcome {
    match prepare_deck_focus(tree, sequence, viewport.focus, pid) {
        Ok(FocusPrep::Unchanged) => return FocusOutcome::Unchanged,
        Ok(FocusPrep::Ready) => {}
        Err(reason) => return FocusOutcome::Rejected(reason),
    }
    viewport.focus = Some(pid);
    FocusOutcome::Applied
}

/// Hide all non-target stack panels, show only `pid`.
fn focus_deck_full(
    tree: &mut LayoutTree,
    sequence: &PanelSequence,
    pid: PanelId,
) -> Result<(), FocusRejection> {
    require_panels_present(tree, sequence.iter().skip(1))?;

    for spid in sequence.iter().skip(1) {
        let c = match spid == pid {
            true => grow(1.0),
            false => fixed(0.0),
        };
        set_constraints(tree, spid, c)?;
    }

    Ok(())
}

fn focus_window(
    tree: &mut LayoutTree,
    sequence: &PanelSequence,
    viewport: &mut ViewportState,
    pid: PanelId,
    panel_count: usize,
) -> FocusOutcome {
    let index = match sequence.index_of(pid) {
        Some(i) => i,
        None => return FocusOutcome::Rejected(FocusRejection::PanelNotFound),
    };

    let already_focused = matches!(viewport.focus, Some(prev) if prev == pid);
    if let Err(reason) = update_window_focus(tree, sequence, viewport, index, panel_count) {
        return FocusOutcome::Rejected(reason);
    }

    viewport.focus = Some(pid);
    match already_focused {
        true => FocusOutcome::Unchanged,
        false => FocusOutcome::Applied,
    }
}

fn window_start_for_index(index: usize, current_start: usize, panel_count: usize) -> usize {
    match index < current_start {
        true => index,
        false => index.saturating_sub(panel_count.saturating_sub(1)),
    }
}

/// Set window visibility constraints: panels in [start, start+size) get grow(1.0),
/// all others get fixed(0.0).
pub(super) fn apply_window_constraints(
    tree: &mut LayoutTree,
    sequence: &PanelSequence,
    start: usize,
    panel_count: usize,
) -> Result<(), PaneError> {
    for (i, pid) in sequence.iter().enumerate() {
        let visible = i >= start && i < start + panel_count;
        let constraint = match visible {
            true => grow(1.0),
            false => fixed(0.0),
        };
        tree.set_constraints(pid, constraint)?;
    }
    Ok(())
}
