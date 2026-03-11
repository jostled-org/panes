use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{Direction, LayoutTree, Node, Placement, grow};

// ---------------------------------------------------------------------------
// Non-strategy runtimes: direct tree topology via add_panel_adjacent_with
// ---------------------------------------------------------------------------

fn build_tree_runtime(count: usize, direction: Direction) -> LayoutRuntime {
    let mut tree = LayoutTree::new();
    let mut pids = Vec::new();
    let nodes: Vec<_> = (0..count)
        .map(|i| {
            let (pid, nid) = tree
                .add_panel(Arc::from(format!("p{i}")), grow(1.0))
                .unwrap();
            pids.push(pid);
            nid
        })
        .collect();
    let root = match direction {
        Direction::Vertical => tree.add_col(0.0, nodes).unwrap(),
        Direction::Horizontal => tree.add_row(0.0, nodes).unwrap(),
    };
    tree.set_root(root);
    let mut seq = panes::PanelSequence::default();
    for pid in &pids {
        seq.insert(seq.len(), *pid);
    }
    let mut rt = LayoutRuntime::from_tree_and_sequence(tree, seq);
    rt.set_focus_unchecked(pids[0]);
    rt
}

fn build_col_tree(count: usize) -> LayoutRuntime {
    build_tree_runtime(count, Direction::Vertical)
}

fn build_row_tree(count: usize) -> LayoutRuntime {
    build_tree_runtime(count, Direction::Horizontal)
}

#[test]
fn insert_horizontal_in_row() {
    let mut rt = build_row_tree(3);
    let focused = rt.focused().unwrap();
    let focused_nid = rt.tree().node_for_panel(focused).unwrap();
    let parent_id = rt.tree().parent(focused_nid).unwrap().unwrap();

    let new_pid = rt
        .add_panel_adjacent_with(
            Arc::from("new"),
            Direction::Horizontal,
            grow(1.0),
            Placement::After,
        )
        .unwrap();

    let children = rt.tree().children(parent_id).unwrap();
    assert_eq!(children.len(), 4);
    let new_nid = rt.tree().node_for_panel(new_pid).unwrap();
    assert_eq!(children[1], new_nid, "new panel should be after focused");
    rt.tree().validate().unwrap();
}

#[test]
fn insert_vertical_in_col() {
    let mut rt = build_col_tree(3);
    let focused = rt.focused().unwrap();
    let focused_nid = rt.tree().node_for_panel(focused).unwrap();
    let parent_id = rt.tree().parent(focused_nid).unwrap().unwrap();

    let new_pid = rt
        .add_panel_adjacent_with(
            Arc::from("new"),
            Direction::Vertical,
            grow(1.0),
            Placement::After,
        )
        .unwrap();

    let children = rt.tree().children(parent_id).unwrap();
    assert_eq!(children.len(), 4);
    let new_nid = rt.tree().node_for_panel(new_pid).unwrap();
    assert_eq!(children[1], new_nid);
    rt.tree().validate().unwrap();
}

#[test]
fn cross_axis_horizontal_in_col() {
    let mut rt = build_col_tree(3);
    let focused = rt.focused().unwrap();
    let focused_nid = rt.tree().node_for_panel(focused).unwrap();
    let parent_id = rt.tree().parent(focused_nid).unwrap().unwrap();

    let new_pid = rt
        .add_panel_adjacent_with(
            Arc::from("new"),
            Direction::Horizontal,
            grow(1.0),
            Placement::After,
        )
        .unwrap();

    // Parent col should still have 3 children (focused replaced by sub-container)
    let children = rt.tree().children(parent_id).unwrap();
    assert_eq!(children.len(), 3);

    // First child should be a Row containing [focused, new]
    let sub_container = children[0];
    assert!(matches!(
        rt.tree().node(sub_container),
        Some(Node::Row { .. })
    ));
    let sub_children = rt.tree().children(sub_container).unwrap();
    assert_eq!(sub_children.len(), 2);
    assert_eq!(sub_children[0], focused_nid);
    assert_eq!(sub_children[1], rt.tree().node_for_panel(new_pid).unwrap());
    rt.tree().validate().unwrap();
}

