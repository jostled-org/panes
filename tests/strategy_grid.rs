use std::sync::Arc;

use panes::StrategyKind;
use panes::runtime::LayoutRuntime;

fn kinds(n: usize) -> Vec<Arc<str>> {
    (0..n).map(|i| Arc::from(format!("p{i}"))).collect()
}

fn grid_runtime(n: usize) -> LayoutRuntime {
    let k = kinds(n);
    LayoutRuntime::from_strategy(
        StrategyKind::ColumnGrid {
            columns: 2,
            gap: 0.0,
        },
        &k,
    )
    .unwrap()
}

fn dashboard_runtime(n: usize) -> LayoutRuntime {
    let k = kinds(n);
    let spans: Arc<[usize]> = vec![1; n].into();
    LayoutRuntime::from_strategy(
        StrategyKind::Dashboard {
            columns: 2,
            gap: 0.0,
            spans,
        },
        &k,
    )
    .unwrap()
}

#[test]
fn grid_add_rebuilds() {
    let mut rt = grid_runtime(4);
    let new_pid = rt.add_panel(Arc::from("p_new")).unwrap();
    assert_eq!(rt.focused(), Some(new_pid));
    assert_eq!(rt.sequence().len(), 5);
}

#[test]
fn grid_remove_rebuilds() {
    let mut rt = grid_runtime(4);
    let p1 = rt.sequence().get(1).unwrap();
    let new_focus = rt.remove_panel(p1).unwrap();
    assert!(new_focus.is_some());
    assert_eq!(rt.sequence().len(), 3);
}

#[test]
fn grid_move_rebuilds() {
    let mut rt = grid_runtime(4);
    let p0 = rt.sequence().get(0).unwrap();
    let moved = rt.move_panel(p0, 3).unwrap();
    assert_eq!(rt.focused(), Some(moved));
}

#[test]
fn dashboard_add_rebuilds() {
    let mut rt = dashboard_runtime(4);
    let new_pid = rt.add_panel(Arc::from("p_new")).unwrap();
    assert_eq!(rt.focused(), Some(new_pid));
    assert_eq!(rt.sequence().len(), 5);
}

#[test]
fn dashboard_remove_rebuilds() {
    let mut rt = dashboard_runtime(4);
    let p2 = rt.sequence().get(2).unwrap();
    let new_focus = rt.remove_panel(p2).unwrap();
    assert!(new_focus.is_some());
    assert_eq!(rt.sequence().len(), 3);
}
