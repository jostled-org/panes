use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{ActivePanelVariant, Direction, FocusDirection, LayoutTree, StrategyKind, grow};

fn kinds(n: usize) -> Vec<Arc<str>> {
    (0..n).map(|i| Arc::from(format!("p{i}"))).collect()
}

fn row_runtime(n: usize) -> LayoutRuntime {
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

fn col_runtime(n: usize) -> LayoutRuntime {
    let k = kinds(n);
    LayoutRuntime::from_strategy(
        StrategyKind::Sequence {
            direction: Direction::Vertical,
            gap: 0.0,
        },
        &k,
    )
    .unwrap()
}

#[test]
fn right_from_leftmost() {
    let mut rt = row_runtime(3);
    let p0 = rt.sequence().get(0).unwrap();
    let p1 = rt.sequence().get(1).unwrap();
    rt.focus(p0).unwrap();
    let frame = rt.resolve(300.0, 100.0).unwrap();

    let result = rt
        .focus_direction(frame.layout(), FocusDirection::Right)
        .unwrap();
    assert_eq!(result, Some(p1));
    assert_eq!(rt.focused(), Some(p1));
}

#[test]
fn left_from_rightmost() {
    let mut rt = row_runtime(3);
    let p1 = rt.sequence().get(1).unwrap();
    let p2 = rt.sequence().get(2).unwrap();
    rt.focus(p2).unwrap();
    let frame = rt.resolve(300.0, 100.0).unwrap();

    let result = rt
        .focus_direction(frame.layout(), FocusDirection::Left)
        .unwrap();
    assert_eq!(result, Some(p1));
    assert_eq!(rt.focused(), Some(p1));
}

#[test]
fn no_candidate_in_direction() {
    let mut rt = row_runtime(3);
    let p0 = rt.sequence().get(0).unwrap();
    rt.focus(p0).unwrap();
    let frame = rt.resolve(300.0, 100.0).unwrap();

    let result = rt
        .focus_direction(frame.layout(), FocusDirection::Left)
        .unwrap();
    assert_eq!(result, None);
    assert_eq!(rt.focused(), Some(p0));
}

#[test]
fn down_in_column() {
    let mut rt = col_runtime(3);
    let p0 = rt.sequence().get(0).unwrap();
    let p1 = rt.sequence().get(1).unwrap();
    rt.focus(p0).unwrap();
    let frame = rt.resolve(100.0, 300.0).unwrap();

    let result = rt
        .focus_direction(frame.layout(), FocusDirection::Down)
        .unwrap();
    assert_eq!(result, Some(p1));
}

#[test]
fn up_in_column() {
    let mut rt = col_runtime(3);
    let p1 = rt.sequence().get(1).unwrap();
    let p2 = rt.sequence().get(2).unwrap();
    rt.focus(p2).unwrap();
    let frame = rt.resolve(100.0, 300.0).unwrap();

    let result = rt
        .focus_direction(frame.layout(), FocusDirection::Up)
        .unwrap();
    assert_eq!(result, Some(p1));
}

#[test]
fn grid_navigation() {
    // 2x2 grid: row[ col[p0, p1], col[p2, p3] ]
    let mut tree = LayoutTree::new();
    let (p0, n0) = tree.add_panel("p0", grow(1.0)).unwrap();
    let (p1, n1) = tree.add_panel("p1", grow(1.0)).unwrap();
    let (p2, n2) = tree.add_panel("p2", grow(1.0)).unwrap();
    let (p3, n3) = tree.add_panel("p3", grow(1.0)).unwrap();
    let left_col = tree.add_col(0.0, vec![n0, n1]).unwrap();
    let right_col = tree.add_col(0.0, vec![n2, n3]).unwrap();
    let root = tree.add_row(0.0, vec![left_col, right_col]).unwrap();
    tree.set_root(root);

    let k: Vec<Arc<str>> = ["p0", "p1", "p2", "p3"]
        .iter()
        .map(|s| Arc::from(*s))
        .collect();
    let mut rt = LayoutRuntime::from_tree_and_strategy(
        tree,
        StrategyKind::Sequence {
            direction: Direction::Horizontal,
            gap: 0.0,
        },
        &k,
    )
    .unwrap();
    rt.focus(p0).unwrap();
    let frame = rt.resolve(200.0, 200.0).unwrap();

    // Right from p0 -> p2 (top-right)
    let result = rt
        .focus_direction(frame.layout(), FocusDirection::Right)
        .unwrap();
    assert_eq!(result, Some(p2));

    // Down from p2 -> p3
    let result = rt
        .focus_direction(frame.layout(), FocusDirection::Down)
        .unwrap();
    assert_eq!(result, Some(p3));

    // Left from p3 -> p1
    let result = rt
        .focus_direction(frame.layout(), FocusDirection::Left)
        .unwrap();
    assert_eq!(result, Some(p1));

    // Up from p1 -> p0
    let result = rt
        .focus_direction(frame.layout(), FocusDirection::Up)
        .unwrap();
    assert_eq!(result, Some(p0));
}

#[test]
fn master_stack_right() {
    let k = kinds(3);
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::MasterStack {
            master_ratio: 0.5,
            gap: 0.0,
        },
        &k,
    )
    .unwrap();
    let master = rt.sequence().get(0).unwrap();
    rt.focus(master).unwrap();
    let frame = rt.resolve(200.0, 200.0).unwrap();

    let result = rt
        .focus_direction(frame.layout(), FocusDirection::Right)
        .unwrap();
    assert!(result.is_some());
    assert_ne!(result.unwrap(), master);
}

