use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::{MutationError, PaneError, TreeError};
use crate::node::PanelId;
use crate::panel::{Constraints, fixed, grow};
use crate::sequence::PanelSequence;
use crate::tree::LayoutTree;
use crate::viewport::ViewportState;

/// Direction for linear layouts (split, columns).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Left-to-right.
    Horizontal,
    /// Top-to-bottom.
    Vertical,
}

/// Sub-variant for single-visible-panel layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanelVariant {
    /// Full-screen single panel.
    Monocle,
    /// Tab bar above content panels.
    Tabbed,
    /// Title bars stacked vertically above content.
    Stacked,
}

/// Definition of a named slot with fixed or grow constraints.
#[derive(Debug, Clone)]
pub struct SlotDef {
    /// The panel kind occupying this slot.
    pub kind: Arc<str>,
    /// Constraints for this slot when visible.
    pub constraints: Constraints,
}

/// Behavioral strategy for a layout, determining how add/remove/move/focus
/// mutations are applied to the tree.
#[derive(Debug, Clone)]
pub enum StrategyKind {
    /// Linear sequence of equal panels (split, columns).
    Sequence {
        /// Layout direction.
        direction: Direction,
        /// Gap between panels.
        gap: f32,
    },

    /// One master panel with a vertical stack (master-stack).
    MasterStack {
        /// Master panel's share of the viewport (0.0–1.0).
        master_ratio: f32,
        /// Gap between panels.
        gap: f32,
    },

    /// Master panel with a deck of one-at-a-time stack panels (deck).
    Deck {
        /// Master panel's share of the viewport (0.0–1.0).
        master_ratio: f32,
        /// Gap between panels.
        gap: f32,
    },

    /// Master panel centered between two side stacks (centered-master).
    CenteredMaster {
        /// Master panel's share of the viewport (0.0–1.0).
        master_ratio: f32,
        /// Gap between panels.
        gap: f32,
    },

    /// Recursive binary split (dwindle, spiral).
    BinarySplit {
        /// Whether child order reverses on even-depth levels (spiral).
        spiral: bool,
        /// Split ratio at each level.
        ratio: f32,
        /// Gap between panels.
        gap: f32,
    },

    /// Uniform grid of panels (grid).
    ColumnGrid {
        /// Number of columns.
        columns: usize,
        /// Gap between panels.
        gap: f32,
    },

    /// CSS-grid dashboard with per-card column spans (dashboard).
    Dashboard {
        /// Number of columns.
        columns: usize,
        /// Gap between panels.
        gap: f32,
        /// Column span per card, in order.
        spans: Arc<[usize]>,
    },

    /// Only one panel visible at a time (monocle, tabbed, stacked).
    ActivePanel {
        /// Which sub-variant of active-panel layout.
        variant: ActivePanelVariant,
        /// Height of the tab bar (tabbed) or title bars (stacked).
        /// Ignored for monocle.
        bar_height: f32,
    },

    /// Scrollable window showing N adjacent panels (scrollable/NIRI).
    Window {
        /// How many panels the window shows at once.
        size: usize,
        /// Gap between visible panels.
        gap: f32,
    },

    /// Fixed-slot layout with named positions (sidebar, holy-grail).
    Slotted {
        /// Slot definitions in layout order.
        slots: Arc<[SlotDef]>,
        /// Gap between slots.
        gap: f32,
        /// Direction of the outer container.
        direction: Direction,
    },
}

impl StrategyKind {
    /// Gap value for this strategy.
    pub fn gap(&self) -> f32 {
        match self {
            Self::Sequence { gap, .. }
            | Self::MasterStack { gap, .. }
            | Self::Deck { gap, .. }
            | Self::CenteredMaster { gap, .. }
            | Self::BinarySplit { gap, .. }
            | Self::ColumnGrid { gap, .. }
            | Self::Dashboard { gap, .. }
            | Self::Window { gap, .. }
            | Self::Slotted { gap, .. } => *gap,
            Self::ActivePanel { .. } => 0.0,
        }
    }

    /// Whether this strategy supports the move operation.
    pub fn supports_move(&self) -> bool {
        !matches!(self, Self::Slotted { .. })
    }
}

// ---------------------------------------------------------------------------
// build_initial
// ---------------------------------------------------------------------------

