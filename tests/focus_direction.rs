#![allow(clippy::unwrap_used, clippy::panic)]
use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{
    ActivePanelVariant, Axis, FocusDirection, FocusOutcome, FocusRejection, LayoutTree, PaneError,
    StrategyKind, grow,
};

fn kinds(n: usize) -> Vec<Arc<str>> {
    (0..n).map(|i| Arc::from(format!("p{i}"))).collect()
}

fn row_runtime(n: usize) -> LayoutRuntime {
    let k = kinds(n);
    LayoutRuntime::from_strategy(
        StrategyKind::Sequence {
            axis: Axis::Row,
            gap: 0.0,
            ratio: None,
        },
        &k,
    )
    .unwrap()
}

fn col_runtime(n: usize) -> LayoutRuntime {
    let k = kinds(n);
    LayoutRuntime::from_strategy(
        StrategyKind::Sequence {
            axis: Axis::Col,
            gap: 0.0,
            ratio: None,
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
    rt.focus(p0);
    let frame = rt.resolve(300.0, 100.0).unwrap();

    let (target, outcome) = rt
        .focus_direction(frame.layout(), FocusDirection::Right)
        .unwrap();
    assert_eq!(target, Some(p1));
    assert_eq!(outcome, FocusOutcome::Applied);
    assert_eq!(rt.focused(), Some(p1));
}

#[test]
fn left_from_rightmost() {
    let mut rt = row_runtime(3);
    let p1 = rt.sequence().get(1).unwrap();
    let p2 = rt.sequence().get(2).unwrap();
    rt.focus(p2);
    let frame = rt.resolve(300.0, 100.0).unwrap();

    let (target, outcome) = rt
        .focus_direction(frame.layout(), FocusDirection::Left)
        .unwrap();
    assert_eq!(target, Some(p1));
    assert_eq!(outcome, FocusOutcome::Applied);
    assert_eq!(rt.focused(), Some(p1));
}

#[test]
fn no_candidate_in_direction() {
    let mut rt = row_runtime(3);
    let p0 = rt.sequence().get(0).unwrap();
    rt.focus(p0);
    let frame = rt.resolve(300.0, 100.0).unwrap();

    let (target, outcome) = rt
        .focus_direction(frame.layout(), FocusDirection::Left)
        .unwrap();
    assert_eq!(target, None);
    assert_eq!(outcome, FocusOutcome::Unchanged);
    assert_eq!(rt.focused(), Some(p0));
}

#[test]
fn down_in_column() {
    let mut rt = col_runtime(3);
    let p0 = rt.sequence().get(0).unwrap();
    let p1 = rt.sequence().get(1).unwrap();
    rt.focus(p0);
    let frame = rt.resolve(100.0, 300.0).unwrap();

    let (target, outcome) = rt
        .focus_direction(frame.layout(), FocusDirection::Down)
        .unwrap();
    assert_eq!(target, Some(p1));
    assert_eq!(outcome, FocusOutcome::Applied);
}

#[test]
fn up_in_column() {
    let mut rt = col_runtime(3);
    let p1 = rt.sequence().get(1).unwrap();
    let p2 = rt.sequence().get(2).unwrap();
    rt.focus(p2);
    let frame = rt.resolve(100.0, 300.0).unwrap();

    let (target, outcome) = rt
        .focus_direction(frame.layout(), FocusDirection::Up)
        .unwrap();
    assert_eq!(target, Some(p1));
    assert_eq!(outcome, FocusOutcome::Applied);
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
            axis: Axis::Row,
            gap: 0.0,
            ratio: None,
        },
        &k,
    );
    rt.focus(p0);
    let frame = rt.resolve(200.0, 200.0).unwrap();

    // Right from p0 -> p2 (top-right)
    let (target, _) = rt
        .focus_direction(frame.layout(), FocusDirection::Right)
        .unwrap();
    assert_eq!(target, Some(p2));

    // Down from p2 -> p3
    let (target, _) = rt
        .focus_direction(frame.layout(), FocusDirection::Down)
        .unwrap();
    assert_eq!(target, Some(p3));

    // Left from p3 -> p1
    let (target, _) = rt
        .focus_direction(frame.layout(), FocusDirection::Left)
        .unwrap();
    assert_eq!(target, Some(p1));

    // Up from p1 -> p0
    let (target, _) = rt
        .focus_direction(frame.layout(), FocusDirection::Up)
        .unwrap();
    assert_eq!(target, Some(p0));
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
    rt.focus(master);
    let frame = rt.resolve(200.0, 200.0).unwrap();

    let (target, outcome) = rt
        .focus_direction(frame.layout(), FocusDirection::Right)
        .unwrap();
    assert!(target.is_some());
    assert_ne!(target.unwrap(), master);
    assert_eq!(outcome, FocusOutcome::Applied);
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
    rt.focus(stack0);
    let frame = rt.resolve(200.0, 200.0).unwrap();

    let (target, _) = rt
        .focus_direction(frame.layout(), FocusDirection::Left)
        .unwrap();
    assert_eq!(target, Some(master));
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
    // No focus set — no candidate, unchanged.
    let frame = rt.resolve(300.0, 100.0).unwrap();

    let (target, outcome) = rt
        .focus_direction(frame.layout(), FocusDirection::Right)
        .unwrap();
    assert_eq!(target, None);
    assert_eq!(outcome, FocusOutcome::Unchanged);
}

