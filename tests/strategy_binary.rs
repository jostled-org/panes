#![allow(clippy::unwrap_used, clippy::panic)]
use std::sync::Arc;

use panes::StrategyKind;
use panes::runtime::LayoutRuntime;

fn kinds(n: usize) -> Vec<Arc<str>> {
    (0..n).map(|i| Arc::from(format!("p{i}"))).collect()
}

fn dwindle_runtime(n: usize) -> LayoutRuntime {
    let k = kinds(n);
    LayoutRuntime::from_strategy(
        StrategyKind::BinarySplit {
            spiral: false,
            ratio: 0.5,
            gap: 0.0,
        },
        &k,
    )
    .unwrap()
}

#[test]
fn spiral_focus_order_matches_input() {
    let named: Vec<Arc<str>> = ["a", "b", "c", "d", "e"]
        .iter()
        .map(|s| Arc::from(*s))
        .collect();
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::BinarySplit {
            spiral: true,
            ratio: 0.5,
            gap: 0.0,
        },
        &named,
    )
    .unwrap();

    // Sequence should follow input order, not DFS tree order
    let mut visited = Vec::new();
    for _ in 0..5 {
        let pid = rt.focused().unwrap();
        let kind = rt.tree().panel_kind(pid).unwrap().to_string();
        visited.push(kind);
        rt.focus_next();
    }
    assert_eq!(visited, vec!["a", "b", "c", "d", "e"]);
}

fn spiral_runtime(n: usize) -> LayoutRuntime {
    let k = kinds(n);
    LayoutRuntime::from_strategy(
        StrategyKind::BinarySplit {
            spiral: true,
            ratio: 0.5,
            gap: 0.0,
        },
        &k,
    )
    .unwrap()
}

#[test]
fn dwindle_add_rebuilds_tree() {
    let mut rt = dwindle_runtime(3);
    let new_pid = rt.add_panel(Arc::from("p_new")).unwrap();
    assert_eq!(rt.focused(), Some(new_pid));
    assert_eq!(rt.sequence().len(), 4);
}

#[test]
fn dwindle_remove_rebuilds_tree() {
    let mut rt = dwindle_runtime(4);
    let p1 = rt.sequence().get(1).unwrap();
    let new_focus = rt.remove_panel(p1).unwrap();
    assert!(new_focus.is_some());
    assert_eq!(rt.sequence().len(), 3);
}

#[test]
fn dwindle_move_rebuilds_tree() {
    let mut rt = dwindle_runtime(4);
    let p0 = rt.sequence().get(0).unwrap();
    let moved = rt.move_panel(p0, 3).unwrap();
    assert_eq!(rt.focused(), Some(moved));
}

#[test]
fn spiral_add_rebuilds_tree() {
    let mut rt = spiral_runtime(3);
    let new_pid = rt.add_panel(Arc::from("p_new")).unwrap();
    assert_eq!(rt.focused(), Some(new_pid));
    assert_eq!(rt.sequence().len(), 4);
}

#[test]
fn spiral_remove_rebuilds_tree() {
    let mut rt = spiral_runtime(4);
    let p2 = rt.sequence().get(2).unwrap();
    let new_focus = rt.remove_panel(p2).unwrap();
    assert!(new_focus.is_some());
    assert_eq!(rt.sequence().len(), 3);
}
