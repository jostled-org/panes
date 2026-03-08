use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{Direction, StrategyKind};

fn kinds(n: usize) -> Vec<Arc<str>> {
    (0..n).map(|i| Arc::from(format!("p{i}"))).collect()
}

fn sequence_runtime(n: usize) -> LayoutRuntime {
    let k = kinds(n);
    LayoutRuntime::from_strategy(
        StrategyKind::Sequence {
            direction: Direction::Horizontal,
            gap: 0.0,
        },
        &k,
    )
    .unwrap()
}

#[test]
fn sequence_initial_focus_is_first() {
    let rt = sequence_runtime(3);
    let first = rt.sequence().get(0).unwrap();
    assert_eq!(rt.focused(), Some(first));
}

#[test]
fn sequence_add_shifts_focus() {
    let mut rt = sequence_runtime(2);
    let new_pid = rt.add_panel(Arc::from("p_new")).unwrap();
    assert_eq!(rt.focused(), Some(new_pid));
    assert_eq!(rt.sequence().len(), 3);
}

#[test]
fn sequence_remove_focused_shifts_to_neighbor() {
    let mut rt = sequence_runtime(3);
    let p1 = rt.sequence().get(1).unwrap();
    rt.focus(p1);
    let new_focus = rt.remove_panel(p1).unwrap();
    assert!(new_focus.is_some());
    assert_ne!(new_focus, Some(p1));
    assert_eq!(rt.sequence().len(), 2);
}

#[test]
fn sequence_move_updates_focus() {
    let mut rt = sequence_runtime(3);
    let p0 = rt.sequence().get(0).unwrap();
    let moved = rt.move_panel(p0, 2).unwrap();
    assert_eq!(rt.focused(), Some(moved));
}

#[test]
fn sequence_focus_next_wraps() {
    let mut rt = sequence_runtime(3);
    let p2 = rt.sequence().get(2).unwrap();
    let p0 = rt.sequence().get(0).unwrap();
    rt.focus(p2);
    rt.focus_next();
    assert_eq!(rt.focused(), Some(p0));
}

#[test]
fn sequence_focus_prev_wraps() {
    let mut rt = sequence_runtime(3);
    let p0 = rt.sequence().get(0).unwrap();
    let p2 = rt.sequence().get(2).unwrap();
    assert_eq!(rt.focused(), Some(p0));
    rt.focus_prev();
    assert_eq!(rt.focused(), Some(p2));
}

// -- MasterStack --

fn master_stack_runtime(n: usize) -> LayoutRuntime {
    let k = kinds(n);
    LayoutRuntime::from_strategy(
        StrategyKind::MasterStack {
            master_ratio: 0.5,
            gap: 0.0,
        },
        &k,
    )
    .unwrap()
}

#[test]
fn master_stack_add_goes_to_stack() {
    let mut rt = master_stack_runtime(3);
    let new_pid = rt.add_panel(Arc::from("p_new")).unwrap();
    assert_eq!(rt.focused(), Some(new_pid));
    assert_eq!(rt.sequence().len(), 4);
}

#[test]
fn master_stack_remove_master_promotes_next() {
    let mut rt = master_stack_runtime(3);
    let master = rt.sequence().get(0).unwrap();
    let new_focus = rt.remove_panel(master).unwrap();
    assert!(new_focus.is_some());
    assert_eq!(rt.sequence().len(), 2);
}

// -- CenteredMaster --

fn centered_master_runtime(n: usize) -> LayoutRuntime {
    let k = kinds(n);
    LayoutRuntime::from_strategy(
        StrategyKind::CenteredMaster {
            master_ratio: 0.5,
            gap: 0.0,
        },
        &k,
    )
    .unwrap()
}

#[test]
fn centered_master_add_panel() {
    let mut rt = centered_master_runtime(3);
    let new_pid = rt.add_panel(Arc::from("p_new")).unwrap();
    assert_eq!(rt.focused(), Some(new_pid));
    assert_eq!(rt.sequence().len(), 4);
}

// -- Deck --

fn deck_runtime(n: usize) -> LayoutRuntime {
    let k = kinds(n);
    LayoutRuntime::from_strategy(
        StrategyKind::Deck {
            master_ratio: 0.5,
            gap: 0.0,
        },
        &k,
    )
    .unwrap()
}

#[test]
fn deck_focus_changes_visibility() {
    let mut rt = deck_runtime(3);
    let p1 = rt.sequence().get(1).unwrap();
    let p2 = rt.sequence().get(2).unwrap();
    rt.focus(p2);
    assert_eq!(rt.focused(), Some(p2));
    // p1 should be hidden (stack panel), p2 visible
    let c1 = rt.tree().panel_constraints(p1).unwrap();
    assert_eq!(c1.fixed, Some(0.0));
    let c2 = rt.tree().panel_constraints(p2).unwrap();
    assert!(c2.grow.is_some());
}

#[test]
fn deck_remove_master_promotes() {
    let mut rt = deck_runtime(3);
    let master = rt.sequence().get(0).unwrap();
    let new_focus = rt.remove_panel(master).unwrap();
    assert!(new_focus.is_some());
    assert_eq!(rt.sequence().len(), 2);
}