#[test]
fn returns_rejected_when_target_cannot_be_focused() {
    let mut tree = LayoutTree::new();
    let (p0, n0) = tree.add_panel("p0", grow(1.0)).unwrap();
    let (p1, n1) = tree.add_panel("p1", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1]).unwrap();
    tree.set_root(root);

    let mut sequence = panes::PanelSequence::default();
    sequence.push(p0);
    sequence.push(p1);

    let mut rt = LayoutRuntime::from_tree_and_sequence(tree, sequence);
    let frame = rt.resolve(200.0, 100.0).unwrap();
    rt.tree_mut().remove_panel(p1).unwrap();

    // Candidate found (p1 still in layout snapshot) but focus rejected.
    let (target, outcome) = rt
        .focus_direction(frame.layout(), FocusDirection::Right)
        .unwrap();

    assert_eq!(target, Some(p1));
    assert_eq!(
        outcome,
        FocusOutcome::Rejected(FocusRejection::PanelNotFound)
    );
    assert_eq!(rt.focused(), Some(p0));
}

#[test]
fn single_panel() {
    let mut rt = row_runtime(1);
    let p0 = rt.sequence().get(0).unwrap();
    rt.focus(p0);
    let frame = rt.resolve(100.0, 100.0).unwrap();

    for dir in [
        FocusDirection::Left,
        FocusDirection::Right,
        FocusDirection::Up,
        FocusDirection::Down,
    ] {
        let (target, outcome) = rt.focus_direction(frame.layout(), dir).unwrap();
        assert_eq!(target, None);
        assert_eq!(outcome, FocusOutcome::Unchanged);
    }
}

#[test]
fn active_panel_returns_spatial_nav_error() {
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

    let result = rt.focus_direction(frame.layout(), FocusDirection::Right);
    assert!(matches!(
        result,
        Err(PaneError::InvalidMutation(
            panes::MutationError::SpatialNavUnsupported
        ))
    ));
}

#[test]
fn window_returns_spatial_nav_error() {
    let k = kinds(3);
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::Window {
            panel_count: 2,
            gap: 0.0,
        },
        &k,
    )
    .unwrap();
    let frame = rt.resolve(100.0, 100.0).unwrap();

    let result = rt.focus_direction(frame.layout(), FocusDirection::Right);
    assert!(matches!(
        result,
        Err(PaneError::InvalidMutation(
            panes::MutationError::SpatialNavUnsupported
        ))
    ));
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
            axis: Axis::Row,
            gap: 0.0,
            ratio: None,
        },
        &k,
    );
    rt.focus(s0);
    let frame = rt.resolve(200.0, 300.0).unwrap();

    let (target, _) = rt
        .focus_direction(frame.layout(), FocusDirection::Right)
        .unwrap();
    assert_eq!(target, Some(s2));
}

