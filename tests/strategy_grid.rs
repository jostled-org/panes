use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{CardSpan, GridColumnMode, StrategyKind};

fn kinds(n: usize) -> Vec<Arc<str>> {
    (0..n).map(|i| Arc::from(format!("p{i}"))).collect()
}

fn dashboard_fixed_runtime(n: usize, columns: usize) -> LayoutRuntime {
    let k = kinds(n);
    let spans: Arc<[CardSpan]> = vec![CardSpan::Columns(1); n].into();
    LayoutRuntime::from_strategy(
        StrategyKind::Dashboard {
            columns: GridColumnMode::Fixed(columns),
            gap: 0.0,
            spans,
        },
        &k,
    )
    .unwrap()
}

fn dashboard_auto_fill_runtime(n: usize) -> LayoutRuntime {
    let k = kinds(n);
    let spans: Arc<[CardSpan]> = vec![CardSpan::Columns(1); n].into();
    LayoutRuntime::from_strategy(
        StrategyKind::Dashboard {
            columns: GridColumnMode::AutoFill { min_width: 200.0 },
            gap: 0.0,
            spans,
        },
        &k,
    )
    .unwrap()
}

#[test]
fn dashboard_add_rebuilds() {
    let mut rt = dashboard_fixed_runtime(4, 2);
    let new_pid = rt.add_panel(Arc::from("p_new")).unwrap();
    assert_eq!(rt.focused(), Some(new_pid));
    assert_eq!(rt.sequence().len(), 5);
}

#[test]
fn dashboard_remove_rebuilds() {
    let mut rt = dashboard_fixed_runtime(4, 2);
    let p2 = rt.sequence().get(2).unwrap();
    let new_focus = rt.remove_panel(p2).unwrap();
    assert!(new_focus.is_some());
    assert_eq!(rt.sequence().len(), 3);
}

#[test]
fn dashboard_move_rebuilds() {
    let mut rt = dashboard_fixed_runtime(4, 2);
    let p0 = rt.sequence().get(0).unwrap();
    let moved = rt.move_panel(p0, 3).unwrap();
    assert_eq!(rt.focused(), Some(moved));
}

#[test]
fn dashboard_3col_add_rebuilds() {
    let mut rt = dashboard_fixed_runtime(4, 3);
    let new_pid = rt.add_panel(Arc::from("p_new")).unwrap();
    assert_eq!(rt.focused(), Some(new_pid));
    assert_eq!(rt.sequence().len(), 5);
}

#[test]
fn dashboard_3col_remove_rebuilds() {
    let mut rt = dashboard_fixed_runtime(4, 3);
    let p1 = rt.sequence().get(1).unwrap();
    let new_focus = rt.remove_panel(p1).unwrap();
    assert!(new_focus.is_some());
    assert_eq!(rt.sequence().len(), 3);
}

#[test]
fn dashboard_3col_move_rebuilds() {
    let mut rt = dashboard_fixed_runtime(4, 3);
    let p0 = rt.sequence().get(0).unwrap();
    let moved = rt.move_panel(p0, 3).unwrap();
    assert_eq!(rt.focused(), Some(moved));
}

#[test]
fn dashboard_auto_fill_add_remove() {
    let mut rt = dashboard_auto_fill_runtime(4);
    let new_pid = rt.add_panel(Arc::from("p_new")).unwrap();
    assert_eq!(rt.focused(), Some(new_pid));
    assert_eq!(rt.sequence().len(), 5);

    let p1 = rt.sequence().get(1).unwrap();
    let new_focus = rt.remove_panel(p1).unwrap();
    assert!(new_focus.is_some());
    assert_eq!(rt.sequence().len(), 4);
}
