use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::sequence::PanelSequence;
use crate::tree::LayoutTree;

use super::{ActivePanelVariant, Direction, SlotDef, StrategyKind};

/// Build the initial tree for a strategy and panel kinds.
/// Returns the tree and populates the sequence with panel IDs in order.
pub fn build_initial(
    strategy: &StrategyKind,
    kinds: &[Arc<str>],
    sequence: &mut PanelSequence,
    viewport: &mut crate::viewport::ViewportState,
) -> Result<LayoutTree, PaneError> {
    match kinds.is_empty() {
        true => return Err(PaneError::InvalidTree(crate::error::TreeError::NoKinds)),
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
pub(super) fn populate_sequence_by_kinds(
    tree: &LayoutTree,
    kinds: &[Arc<str>],
    sequence: &mut PanelSequence,
) {
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

pub(super) fn build_deck_tree(
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

pub(super) fn build_binary_split_tree(
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

pub(super) fn build_column_grid_tree(
    kinds: &[Arc<str>],
    columns: usize,
    gap_px: f32,
) -> Result<LayoutTree, PaneError> {
    let layout = crate::preset::Grid::new(columns, kinds.iter().map(Arc::clone))
        .gap(gap_px)
        .build()?;
    Ok(LayoutTree::from(layout))
}

pub(super) fn build_dashboard_tree(
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

/// Build a tree from a strategy and kinds list.
pub(super) fn build_tree_for_strategy(
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
        StrategyKind::Window { size, .. } if *size == 0 => Err(PaneError::InvalidTree(
            crate::error::TreeError::WindowSizeZero,
        )),
        StrategyKind::Window { size, gap } => build_window_tree(kinds, *size, *gap, 0),
        StrategyKind::Slotted {
            slots,
            gap,
            direction,
        } => build_slotted_tree(slots, *gap, *direction),
    }
}
