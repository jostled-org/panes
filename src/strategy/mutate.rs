use std::sync::Arc;

use crate::error::{MutationError, PaneError, TreeError};
use crate::node::PanelId;
use crate::panel::fixed;
use crate::sequence::PanelSequence;
use crate::tree::LayoutTree;
use crate::viewport::ViewportState;

use super::StrategyKind;
use super::build::{build_tree_for_strategy, populate_sequence_by_kinds};
use super::focus::try_apply_focus;

// ---------------------------------------------------------------------------
// apply_add
// ---------------------------------------------------------------------------

/// Add a panel at a specific sequence index.
///
/// Slotted: restores a collapsed slot (index ignored).
/// All others: inserts the kind at `index`, rebuilds the tree via
/// `build_tree_for_strategy`, and applies focus to the new panel.
pub fn apply_add(
    strategy: &StrategyKind,
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    kind: Arc<str>,
    index: usize,
) -> Result<PanelId, PaneError> {
    match strategy {
        StrategyKind::Slotted { .. } => add_slotted(tree, viewport),
        _ => add_via_rebuild(strategy, tree, sequence, viewport, kind, index),
    }
}

fn add_via_rebuild(
    strategy: &StrategyKind,
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    kind: Arc<str>,
    index: usize,
) -> Result<PanelId, PaneError> {
    let mut kinds = collect_kinds_from_sequence(tree, sequence);
    let clamped = index.min(kinds.len());
    kinds.insert(clamped, kind);
    rebuild_tree_and_sequence(tree, sequence, &kinds, |kinds| {
        build_tree_for_strategy(strategy, kinds)
    })?;
    let new_pid = sequence
        .get(clamped)
        .ok_or(PaneError::InvalidTree(TreeError::EmptyAfterRebuild))?;
    // Rebuild hardcodes active=0. Set focus to match so try_apply_focus
    // can properly transition away from it.
    viewport.focus = sequence.get(0);
    try_apply_focus(strategy, tree, sequence, viewport, new_pid);
    Ok(new_pid)
}

fn add_slotted(tree: &mut LayoutTree, viewport: &mut ViewportState) -> Result<PanelId, PaneError> {
    let pid = viewport
        .collapsed
        .iter()
        .next()
        .copied()
        .ok_or(PaneError::InvalidMutation(MutationError::NoCollapsedSlots))?;
    let saved = viewport
        .saved_constraints
        .remove(&pid)
        .ok_or(PaneError::InvalidMutation(
            MutationError::SlotNoSavedConstraints,
        ))?;
    tree.set_constraints(pid, saved)?;
    viewport.collapsed.remove(&pid);
    viewport.focus = Some(pid);
    Ok(pid)
}

// ---------------------------------------------------------------------------
// apply_remove
// ---------------------------------------------------------------------------

/// Remove a panel. Returns the new focus panel.
pub fn apply_remove(
    strategy: &StrategyKind,
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    pid: PanelId,
) -> Result<Option<PanelId>, PaneError> {
    match strategy {
        StrategyKind::Slotted { .. } => remove_slotted(tree, sequence, viewport, pid),
        _ => remove_via_rebuild(strategy, tree, sequence, viewport, pid),
    }
}

fn remove_slotted(
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    pid: PanelId,
) -> Result<Option<PanelId>, PaneError> {
    let current = tree.panel_constraints(pid)?;
    viewport.saved_constraints.insert(pid, current);
    tree.set_constraints(pid, fixed(0.0))?;
    viewport.collapsed.insert(pid);
    // Panel may not be in sequence (e.g. decoration panels), fall back to index 0.
    let removed_idx = sequence.remove(pid).unwrap_or(0);
    let new_focus = sequence.neighbor_after_removal(removed_idx);
    viewport.focus = new_focus;
    Ok(new_focus)
}

fn remove_via_rebuild(
    strategy: &StrategyKind,
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    pid: PanelId,
) -> Result<Option<PanelId>, PaneError> {
    // Panel may not be in sequence (e.g. decoration panels), fall back to index 0.
    let removed_idx = sequence.remove(pid).unwrap_or(0);
    match sequence.is_empty() {
        true => {
            viewport.focus = None;
            return Ok(None);
        }
        false => {}
    }
    let kinds = collect_kinds_from_sequence(tree, sequence);
    rebuild_tree_and_sequence(tree, sequence, &kinds, |kinds| {
        build_tree_for_strategy(strategy, kinds)
    })?;
    let focus_idx = removed_idx.min(sequence.len().saturating_sub(1));
    let new_focus = sequence.get(focus_idx);
    // Rebuild hardcodes active=0. Set focus to match so try_apply_focus
    // can properly transition away from it.
    viewport.focus = sequence.get(0);
    if let Some(pid) = new_focus {
        try_apply_focus(strategy, tree, sequence, viewport, pid);
    }
    Ok(new_focus)
}

// ---------------------------------------------------------------------------
// apply_move
// ---------------------------------------------------------------------------

/// Move a panel to a new sequence index.
pub fn apply_move(
    strategy: &StrategyKind,
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    pid: PanelId,
    new_index: usize,
) -> Result<PanelId, PaneError> {
    match strategy.supports_move() {
        false => {
            return Err(PaneError::InvalidMutation(MutationError::MoveNotSupported));
        }
        true => {}
    }

    match new_index >= sequence.len() {
        true => return Err(PaneError::SequenceOutOfBounds(new_index, sequence.len())),
        false => {}
    }

    sequence
        .move_to(pid, new_index)
        .ok_or(PaneError::PanelNotFound(pid))?;

    rebuild_from_sequence(strategy, tree, sequence)?;

    let moved_pid = sequence
        .get(new_index)
        .ok_or_else(|| PaneError::SequenceOutOfBounds(new_index, sequence.len()))?;
    // Rebuild hardcodes active=0. Set focus to match so try_apply_focus
    // can properly transition away from it.
    viewport.focus = sequence.get(0);
    try_apply_focus(strategy, tree, sequence, viewport, moved_pid);
    Ok(moved_pid)
}

fn rebuild_from_sequence(
    strategy: &StrategyKind,
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
) -> Result<(), PaneError> {
    let kinds = collect_kinds_from_sequence(tree, sequence);
    match strategy {
        StrategyKind::Slotted { .. } => return Ok(()),
        _ => {}
    }
    rebuild_tree_and_sequence(tree, sequence, &kinds, |kinds| {
        build_tree_for_strategy(strategy, kinds)
    })
}

/// Rebuild the tree from a kinds list and repopulate the sequence.
fn rebuild_tree_and_sequence(
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    kinds: &[Arc<str>],
    builder: impl FnOnce(&[Arc<str>]) -> Result<LayoutTree, PaneError>,
) -> Result<(), PaneError> {
    *tree = builder(kinds)?;
    let mut new_seq = PanelSequence::default();
    populate_sequence_by_kinds(tree, kinds, &mut new_seq);
    *sequence = new_seq;
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Collect panel kinds from the sequence, preserving order.
/// Skips panels missing from the tree (only possible via `tree_mut()` corruption).
pub(crate) fn collect_kinds_from_sequence(
    tree: &LayoutTree,
    sequence: &PanelSequence,
) -> Vec<Arc<str>> {
    sequence
        .iter()
        .filter_map(|pid| tree.panel_kind_arc(pid).ok())
        .collect()
}
