use std::sync::Arc;

use crate::error::{MutationError, PaneError, TreeError};
use crate::node::PanelId;
use crate::panel::{fixed, grow};
use crate::sequence::PanelSequence;
use crate::tree::LayoutTree;
use crate::viewport::ViewportState;

use super::StrategyKind;
use super::build::{
    build_binary_split_tree, build_column_grid_tree, build_dashboard_tree, build_tree_for_strategy,
    populate_sequence_by_kinds,
};
use super::focus::apply_window_constraints;

// ---------------------------------------------------------------------------
// apply_add
// ---------------------------------------------------------------------------

/// Add a panel to an existing layout.
pub fn apply_add(
    strategy: &StrategyKind,
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    kind: Arc<str>,
) -> Result<PanelId, PaneError> {
    match strategy {
        StrategyKind::Sequence { .. } => add_append(tree, sequence, viewport, kind),
        StrategyKind::MasterStack { .. } => add_to_stack(tree, sequence, viewport, kind),
        StrategyKind::Deck { .. } => add_to_deck(tree, sequence, viewport, kind),
        StrategyKind::CenteredMaster { .. } => add_to_centered(tree, sequence, viewport, kind),
        StrategyKind::BinarySplit { spiral, ratio, gap } => {
            add_via_rebuild(tree, sequence, viewport, kind, |kinds| {
                build_binary_split_tree(kinds, *spiral, *ratio, *gap)
            })
        }
        StrategyKind::ColumnGrid { columns, gap } => {
            add_via_rebuild(tree, sequence, viewport, kind, |kinds| {
                build_column_grid_tree(kinds, *columns, *gap)
            })
        }
        StrategyKind::Dashboard {
            columns,
            gap,
            spans,
        } => add_via_rebuild(tree, sequence, viewport, kind, |kinds| {
            build_dashboard_tree(kinds, *columns, *gap, spans)
        }),
        StrategyKind::ActivePanel { .. } => add_active_panel(tree, sequence, viewport, kind),
        StrategyKind::Window { size, .. } => add_window(tree, sequence, viewport, kind, *size),
        StrategyKind::Slotted { .. } => add_slotted(tree, viewport),
    }
}

fn add_append(
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    kind: Arc<str>,
) -> Result<PanelId, PaneError> {
    let pid = append_to_root(tree, kind)?;
    sequence.push(pid);
    viewport.focus = Some(pid);
    Ok(pid)
}

fn add_to_stack(
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    kind: Arc<str>,
) -> Result<PanelId, PaneError> {
    let pid = append_to_stack_container(tree, kind)?;
    sequence.push(pid);
    viewport.focus = Some(pid);
    Ok(pid)
}

fn add_to_deck(
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    kind: Arc<str>,
) -> Result<PanelId, PaneError> {
    hide_prev_stack_panel(tree, sequence, viewport)?;
    let pid = append_to_stack_container(tree, kind)?;
    sequence.push(pid);
    viewport.focus = Some(pid);
    Ok(pid)
}

fn hide_prev_stack_panel(
    tree: &mut LayoutTree,
    sequence: &PanelSequence,
    viewport: &ViewportState,
) -> Result<(), PaneError> {
    let (prev, idx) = match viewport.focus {
        Some(prev) => (prev, sequence.index_of(prev)),
        None => return Ok(()),
    };
    match idx {
        Some(i) if i > 0 => tree.set_constraints(prev, fixed(0.0)),
        _ => Ok(()),
    }
}

fn add_to_centered(
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    kind: Arc<str>,
) -> Result<PanelId, PaneError> {
    let pid = append_to_shorter_side(tree, kind)?;
    sequence.push(pid);
    viewport.focus = Some(pid);
    Ok(pid)
}

fn add_via_rebuild(
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    kind: Arc<str>,
    builder: impl FnOnce(&[Arc<str>]) -> Result<LayoutTree, PaneError>,
) -> Result<PanelId, PaneError> {
    let mut kinds = collect_kinds_from_sequence(tree, sequence)?;
    kinds.push(kind);
    rebuild_tree_and_sequence(tree, sequence, &kinds, builder)?;
    let new_pid = sequence
        .get(sequence.len() - 1)
        .ok_or(PaneError::InvalidTree(TreeError::EmptyAfterRebuild))?;
    viewport.focus = Some(new_pid);
    Ok(new_pid)
}

fn add_active_panel(
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    kind: Arc<str>,
) -> Result<PanelId, PaneError> {
    if let Some(prev) = viewport.focus {
        tree.set_constraints(prev, fixed(0.0))?;
    }
    let pid = append_to_root(tree, kind)?;
    sequence.push(pid);
    viewport.focus = Some(pid);
    Ok(pid)
}