#[test]
fn master_stack_left() {
    let k = kinds(3);
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::MasterStack {
            master_ratio: 0.5,
            gap: 0.0,
        },
        &k,
    )
    .unwrap();
    let master = rt.sequence().get(0).unwrap();
    let stack0 = rt.sequence().get(1).unwrap();
    rt.focus(stack0).unwrap();
    let frame = rt.resolve(200.0, 200.0).unwrap();

    let result = rt
        .focus_direction(frame.layout(), FocusDirection::Left)
        .unwrap();
    assert_eq!(result, Some(master));
}

#[test]
fn no_focused_panel() {
    let mut tree = LayoutTree::new();
    let mut nids = Vec::new();
    for i in 0..3 {
        let (_, nid) = tree.add_panel(format!("p{i}"), grow(1.0)).unwrap();
        nids.push(nid);
    }
    let root = tree.add_row(0.0, nids).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::from(tree);
    // No focus set — focus_direction should return Ok(None)
    let frame = rt.resolve(300.0, 100.0).unwrap();

    let result = rt
        .focus_direction(frame.layout(), FocusDirection::Right)
        .unwrap();
    assert_eq!(result, None);
}

#[test]
fn single_panel() {
    let mut rt = row_runtime(1);
    let p0 = rt.sequence().get(0).unwrap();
    rt.focus(p0).unwrap();
    let frame = rt.resolve(100.0, 100.0).unwrap();

    for dir in [
        FocusDirection::Left,
        FocusDirection::Right,
        FocusDirection::Up,
        FocusDirection::Down,
    ] {
        let result = rt.focus_direction(frame.layout(), dir).unwrap();
        assert_eq!(result, None);
    }
}

#[test]
fn zero_area_skipped() {
    let k = kinds(3);
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::ActivePanel {
            variant: ActivePanelVariant::Monocle,
            bar_height: 0.0,
        },
        &k,
    )
    .unwrap();
    let frame = rt.resolve(100.0, 100.0).unwrap();

    let result = rt
        .focus_direction(frame.layout(), FocusDirection::Right)
        .unwrap();
    assert_eq!(result, None);
}

#[test]
fn diagonal_tiebreak() {
    // row[ left_col[s0], right_col[s1, s2, s3] ]
    // 200x300: s0 center (50, 150)
    // s1 center (150, 50), s2 center (150, 150), s3 center (150, 250)
    // Going right from s0: s2 has secondary=0, s1 and s3 have secondary=100
    // s2 wins on secondary distance
    let mut tree = LayoutTree::new();
    let (s0, sn0) = tree.add_panel("s0", grow(1.0)).unwrap();
    let (_, sn1) = tree.add_panel("s1", grow(1.0)).unwrap();
    let (s2, sn2) = tree.add_panel("s2", grow(1.0)).unwrap();
    let (_, sn3) = tree.add_panel("s3", grow(1.0)).unwrap();
    let left = tree.add_col(0.0, vec![sn0]).unwrap();
    let right = tree.add_col(0.0, vec![sn1, sn2, sn3]).unwrap();
    let root = tree.add_row(0.0, vec![left, right]).unwrap();
    tree.set_root(root);

    let k: Vec<Arc<str>> = ["s0", "s1", "s2", "s3"]
        .iter()
        .map(|s| Arc::from(*s))
        .collect();
    let mut rt = LayoutRuntime::from_tree_and_strategy(
        tree,
        StrategyKind::Sequence {
            direction: Direction::Horizontal,
            gap: 0.0,
        },
        &k,
    )
    .unwrap();
    rt.focus(s0).unwrap();
    let frame = rt.resolve(200.0, 300.0).unwrap();

    let result = rt
        .focus_direction(frame.layout(), FocusDirection::Right)
        .unwrap();
    assert_eq!(result, Some(s2));
}

#[test]
fn collapsed_middle_skipped() {
    // Three panels in a row, collapse the middle one.
    // Right from p0 should skip p1 (zero area) and land on p2.
    let mut rt = row_runtime(3);
    let p0 = rt.sequence().get(0).unwrap();
    let p1 = rt.sequence().get(1).unwrap();
    let p2 = rt.sequence().get(2).unwrap();
    rt.focus(p0).unwrap();
    rt.toggle_collapsed(p1).unwrap();
    let frame = rt.resolve(300.0, 100.0).unwrap();

    let result = rt
        .focus_direction(frame.layout(), FocusDirection::Right)
        .unwrap();
    assert_eq!(result, Some(p2));
}

#[test]
fn focus_direction_current_uses_cached_layout() {
    let mut rt = row_runtime(3);
    let p0 = rt.sequence().get(0).unwrap();
    let p1 = rt.sequence().get(1).unwrap();
    rt.focus(p0).unwrap();
    rt.resolve(300.0, 100.0).unwrap();

    let result = rt.focus_direction_current(FocusDirection::Right).unwrap();
    assert_eq!(result, Some(p1));
    assert_eq!(rt.focused(), Some(p1));
}

#[test]
fn focus_direction_current_without_resolve_errors() {
    let mut rt = row_runtime(3);
    let p0 = rt.sequence().get(0).unwrap();
    rt.focus(p0).unwrap();

    let result = rt.focus_direction_current(FocusDirection::Right);
    assert!(result.is_err());
}
