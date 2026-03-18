use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{ActivePanelVariant, Direction, Placement, StrategyKind};

fn kinds(n: usize) -> Vec<Arc<str>> {
    (0..n).map(|i| Arc::from(format!("p{i}"))).collect()
}

fn sequence_kinds(rt: &LayoutRuntime) -> Vec<String> {
    (0..rt.sequence().len())
        .map(|i| {
            let pid = rt.sequence().get(i).unwrap();
            rt.tree().panel_kind(pid).unwrap().to_owned()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Sequence: positional insertion
// ---------------------------------------------------------------------------

fn sequence_runtime(n: usize) -> LayoutRuntime {
    LayoutRuntime::from_strategy(
        StrategyKind::Sequence {
            direction: Direction::Horizontal,
            gap: 0.0,
            ratio: None,
        },
        &kinds(n),
    )
    .unwrap()
}

#[test]
fn sequence_add_panel_inserts_after_focused() {
    let mut rt = sequence_runtime(3);
    // Focus p1
    let p1 = rt.sequence().get(1).unwrap();
    rt.focus(p1);
    rt.add_panel(Arc::from("new")).unwrap();
    // "new" should be at index 2 (after p1 at index 1)
    assert_eq!(sequence_kinds(&rt), ["p0", "p1", "new", "p2"]);
}

#[test]
fn sequence_add_panel_with_before() {
    let mut rt = sequence_runtime(3);
    let p1 = rt.sequence().get(1).unwrap();
    rt.focus(p1);
    rt.add_panel_with(Arc::from("new"), Placement::Before)
        .unwrap();
    assert_eq!(sequence_kinds(&rt), ["p0", "new", "p1", "p2"]);
}

#[test]
fn sequence_add_panel_with_end() {
    let mut rt = sequence_runtime(3);
    let p1 = rt.sequence().get(1).unwrap();
    rt.focus(p1);
    rt.add_panel_with(Arc::from("new"), Placement::End).unwrap();
    assert_eq!(sequence_kinds(&rt), ["p0", "p1", "p2", "new"]);
}

#[test]
fn sequence_add_panel_with_after() {
    let mut rt = sequence_runtime(3);
    let p1 = rt.sequence().get(1).unwrap();
    rt.focus(p1);
    rt.add_panel_with(Arc::from("new"), Placement::After)
        .unwrap();
    assert_eq!(sequence_kinds(&rt), ["p0", "p1", "new", "p2"]);
}

// ---------------------------------------------------------------------------
// MasterStack: positional insertion and rebuild
// ---------------------------------------------------------------------------

fn master_stack_runtime(n: usize) -> LayoutRuntime {
    LayoutRuntime::from_strategy(
        StrategyKind::MasterStack {
            master_ratio: 0.5,
            gap: 0.0,
        },
        &kinds(n),
    )
    .unwrap()
}

#[test]
fn master_stack_add_panel_rebuilds_correctly() {
    let mut rt = master_stack_runtime(3);
    rt.add_panel(Arc::from("new")).unwrap();
    rt.tree().validate().unwrap();
    assert_eq!(rt.sequence().len(), 4);
}

#[test]
fn master_stack_swap_after_add() {
    let mut rt = master_stack_runtime(3);
    rt.add_panel(Arc::from("new")).unwrap();
    rt.swap_next();
    rt.tree().validate().unwrap();
}

// ---------------------------------------------------------------------------
// Tabbed: add_panel creates decorative _tab panels
// ---------------------------------------------------------------------------

#[test]
fn tabbed_add_panel_creates_tab_decoration() {
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::ActivePanel {
            variant: ActivePanelVariant::Tabbed,
            bar_height: 30.0,
        },
        &[Arc::from("editor"), Arc::from("terminal")],
    )
    .unwrap();

    let new_pid = rt.add_panel(Arc::from("logs")).unwrap();
    assert_eq!(rt.focused(), Some(new_pid));
    assert_eq!(rt.sequence().len(), 3);
    rt.tree().validate().unwrap();

    // The tree should contain _tab decoration panels
    let has_tab = rt
        .tree()
        .panels_by_kind("logs_tab")
        .first()
        .copied()
        .is_some();
    assert!(has_tab, "tabbed add_panel should create _tab decoration");
}

#[test]
fn stacked_add_panel_creates_title_decoration() {
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::ActivePanel {
            variant: ActivePanelVariant::Stacked,
            bar_height: 30.0,
        },
        &[Arc::from("editor"), Arc::from("terminal")],
    )
    .unwrap();

    rt.add_panel(Arc::from("logs")).unwrap();
    rt.tree().validate().unwrap();

    let has_title = rt
        .tree()
        .panels_by_kind("logs_title")
        .first()
        .copied()
        .is_some();
    assert!(
        has_title,
        "stacked add_panel should create _title decoration"
    );
}

// ---------------------------------------------------------------------------
// Deck/ActivePanel: move preserves visibility (side fix)
// ---------------------------------------------------------------------------

#[test]
fn deck_move_preserves_visibility() {
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::Deck {
            master_ratio: 0.5,
            gap: 0.0,
        },
        &kinds(4),
    )
    .unwrap();

    // Focus p2 (stack panel)
    let p2 = rt.sequence().get(2).unwrap();
    rt.focus(p2);

    // Move p2 to index 3
    rt.move_panel(p2, 3).unwrap();

    let focused = rt.focused().unwrap();
    let c = rt.tree().panel_constraints(focused).unwrap();
    assert!(
        c.grow.is_some(),
        "focused panel should be visible after move"
    );
}

#[test]
fn monocle_move_preserves_visibility() {
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::ActivePanel {
            variant: ActivePanelVariant::Monocle,
            bar_height: 0.0,
        },
        &kinds(3),
    )
    .unwrap();

    let p1 = rt.sequence().get(1).unwrap();
    rt.focus(p1);

    rt.move_panel(p1, 2).unwrap();

    let focused = rt.focused().unwrap();
    let c = rt.tree().panel_constraints(focused).unwrap();
    assert!(
        c.grow.is_some(),
        "focused panel should be visible after move"
    );
}

// ---------------------------------------------------------------------------
// Strategy: swap_next after add_panel is consistent
// ---------------------------------------------------------------------------

#[test]
fn swap_next_after_add_on_all_strategies() {
    let strategies: Vec<StrategyKind> = vec![
        StrategyKind::Sequence {
            direction: Direction::Horizontal,
            gap: 0.0,
            ratio: None,
        },
        StrategyKind::MasterStack {
            master_ratio: 0.5,
            gap: 0.0,
        },
        StrategyKind::CenteredMaster {
            master_ratio: 0.5,
            gap: 0.0,
        },
        StrategyKind::BinarySplit {
            spiral: false,
            ratio: 0.5,
            gap: 0.0,
        },
        StrategyKind::ColumnGrid {
            columns: 2,
            gap: 0.0,
        },
    ];

    for strategy in strategies {
        let mut rt = LayoutRuntime::from_strategy(strategy, &kinds(3)).unwrap();
        rt.add_panel(Arc::from("new")).unwrap();
        rt.swap_next();
        rt.tree().validate().unwrap();
    }
}