#[test]
fn cross_axis_vertical_in_row() {
    let mut rt = build_row_tree(3);
    let focused = rt.focused().unwrap();
    let focused_nid = rt.tree().node_for_panel(focused).unwrap();
    let parent_id = rt.tree().parent(focused_nid).unwrap().unwrap();

    let new_pid = rt
        .add_panel_adjacent_with(
            Arc::from("new"),
            Direction::Vertical,
            grow(1.0),
            Placement::After,
        )
        .unwrap();

    let children = rt.tree().children(parent_id).unwrap();
    assert_eq!(children.len(), 3);

    let sub_container = children[0];
    assert!(matches!(
        rt.tree().node(sub_container),
        Some(Node::Col { .. })
    ));
    let sub_children = rt.tree().children(sub_container).unwrap();
    assert_eq!(sub_children.len(), 2);
    assert_eq!(sub_children[0], focused_nid);
    assert_eq!(sub_children[1], rt.tree().node_for_panel(new_pid).unwrap());
    rt.tree().validate().unwrap();
}

#[test]
fn new_panel_is_focused() {
    let mut rt = build_row_tree(2);
    let new_pid = rt.add_panel(Arc::from("new")).unwrap();
    assert_eq!(rt.focused(), Some(new_pid));
}

#[test]
fn new_panel_in_sequence() {
    let mut rt = build_row_tree(3);
    let focused = rt.focused().unwrap();
    let focused_idx = rt.sequence().index_of(focused).unwrap();

    let new_pid = rt.add_panel(Arc::from("new")).unwrap();

    let new_idx = rt.sequence().index_of(new_pid).unwrap();
    assert_eq!(new_idx, focused_idx + 1);
}

#[test]
fn no_focus_returns_error() {
    let mut tree = LayoutTree::new();
    let (_, n0) = tree.add_panel("a", grow(1.0)).unwrap();
    let (_, n1) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    let result = rt.add_panel(Arc::from("new"));
    assert!(result.is_err());
}

#[test]
fn strategy_adjacent_delegates_to_strategy() {
    let kinds: Vec<Arc<str>> = vec![Arc::from("editor"), Arc::from("terminal")];
    let strategy = panes::StrategyKind::MasterStack {
        master_ratio: 0.5,
        gap: 0.0,
    };
    let mut rt = LayoutRuntime::from_strategy(strategy, &kinds).unwrap();

    let new_pid = rt.add_panel(Arc::from("sidebar")).unwrap();

    assert_eq!(rt.focused(), Some(new_pid));
    rt.tree().validate().unwrap();
}

#[test]
fn repeated_adjacent() {
    let mut rt = build_row_tree(2);
    let original_focused = rt.focused().unwrap();

    let first = rt
        .add_panel_adjacent_with(
            Arc::from("a"),
            Direction::Horizontal,
            grow(1.0),
            Placement::After,
        )
        .unwrap();
    assert_eq!(rt.focused(), Some(first));

    let second = rt
        .add_panel_adjacent_with(
            Arc::from("b"),
            Direction::Horizontal,
            grow(1.0),
            Placement::After,
        )
        .unwrap();
    assert_eq!(rt.focused(), Some(second));

    // Sequence order: original_focused, first, second, ...
    let seq = rt.sequence();
    let idx_orig = seq.index_of(original_focused).unwrap();
    let idx_first = seq.index_of(first).unwrap();
    let idx_second = seq.index_of(second).unwrap();
    assert_eq!(idx_first, idx_orig + 1);
    assert_eq!(idx_second, idx_first + 1);
    rt.tree().validate().unwrap();
}

#[test]
fn cross_axis_preserves_parent_gap() {
    let mut tree = panes::LayoutTree::new();
    let (p0, n0) = tree.add_panel("p0", grow(1.0)).unwrap();
    let (_, n1) = tree.add_panel("p1", grow(1.0)).unwrap();
    let (_, n2) = tree.add_panel("p2", grow(1.0)).unwrap();
    let root = tree.add_row(8.0, vec![n0, n1, n2]).unwrap();
    tree.set_root(root);

    let mut rt = LayoutRuntime::new(tree);
    rt.set_focus_unchecked(p0);

    rt.add_panel_adjacent_with(
        Arc::from("new"),
        Direction::Vertical,
        grow(1.0),
        Placement::After,
    )
    .unwrap();

    // Parent row should still have gap=8
    match rt.tree().node(root) {
        Some(Node::Row { gap, .. }) => assert_eq!(*gap, 8.0),
        other => panic!("expected Row, got {other:?}"),
    }
    rt.tree().validate().unwrap();
}