/// Build the initial tree for a strategy and panel kinds.
/// Returns the tree and populates the sequence with panel IDs in order.
pub fn build_initial(
    strategy: &StrategyKind,
    kinds: &[Arc<str>],
    sequence: &mut PanelSequence,
    viewport: &mut ViewportState,
) -> Result<LayoutTree, PaneError> {
    match kinds.is_empty() {
        true => return Err(PaneError::InvalidTree(TreeError::NoKinds)),
        false => {}
    }

    let tree = build_tree_for_strategy(strategy, kinds)?;

    populate_sequence_by_kinds(&tree, kinds, sequence);
    viewport.focus = sequence.get(0);

    Ok(tree)
}

/// Populate the sequence from the input kinds list, looking up each kind's
/// panel ID in the tree. This preserves the caller's logical order regardless
/// of tree topology (e.g. spiral/dwindle nest panels at varying depths).
/// Decorative panels (tabs, titles) that the preset builder generates but
/// which aren't in the input kinds are excluded.
fn populate_sequence_by_kinds(tree: &LayoutTree, kinds: &[Arc<str>], sequence: &mut PanelSequence) {
    for kind in kinds {
        for &pid in tree.panels_by_kind(kind) {
            sequence.push(pid);
        }
    }
}

// ---------------------------------------------------------------------------
// Tree builders (reuse preset logic via LayoutBuilder)
// ---------------------------------------------------------------------------

fn build_sequence_tree(
    kinds: &[Arc<str>],
    direction: Direction,
    gap_px: f32,
) -> Result<LayoutTree, PaneError> {
    let mut b = LayoutBuilder::new();
    let add = |ctx: &mut crate::ContainerCtx| {
        for kind in kinds {
            ctx.panel(Arc::clone(kind));
        }
    };
    match direction {
        Direction::Horizontal => b.row_gap(gap_px, add)?,
        Direction::Vertical => b.col_gap(gap_px, add)?,
    }
    Ok(LayoutTree::from(b.build()?))
}

fn build_master_stack_tree(
    kinds: &[Arc<str>],
    master_ratio: f32,
    gap_px: f32,
) -> Result<LayoutTree, PaneError> {
    match kinds.len() {
        1 => build_sequence_tree(kinds, Direction::Horizontal, 0.0),
        _ => {
            let layout = crate::preset::MasterStack::new(kinds.iter().map(Arc::clone))
                .master_ratio(master_ratio)
                .gap(gap_px)
                .build()?;
            Ok(LayoutTree::from(layout))
        }
    }
}

fn build_deck_tree(
    kinds: &[Arc<str>],
    master_ratio: f32,
    gap_px: f32,
    active: usize,
) -> Result<LayoutTree, PaneError> {
    let layout = crate::preset::Deck::new(kinds.iter().map(Arc::clone))
        .master_ratio(master_ratio)
        .gap(gap_px)
        .active(active)
        .build()?;
    Ok(LayoutTree::from(layout))
}

fn build_centered_master_tree(
    kinds: &[Arc<str>],
    master_ratio: f32,
    gap_px: f32,
) -> Result<LayoutTree, PaneError> {
    let layout = crate::preset::CenteredMaster::new(kinds.iter().map(Arc::clone))
        .master_ratio(master_ratio)
        .gap(gap_px)
        .build()?;
    Ok(LayoutTree::from(layout))
}

fn build_binary_split_tree(
    kinds: &[Arc<str>],
    spiral: bool,
    ratio: f32,
    gap_px: f32,
) -> Result<LayoutTree, PaneError> {
    let layout = match spiral {
        true => crate::preset::Spiral::new(kinds.iter().map(Arc::clone))
            .ratio(ratio)
            .gap(gap_px)
            .build()?,
        false => crate::preset::Dwindle::new(kinds.iter().map(Arc::clone))
            .ratio(ratio)
            .gap(gap_px)
            .build()?,
    };
    Ok(LayoutTree::from(layout))
}

fn build_column_grid_tree(
    kinds: &[Arc<str>],
    columns: usize,
    gap_px: f32,
) -> Result<LayoutTree, PaneError> {
    let layout = crate::preset::Grid::new(columns, kinds.iter().map(Arc::clone))
        .gap(gap_px)
        .build()?;
    Ok(LayoutTree::from(layout))
}

