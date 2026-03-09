mod helpers;

use helpers::build_row_tree;
use panes::runtime::LayoutRuntime;
use panes::{fixed, grow};

#[test]
fn two_equal_resize_larger() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(pids[0], 0.2).unwrap();

    let frame = rt.resolve(1000.0, 100.0).unwrap();
    let r0 = frame.layout().get(pids[0]).unwrap();
    let r1 = frame.layout().get(pids[1]).unwrap();
    let total = r0.w + r1.w;
    let ratio = r0.w / total;
    assert!((ratio - 0.7).abs() < 0.01, "expected ~0.7, got {ratio}");
}

#[test]
fn two_equal_resize_smaller() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(pids[0], -0.2).unwrap();

    let frame = rt.resolve(1000.0, 100.0).unwrap();
    let r0 = frame.layout().get(pids[0]).unwrap();
    let r1 = frame.layout().get(pids[1]).unwrap();
    let total = r0.w + r1.w;
    let ratio = r0.w / total;
    assert!((ratio - 0.3).abs() < 0.01, "expected ~0.3, got {ratio}");
}

#[test]
fn three_panels_proportional() {
    let (tree, pids) = build_row_tree(3, grow(1.0));
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(pids[1], 0.1).unwrap();

    let frame = rt.resolve(1000.0, 100.0).unwrap();
    let widths: Vec<f32> = pids
        .iter()
        .map(|&p| frame.layout().get(p).unwrap().w)
        .collect();
    let total: f32 = widths.iter().sum();
    let mid_ratio = widths[1] / total;
    assert!(
        (mid_ratio - 0.433).abs() < 0.02,
        "expected ~0.433, got {mid_ratio}"
    );
    // Other two should be equal
    assert!(
        (widths[0] - widths[2]).abs() < 1.0,
        "siblings should be equal: {} vs {}",
        widths[0],
        widths[2]
    );
}

#[test]
fn clamp_large_positive() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(pids[0], 0.99).unwrap();

    let c0 = rt.tree().panel_constraints(pids[0]).unwrap();
    let c1 = rt.tree().panel_constraints(pids[1]).unwrap();
    assert!(c0.grow.unwrap() > 0.0);
    assert!(c1.grow.unwrap() > 0.0);
}

#[test]
fn clamp_large_negative() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(pids[0], -0.99).unwrap();

    let c0 = rt.tree().panel_constraints(pids[0]).unwrap();
    let c1 = rt.tree().panel_constraints(pids[1]).unwrap();
    assert!(c0.grow.unwrap() > 0.0);
    assert!(c1.grow.unwrap() > 0.0);
}

#[test]
fn error_on_fixed_sibling() {
    let mut tree = panes::LayoutTree::new();
    let (p0, n0) = tree.add_panel("a", grow(1.0)).unwrap();
    let (_, n1) = tree.add_panel("b", fixed(100.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    let err = rt.resize_boundary(p0, 0.1).unwrap_err();
    assert!(
        err.to_string().contains("grow constraints"),
        "unexpected error: {err}"
    );
}

#[test]
fn error_on_single_child() {
    let mut tree = panes::LayoutTree::new();
    let (p0, n0) = tree.add_panel("a", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    let err = rt.resize_boundary(p0, 0.1).unwrap_err();
    assert!(
        err.to_string().contains("only child"),
        "unexpected error: {err}"
    );
}

#[test]
fn error_on_nan_delta() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::new(tree);

    let err = rt.resize_boundary(pids[0], f32::NAN).unwrap_err();
    assert!(
        err.to_string().contains("finite"),
        "unexpected error: {err}"
    );
}

#[test]
fn preserves_min_max() {
    let mut tree = panes::LayoutTree::new();
    let (p0, n0) = tree.add_panel("a", grow(1.0).min(20.0)).unwrap();
    let (_, n1) = tree.add_panel("b", grow(1.0).max(500.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(p0, 0.1).unwrap();

    let c0 = rt.tree().panel_constraints(p0).unwrap();
    assert_eq!(c0.min, Some(20.0));
}

#[test]
fn zero_delta_is_noop() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::new(tree);

    let before_0 = rt.tree().panel_constraints(pids[0]).unwrap();
    let before_1 = rt.tree().panel_constraints(pids[1]).unwrap();

    rt.resize_boundary(pids[0], 0.0).unwrap();

    let after_0 = rt.tree().panel_constraints(pids[0]).unwrap();
    let after_1 = rt.tree().panel_constraints(pids[1]).unwrap();
    assert_eq!(before_0, after_0);
    assert_eq!(before_1, after_1);
}