#[test]
fn repeated_cross_axis() {
    let mut rt = build_row_tree(2);

    // Alternate vertical and horizontal splits three times
    rt.add_panel_adjacent_with(
        Arc::from("v1"),
        Direction::Vertical,
        grow(1.0),
        Placement::After,
    )
    .unwrap();
    rt.add_panel_adjacent_with(
        Arc::from("h1"),
        Direction::Horizontal,
        grow(1.0),
        Placement::After,
    )
    .unwrap();
    rt.add_panel_adjacent_with(
        Arc::from("v2"),
        Direction::Vertical,
        grow(1.0),
        Placement::After,
    )
    .unwrap();

    rt.tree().validate().unwrap();
    assert_eq!(rt.sequence().len(), 5);
}

#[test]
fn single_child_parent() {
    let mut tree = LayoutTree::new();
    let (p0, n0) = tree.add_panel("only", grow(1.0)).unwrap();
    let root = tree.add_col(0.0, vec![n0]).unwrap();
    tree.set_root(root);

    let mut rt = LayoutRuntime::new(tree);
    rt.set_focus_unchecked(p0);

    let new_pid = rt
        .add_panel_adjacent_with(
            Arc::from("sibling"),
            Direction::Vertical,
            grow(1.0),
            Placement::After,
        )
        .unwrap();

    // Same axis: should be a sibling, not wrapped
    let children = rt.tree().children(root).unwrap();
    assert_eq!(children.len(), 2);
    let new_nid = rt.tree().node_for_panel(new_pid).unwrap();
    assert_eq!(children[1], new_nid);
    rt.tree().validate().unwrap();
}

#[test]
fn resolve_after_adjacent() {
    let mut rt = build_row_tree(2);
    rt.add_panel_adjacent_with(
        Arc::from("new"),
        Direction::Vertical,
        grow(1.0),
        Placement::After,
    )
    .unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    let layout = frame.layout();

    // All 3 panels should have valid geometry
    for entry in layout.panels() {
        let r = entry.rect;
        assert!(r.w > 0.0, "panel {} has zero width", entry.id);
        assert!(r.h > 0.0, "panel {} has zero height", entry.id);
    }
}

#[test]
fn adjacent_with_custom_constraints() {
    let mut rt = build_row_tree(2);
    let custom = panes::fixed(200.0);

    let new_pid = rt
        .add_panel_adjacent_with(
            Arc::from("fixed"),
            Direction::Horizontal,
            custom,
            Placement::After,
        )
        .unwrap();

    let stored = rt.tree().panel_constraints(new_pid).unwrap();
    assert_eq!(stored.fixed, Some(200.0));
    rt.tree().validate().unwrap();
}

#[test]
fn placement_before_same_axis() {
    let mut rt = build_row_tree(3);
    let focused = rt.focused().unwrap();
    let focused_nid = rt.tree().node_for_panel(focused).unwrap();
    let parent_id = rt.tree().parent(focused_nid).unwrap().unwrap();

    let new_pid = rt
        .add_panel_adjacent_with(
            Arc::from("before"),
            Direction::Horizontal,
            grow(1.0),
            Placement::Before,
        )
        .unwrap();

    let children = rt.tree().children(parent_id).unwrap();
    let new_nid = rt.tree().node_for_panel(new_pid).unwrap();
    // New panel should be at the position where focused was (index 0)
    assert_eq!(children[0], new_nid);
    assert_eq!(children[1], focused_nid);
    rt.tree().validate().unwrap();
}

#[test]
fn placement_before_cross_axis() {
    let mut rt = build_col_tree(3);
    let focused = rt.focused().unwrap();
    let focused_nid = rt.tree().node_for_panel(focused).unwrap();

    let new_pid = rt
        .add_panel_adjacent_with(
            Arc::from("before"),
            Direction::Horizontal,
            grow(1.0),
            Placement::Before,
        )
        .unwrap();

    // Cross-axis wraps in a Row: [new, focused]
    let new_nid = rt.tree().node_for_panel(new_pid).unwrap();
    let new_parent = rt.tree().parent(new_nid).unwrap().unwrap();
    assert!(matches!(rt.tree().node(new_parent), Some(Node::Row { .. })));

    let sub_children = rt.tree().children(new_parent).unwrap();
    assert_eq!(sub_children.len(), 2);
    assert_eq!(sub_children[0], new_nid);
    assert_eq!(sub_children[1], focused_nid);
    rt.tree().validate().unwrap();
}

