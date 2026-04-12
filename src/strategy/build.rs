use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::sequence::PanelSequence;
use crate::tree::LayoutTree;

use crate::panel::Axis;

use super::{ActivePanelVariant, CardSpan, GridColumnMode, SlotDef, StrategyKind};

fn validate_strategy(strategy: &StrategyKind) -> Result<(), PaneError> {
    match strategy {
        StrategyKind::Sequence { gap, ratio, .. } => {
            crate::preset::validate_f32_param("gap", *gap)?;
            ratio.map_or(Ok(()), |value| {
                crate::preset::validate_share_param("ratio", value)
            })
        }
        StrategyKind::MasterStack { master_ratio, gap }
        | StrategyKind::Deck { master_ratio, gap }
        | StrategyKind::CenteredMaster { master_ratio, gap } => {
            crate::preset::validate_share_param("master_ratio", *master_ratio)?;
            crate::preset::validate_f32_param("gap", *gap)
        }
        StrategyKind::BinarySplit { ratio, gap, .. } => {
            crate::preset::validate_share_param("ratio", *ratio)?;
            crate::preset::validate_f32_param("gap", *gap)
        }
        StrategyKind::Dashboard { columns, gap, .. } => {
            crate::preset::validate_grid_columns(*columns)?;
            crate::preset::validate_f32_param("gap", *gap)
        }
        StrategyKind::ActivePanel { bar_height, .. } => {
            crate::preset::validate_f32_param("bar_height", *bar_height)
        }
        StrategyKind::Window { panel_count, gap } if *panel_count == 0 => Err(
            PaneError::InvalidTree(crate::error::TreeError::WindowSizeZero),
        ),
        StrategyKind::Window { gap, .. } | StrategyKind::Slotted { gap, .. } => {
            crate::preset::validate_f32_param("gap", *gap)
        }
    }
}