fn build_dashboard_tree(
    kinds: &[Arc<str>],
    columns: usize,
    gap_px: f32,
    spans: &[usize],
) -> Result<LayoutTree, PaneError> {
    let cards: Vec<(Arc<str>, usize)> = kinds
        .iter()
        .enumerate()
        .map(|(i, k)| (Arc::clone(k), spans.get(i).copied().unwrap_or(1)))
        .collect();
    let layout = crate::preset::Dashboard::new(cards)
        .columns(columns)
        .gap(gap_px)
        .build()?;
    Ok(LayoutTree::from(layout))
}

fn build_active_panel_tree(
    kinds: &[Arc<str>],
    variant: ActivePanelVariant,
    bar_height: f32,
    active: usize,
) -> Result<LayoutTree, PaneError> {
    let layout = match variant {
        ActivePanelVariant::Monocle => crate::preset::Monocle::new(kinds.iter().map(Arc::clone))
            .active(active)
            .build()?,
        ActivePanelVariant::Tabbed => crate::preset::Tabbed::new(kinds.iter().map(Arc::clone))
            .active(active)
            .tab_height(bar_height)
            .build()?,
        ActivePanelVariant::Stacked => crate::preset::Stacked::new(kinds.iter().map(Arc::clone))
            .active(active)
            .title_height(bar_height)
            .build()?,
    };
    Ok(LayoutTree::from(layout))
}

fn build_window_tree(
    kinds: &[Arc<str>],
    _size: usize,
    gap_px: f32,
    window_start: usize,
) -> Result<LayoutTree, PaneError> {
    let layout = crate::preset::Scrollable::new(kinds.iter().map(Arc::clone))
        .active(window_start)
        .gap(gap_px)
        .build()?;
    Ok(LayoutTree::from(layout))
}

fn build_slotted_tree(
    slots: &[SlotDef],
    gap_px: f32,
    direction: Direction,
) -> Result<LayoutTree, PaneError> {
    let mut b = LayoutBuilder::new();
    let add = |ctx: &mut crate::ContainerCtx| {
        for slot in slots {
            ctx.panel_with(Arc::clone(&slot.kind), slot.constraints);
        }
    };
    match direction {
        Direction::Horizontal => b.row_gap(gap_px, add)?,
        Direction::Vertical => b.col_gap(gap_px, add)?,
    }
    Ok(LayoutTree::from(b.build()?))
}

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
    // Hide previous stack panel if focused
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

/// Build a tree from a strategy and kinds list.
fn build_tree_for_strategy(
    strategy: &StrategyKind,
    kinds: &[Arc<str>],
) -> Result<LayoutTree, PaneError> {
    match strategy {
        StrategyKind::Sequence { direction, gap } => build_sequence_tree(kinds, *direction, *gap),
        StrategyKind::MasterStack { master_ratio, gap } => {
            build_master_stack_tree(kinds, *master_ratio, *gap)
        }
        StrategyKind::Deck { master_ratio, gap } => build_deck_tree(kinds, *master_ratio, *gap, 0),
        StrategyKind::CenteredMaster { master_ratio, gap } => {
            build_centered_master_tree(kinds, *master_ratio, *gap)
        }
        StrategyKind::BinarySplit { spiral, ratio, gap } => {
            build_binary_split_tree(kinds, *spiral, *ratio, *gap)
        }
        StrategyKind::ColumnGrid { columns, gap } => build_column_grid_tree(kinds, *columns, *gap),
        StrategyKind::Dashboard {
            columns,
            gap,
            spans,
        } => build_dashboard_tree(kinds, *columns, *gap, spans),
        StrategyKind::ActivePanel {
            variant,
            bar_height,
        } => build_active_panel_tree(kinds, *variant, *bar_height, 0),
        StrategyKind::Window { size, .. } if *size == 0 => {
            Err(PaneError::InvalidTree(TreeError::WindowSizeZero))
        }
        StrategyKind::Window { size, gap } => build_window_tree(kinds, *size, *gap, 0),
        StrategyKind::Slotted {
            slots,
            gap,
            direction,
        } => build_slotted_tree(slots, *gap, *direction),
    }
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
// apply_focus
// ---------------------------------------------------------------------------

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

fn window_start_for_index(index: usize, current_start: usize, size: usize) -> usize {
    match index < current_start {
        true => index,
        false => index.saturating_sub(size - 1),
    }
}

/// Set window visibility constraints: panels in [start, start+size) get grow(1.0),
/// all others get fixed(0.0).
fn apply_window_constraints(
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