#[test]
fn collapsed_middle_skipped() {
    // Three panels in a row, collapse the middle one.
    // Right from p0 should skip p1 (zero area) and land on p2.
    let mut rt = row_runtime(3);
    let p0 = rt.sequence().get(0).unwrap();
    let p1 = rt.sequence().get(1).unwrap();
    let p2 = rt.sequence().get(2).unwrap();
    rt.focus(p0);
    rt.toggle_collapsed(p1).unwrap();
    let frame = rt.resolve(300.0, 100.0).unwrap();

    let (target, _) = rt
        .focus_direction(frame.layout(), FocusDirection::Right)
        .unwrap();
    assert_eq!(target, Some(p2));
}

#[test]
fn focus_direction_current_uses_cached_layout() {
    let mut rt = row_runtime(3);
    let p0 = rt.sequence().get(0).unwrap();
    let p1 = rt.sequence().get(1).unwrap();
    rt.focus(p0);
    rt.resolve(300.0, 100.0).unwrap();

    let (target, outcome) = rt.focus_direction_current(FocusDirection::Right).unwrap();
    assert_eq!(target, Some(p1));
    assert_eq!(outcome, FocusOutcome::Applied);
    assert_eq!(rt.focused(), Some(p1));
}

#[test]
fn focus_direction_current_without_resolve_returns_none() {
    let mut rt = row_runtime(3);
    let p0 = rt.sequence().get(0).unwrap();
    rt.focus(p0);

    let (target, outcome) = rt.focus_direction_current(FocusDirection::Right).unwrap();
    assert_eq!(target, None);
    assert_eq!(outcome, FocusOutcome::Unchanged);
}

#[test]
fn master_stack_down_prefers_cross_axis_overlap() {
    // Issue #22: Down from a mid-stack panel should go to the panel
    // directly below, not jump to the master panel whose center
    // happens to be vertically closer.
    let k: Vec<Arc<str>> = ["editor", "terminal", "logs", "panel-7", "panel-8"]
        .iter()
        .map(|s| Arc::from(*s))
        .collect();
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::MasterStack {
            master_ratio: 0.6,
            gap: 1.0,
        },
        &k,
    )
    .unwrap();

    let logs = rt.sequence().get(2).unwrap();
    let panel7 = rt.sequence().get(3).unwrap();
    let panel8 = rt.sequence().get(4).unwrap();
    let frame = rt.resolve(180.0, 56.0).unwrap();

    // Down from logs → panel-7 (not editor)
    rt.focus(logs);
    let (target, _) = rt
        .focus_direction(frame.layout(), FocusDirection::Down)
        .unwrap();
    assert_eq!(target, Some(panel7));

    // Down from panel-7 → panel-8
    let (target, _) = rt
        .focus_direction(frame.layout(), FocusDirection::Down)
        .unwrap();
    assert_eq!(target, Some(panel8));

    // Up from panel-8 → panel-7 (not editor)
    let (target, _) = rt
        .focus_direction(frame.layout(), FocusDirection::Up)
        .unwrap();
    assert_eq!(target, Some(panel7));

    // Up from panel-7 → logs
    let (target, _) = rt
        .focus_direction(frame.layout(), FocusDirection::Up)
        .unwrap();
    assert_eq!(target, Some(logs));
}

#[test]
fn focus_direction_reports_rejected_when_target_cannot_be_applied() {
    // Build a runtime, resolve, then remove the only directional target.
    // Directional focus should surface the rejection, not hide it behind None.
    let mut tree = LayoutTree::new();
    let (p0, n0) = tree.add_panel("p0", grow(1.0)).unwrap();
    let (p1, n1) = tree.add_panel("p1", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1]).unwrap();
    tree.set_root(root);

    let mut sequence = panes::PanelSequence::default();
    sequence.push(p0);
    sequence.push(p1);

    let mut rt = LayoutRuntime::from_tree_and_sequence(tree, sequence);
    rt.focus(p0);
    let frame = rt.resolve(200.0, 100.0).unwrap();

    // Remove p1 from the tree — it's the only rightward candidate.
    rt.tree_mut().remove_panel(p1).unwrap();

    // Candidate was found (p1 still in the layout snapshot) but rejected.
    let (target, outcome) = rt
        .focus_direction(frame.layout(), FocusDirection::Right)
        .unwrap();
    assert_eq!(target, Some(p1));
    assert_eq!(
        outcome,
        FocusOutcome::Rejected(FocusRejection::PanelNotFound)
    );
    assert_eq!(rt.focused(), Some(p0));
}