/// Build the initial tree for a strategy and panel kinds.
/// Returns the tree and populates the sequence with panel IDs in order.
pub fn build_initial(
    strategy: &StrategyKind,
    kinds: &[Arc<str>],
    sequence: &mut PanelSequence,
    viewport: &mut crate::viewport::ViewportState,
) -> Result<LayoutTree, PaneError> {
    crate::preset::validate_kinds(kinds)?;

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
///
/// Handles repeated kinds by tracking which occurrence of each kind has been
/// consumed: the first "editor" in `kinds` maps to `panels_by_kind("editor")[0]`,
/// the second to `[1]`, etc.
pub(crate) fn populate_sequence_by_kinds(
    tree: &LayoutTree,
    kinds: &[Arc<str>],
    sequence: &mut PanelSequence,
) {
    let mut occurrence: rustc_hash::FxHashMap<&str, usize> = rustc_hash::FxHashMap::default();
    for kind in kinds {
        let idx = occurrence.entry(kind).or_insert(0);
        if let Some(&pid) = tree.panels_by_kind(kind).get(*idx) {
            sequence.push(pid);
        }
        *idx += 1;
    }
}

// ---------------------------------------------------------------------------
// Tree builders (reuse preset logic via LayoutBuilder)
// ---------------------------------------------------------------------------

fn build_sequence_tree(
    kinds: &[Arc<str>],
    axis: Axis,
    gap_px: f32,
    ratio: Option<f32>,
) -> Result<LayoutTree, PaneError> {
    let mut b = LayoutBuilder::new();
    let add = |ctx: &mut crate::ContainerCtx| match (ratio, kinds.len()) {
        (Some(r), 2) => {
            ctx.panel_with(Arc::clone(&kinds[0]), crate::panel::grow(r));
            ctx.panel_with(Arc::clone(&kinds[1]), crate::panel::grow(1.0 - r));
        }
        _ => {
            for kind in kinds {
                ctx.panel(Arc::clone(kind));
            }
        }
    };
    match axis {
        Axis::Row => b.row_gap(gap_px, add)?,
        Axis::Col => b.col_gap(gap_px, add)?,
    }
    Ok(LayoutTree::from(b.build()?))
}

fn build_master_stack_tree(
    kinds: &[Arc<str>],
    master_ratio: f32,
    gap_px: f32,
) -> Result<LayoutTree, PaneError> {
    match kinds.len() {
        1 => build_sequence_tree(kinds, Axis::Row, 0.0, None),
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

fn build_cards(kinds: &[Arc<str>], spans: &[CardSpan]) -> Box<[(Arc<str>, CardSpan)]> {
    kinds
        .iter()
        .enumerate()
        .map(|(i, k)| {
            (
                Arc::clone(k),
                spans.get(i).copied().unwrap_or(CardSpan::Columns(1)),
            )
        })
        .collect()
}

pub(super) fn build_dashboard_for_mode(
    kinds: &[Arc<str>],
    columns: GridColumnMode,
    gap_px: f32,
    spans: &[CardSpan],
    auto_rows: bool,
) -> Result<LayoutTree, PaneError> {
    let cards = build_cards(kinds, spans);
    let mut preset = crate::preset::Dashboard::new(cards);
    preset = match columns {
        GridColumnMode::Fixed(n) => preset.columns(n),
        GridColumnMode::AutoFill { min_width } => preset.auto_fill(min_width),
        GridColumnMode::AutoFit { min_width } => preset.auto_fit(min_width),
    };
    preset = match auto_rows {
        true => preset.auto_rows(),
        false => preset,
    };
    let layout = preset.gap(gap_px).build()?;
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
        ActivePanelVariant::Tabbed => {
            crate::preset::ActivePanelPreset::new_tabbed(kinds.iter().map(Arc::clone))
                .active(active)
                .bar_height(bar_height)
                .build()?
        }
        ActivePanelVariant::Stacked => {
            crate::preset::ActivePanelPreset::new_stacked(kinds.iter().map(Arc::clone))
                .active(active)
                .bar_height(bar_height)
                .build()?
        }
    };
    Ok(LayoutTree::from(layout))
}

fn build_window_tree(
    kinds: &[Arc<str>],
    gap_px: f32,
    window_start: usize,
) -> Result<LayoutTree, PaneError> {
    let layout = crate::preset::Scrollable::new(kinds.iter().map(Arc::clone))
        .active(window_start)
        .gap(gap_px)
        .build()?;
    Ok(LayoutTree::from(layout))
}

fn build_slotted_tree(slots: &[SlotDef], gap_px: f32, axis: Axis) -> Result<LayoutTree, PaneError> {
    let mut b = LayoutBuilder::new();
    let add = |ctx: &mut crate::ContainerCtx| {
        for slot in slots {
            ctx.panel_with(Arc::clone(&slot.kind), slot.constraints);
        }
    };
    match axis {
        Axis::Row => b.row_gap(gap_px, add)?,
        Axis::Col => b.col_gap(gap_px, add)?,
    }
    Ok(LayoutTree::from(b.build()?))
}

/// Build a tree from a strategy and kinds list.
pub(crate) fn build_tree_for_strategy(
    strategy: &StrategyKind,
    kinds: &[Arc<str>],
) -> Result<LayoutTree, PaneError> {
    validate_strategy(strategy)?;

    match strategy {
        StrategyKind::Sequence { axis, gap, ratio } => {
            build_sequence_tree(kinds, *axis, *gap, *ratio)
        }
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
        StrategyKind::Dashboard {
            columns,
            gap,
            spans,
            auto_rows,
        } => build_dashboard_for_mode(kinds, *columns, *gap, spans, *auto_rows),
        StrategyKind::ActivePanel {
            variant,
            bar_height,
        } => build_active_panel_tree(kinds, *variant, *bar_height, 0),
        StrategyKind::Window { gap, .. } => build_window_tree(kinds, *gap, 0),
        StrategyKind::Slotted { slots, gap, axis } => build_slotted_tree(slots, *gap, *axis),
    }
}