fn add_window(
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    kind: Arc<str>,
    size: usize,
) -> Result<PanelId, PaneError> {
    let pid = append_to_root(tree, kind)?;
    sequence.push(pid);
    let new_start = sequence.len().saturating_sub(size);
    viewport.window_start = new_start;
    apply_window_constraints(tree, sequence, new_start, size)?;
    viewport.focus = Some(pid);
    Ok(pid)
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

/// Append a grow(1.0) panel as the last child of the root container.
fn append_to_root(tree: &mut LayoutTree, kind: Arc<str>) -> Result<PanelId, PaneError> {
    let (pid, nid) = tree.add_panel(kind, grow(1.0))?;
    let root = tree
        .root()
        .ok_or(PaneError::InvalidTree(TreeError::NoRoot))?;
    let len = tree.children(root)?.len();
    tree.insert_child_at(root, len, nid)?;
    Ok(pid)
}

/// Append a grow(1.0) panel to the stack container (2nd child of root row).
fn append_to_stack_container(tree: &mut LayoutTree, kind: Arc<str>) -> Result<PanelId, PaneError> {
    let (pid, nid) = tree.add_panel(kind, grow(1.0))?;
    let root = tree
        .root()
        .ok_or(PaneError::InvalidTree(TreeError::NoRoot))?;
    let root_children = tree.children(root)?;
    let (container, container_len) = match root_children.len() >= 2 {
        true => {
            let stack = root_children[1];
            (stack, tree.children(stack)?.len())
        }
        false => (root, root_children.len()),
    };
    tree.insert_child_at(container, container_len, nid)?;
    Ok(pid)
}

/// Append to the shorter side column in CenteredMaster.
/// Root structure: [left_col, master, right_col]
fn append_to_shorter_side(tree: &mut LayoutTree, kind: Arc<str>) -> Result<PanelId, PaneError> {
    let (pid, nid) = tree.add_panel(kind, grow(1.0))?;
    let root = tree
        .root()
        .ok_or(PaneError::InvalidTree(TreeError::NoRoot))?;
    let root_children = tree.children(root)?.to_vec();

    let (target, target_len) = match root_children.len() >= 3 {
        true => {
            let (left, right) = (root_children[0], root_children[2]);
            let (lc, rc) = (tree.children(left)?.len(), tree.children(right)?.len());
            let shorter = shorter_side(left, right, lc, rc);
            (shorter, tree.children(shorter)?.len())
        }
        false => (root, root_children.len()),
    };
    tree.insert_child_at(target, target_len, nid)?;
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
        StrategyKind::BinarySplit { spiral, ratio, gap } => {
            remove_via_rebuild(tree, sequence, viewport, pid, |kinds| {
                build_binary_split_tree(kinds, *spiral, *ratio, *gap)
            })
        }
        StrategyKind::ColumnGrid { columns, gap } => {
            remove_via_rebuild(tree, sequence, viewport, pid, |kinds| {
                build_column_grid_tree(kinds, *columns, *gap)
            })
        }
        StrategyKind::Dashboard {
            columns,
            gap,
            spans,
        } => remove_via_rebuild(tree, sequence, viewport, pid, |kinds| {
            build_dashboard_tree(kinds, *columns, *gap, spans)
        }),
        _ => remove_incremental(strategy, tree, sequence, viewport, pid),
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
    let removed_idx = sequence.remove(pid).unwrap_or(0);
    let new_focus = sequence.neighbor_after_removal(removed_idx);
    viewport.focus = new_focus;
    Ok(new_focus)
}

fn remove_via_rebuild(
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    pid: PanelId,
    builder: impl FnOnce(&[Arc<str>]) -> Result<LayoutTree, PaneError>,
) -> Result<Option<PanelId>, PaneError> {
    let removed_idx = sequence.remove(pid).unwrap_or(0);
    match sequence.is_empty() {
        true => {
            viewport.focus = None;
            return Ok(None);
        }
        false => {}
    }
    let kinds = collect_kinds_from_sequence(tree, sequence)?;
    rebuild_tree_and_sequence(tree, sequence, &kinds, builder)?;
    let focus_idx = removed_idx.min(sequence.len().saturating_sub(1));
    viewport.focus = sequence.get(focus_idx);
    Ok(viewport.focus)
}

fn remove_incremental(
    strategy: &StrategyKind,
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
    pid: PanelId,
) -> Result<Option<PanelId>, PaneError> {
    let removed_idx = sequence.remove(pid).unwrap_or(0);
    tree.remove_panel(pid)?;
    let new_focus = sequence.neighbor_after_removal(removed_idx);

    match (strategy, new_focus) {
        (StrategyKind::ActivePanel { .. }, Some(focus_pid)) => {
            tree.set_constraints(focus_pid, grow(1.0))?;
        }
        (StrategyKind::Window { size, .. }, _) if !sequence.is_empty() => {
            let ws = viewport
                .window_start
                .min(sequence.len().saturating_sub(*size));
            viewport.window_start = ws;
            apply_window_constraints(tree, sequence, ws, *size)?;
        }
        _ => {}
    }

    viewport.focus = new_focus;
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
    viewport.focus = Some(moved_pid);
    Ok(moved_pid)
}

fn rebuild_from_sequence(
    strategy: &StrategyKind,
    tree: &mut LayoutTree,
    sequence: &mut PanelSequence,
) -> Result<(), PaneError> {
    let kinds = collect_kinds_from_sequence(tree, sequence)?;
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

fn shorter_side(
    left: crate::node::NodeId,
    right: crate::node::NodeId,
    left_count: usize,
    right_count: usize,
) -> crate::node::NodeId {
    match left_count <= right_count {
        true => left,
        false => right,
    }
}

/// Collect panel kinds from the sequence, preserving order.
fn collect_kinds_from_sequence(
    tree: &LayoutTree,
    sequence: &PanelSequence,
) -> Result<Vec<Arc<str>>, PaneError> {
    sequence
        .iter()
        .map(|pid| tree.panel_kind_arc(pid))
        .collect()
}