#[test]
fn placement_before_sequence_order() {
    let mut rt = build_row_tree(3);
    let focused = rt.focused().unwrap();
    let focused_idx = rt.sequence().index_of(focused).unwrap();

    let new_pid = rt
        .add_panel_adjacent_with(
            Arc::from("before"),
            Direction::Horizontal,
            grow(1.0),
            Placement::Before,
        )
        .unwrap();

    // New panel should be at focused's original sequence index
    let new_idx = rt.sequence().index_of(new_pid).unwrap();
    assert_eq!(new_idx, focused_idx);
    // Focused should have shifted right
    let shifted = rt.sequence().index_of(focused).unwrap();
    assert_eq!(shifted, focused_idx + 1);
}

#[test]
fn auto_direction_splits_wider_panel_horizontally() {
    let mut rt = build_row_tree(1);
    // Resolve at landscape dimensions so the single panel is wider than tall
    rt.resolve(800.0, 200.0).unwrap();

    let new_pid = rt.add_panel(Arc::from("auto")).unwrap();

    // Wider panel → horizontal split → sibling in same row
    let new_nid = rt.tree().node_for_panel(new_pid).unwrap();
    let parent = rt.tree().parent(new_nid).unwrap().unwrap();
    assert!(matches!(rt.tree().node(parent), Some(Node::Row { .. })));
    rt.tree().validate().unwrap();
}

#[test]
fn auto_direction_splits_taller_panel_vertically() {
    let mut rt = build_col_tree(1);
    // Resolve at portrait dimensions so the single panel is taller than wide
    rt.resolve(200.0, 800.0).unwrap();

    let new_pid = rt.add_panel(Arc::from("auto")).unwrap();

    // Taller panel → vertical split → sibling in same col
    let new_nid = rt.tree().node_for_panel(new_pid).unwrap();
    let parent = rt.tree().parent(new_nid).unwrap().unwrap();
    assert!(matches!(rt.tree().node(parent), Some(Node::Col { .. })));
    rt.tree().validate().unwrap();
}

#[test]
fn auto_direction_defaults_horizontal_without_resolve() {
    let mut rt = build_row_tree(2);
    // No resolve() call — no cached layout

    let new_pid = rt.add_panel(Arc::from("auto")).unwrap();

    // Fallback horizontal + parent is already a row → sibling
    let new_nid = rt.tree().node_for_panel(new_pid).unwrap();
    let parent = rt.tree().parent(new_nid).unwrap().unwrap();
    assert!(matches!(rt.tree().node(parent), Some(Node::Row { .. })));
    rt.tree().validate().unwrap();
}

// ---------------------------------------------------------------------------
// Strategy runtimes: add_panel_adjacent_with delegates to strategy
// ---------------------------------------------------------------------------

#[test]
fn strategy_adjacent_with_delegates() {
    let kinds: Vec<Arc<str>> = vec![Arc::from("a"), Arc::from("b"), Arc::from("c")];
    let strategy = panes::StrategyKind::Sequence {
        direction: Direction::Horizontal,
        gap: 0.0,
    };
    let mut rt = LayoutRuntime::from_strategy(strategy, &kinds).unwrap();

    // add_panel_adjacent_with on a strategy runtime should delegate
    let new_pid = rt
        .add_panel_adjacent_with(
            Arc::from("new"),
            Direction::Vertical,
            grow(1.0),
            Placement::After,
        )
        .unwrap();

    assert_eq!(rt.focused(), Some(new_pid));
    rt.tree().validate().unwrap();

    // Should still be strategy-consistent: swap_next should work
    rt.swap_next();
    rt.tree().validate().unwrap();
}

#[test]
fn swap_after_add_panel_is_consistent() {
    let kinds: Vec<Arc<str>> = vec![Arc::from("a"), Arc::from("b")];
    let strategy = panes::StrategyKind::Sequence {
        direction: Direction::Horizontal,
        gap: 0.0,
    };
    let mut rt = LayoutRuntime::from_strategy(strategy, &kinds).unwrap();

    rt.add_panel(Arc::from("c")).unwrap();
    rt.swap_next();
    rt.tree().validate().unwrap();
}
