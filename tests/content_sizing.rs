use panes::compiler::{compile, compute_layout, panel_layout};
use panes::runtime::LayoutRuntime;
use panes::{Align, Layout, LayoutTree, PaneError, fixed, grow};

// --- Step 1: Cross-axis min/max constraints ---

#[test]
fn grow_panel_with_max_height() {
    // Col with two grow panels, one has max_height(100.0)
    let mut tree = LayoutTree::new();
    let (pa, a) = tree.add_panel("a", grow(1.0).max_height(100.0)).unwrap();
    let (pb, b) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_col(0.0, vec![a, b]).unwrap();
    tree.set_root(root);

    let mut result = compile(&tree).unwrap();
    compute_layout(&mut result, 400.0, 400.0).unwrap();

    let la = panel_layout(&result, &tree, pa).unwrap();
    assert!(
        la.size.height <= 100.0,
        "constrained panel height {} should be <= 100.0",
        la.size.height
    );

    let lb = panel_layout(&result, &tree, pb).unwrap();
    assert!(
        lb.size.height > 100.0,
        "unconstrained panel height {} should fill remaining space",
        lb.size.height
    );
}

#[test]
fn fixed_panel_with_min_width() {
    // Row with fixed(50.0) panel that has min_width(80.0)
    let mut tree = LayoutTree::new();
    let (pa, a) = tree.add_panel("a", fixed(50.0).min_width(80.0)).unwrap();
    let (_, b) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![a, b]).unwrap();
    tree.set_root(root);

    let mut result = compile(&tree).unwrap();
    compute_layout(&mut result, 400.0, 400.0).unwrap();

    let la = panel_layout(&result, &tree, pa).unwrap();
    assert!(
        la.size.width >= 80.0,
        "panel width {} should be >= 80.0",
        la.size.width
    );
}

#[test]
fn cross_axis_constraints_default_to_none() {
    // Standard layout with no cross-axis constraints behaves identically
    let mut tree = LayoutTree::new();
    let (pa, a) = tree.add_panel("a", grow(1.0)).unwrap();
    let (pb, b) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![a, b]).unwrap();
    tree.set_root(root);

    let mut result = compile(&tree).unwrap();
    compute_layout(&mut result, 400.0, 200.0).unwrap();

    let la = panel_layout(&result, &tree, pa).unwrap();
    assert_eq!(la.size.width, 200.0);
    assert_eq!(la.size.height, 200.0);

    let lb = panel_layout(&result, &tree, pb).unwrap();
    assert_eq!(lb.size.width, 200.0);
    assert_eq!(lb.size.height, 200.0);
}

// --- Step 2: Alignment controls ---

#[test]
fn align_center_with_fixed_size() {
    // Row with a fixed-size panel aligned to center — should not stretch
    let mut tree = LayoutTree::new();
    let (pa, a) = tree
        .add_panel("a", fixed(100.0).align(Align::Center))
        .unwrap();
    let root = tree.add_row(0.0, vec![a]).unwrap();
    tree.set_root(root);

    let mut result = compile(&tree).unwrap();
    compute_layout(&mut result, 400.0, 400.0).unwrap();

    let la = panel_layout(&result, &tree, pa).unwrap();
    // Panel width is fixed at 100, not stretched to 400
    assert_eq!(la.size.width, 100.0);
    // Panel is centered vertically: y offset > 0
    assert!(
        la.location.y > 0.0,
        "centered panel y offset {} should be > 0",
        la.location.y
    );
}

#[test]
fn align_start_positions_at_origin() {
    // Col with a fixed-size panel aligned to Start — should sit at container x origin
    let mut tree = LayoutTree::new();
    let (pa, a) = tree
        .add_panel("a", fixed(50.0).align(Align::Start))
        .unwrap();
    let root = tree.add_col(0.0, vec![a]).unwrap();
    tree.set_root(root);

    let mut result = compile(&tree).unwrap();
    compute_layout(&mut result, 400.0, 400.0).unwrap();

    let la = panel_layout(&result, &tree, pa).unwrap();
    // Panel x == container x (no centering on the cross axis)
    assert_eq!(la.location.x, 0.0);
    // Panel should not stretch to fill width
    assert!(
        la.size.width < 400.0,
        "start-aligned panel width {} should not stretch to 400",
        la.size.width
    );
}

#[test]
fn align_defaults_to_stretch() {
    // No align set — panel fills its container (existing behavior)
    let mut tree = LayoutTree::new();
    let (pa, a) = tree.add_panel("a", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![a]).unwrap();
    tree.set_root(root);

    let mut result = compile(&tree).unwrap();
    compute_layout(&mut result, 400.0, 400.0).unwrap();

    let la = panel_layout(&result, &tree, pa).unwrap();
    assert_eq!(la.size.width, 400.0);
    assert_eq!(la.size.height, 400.0);
}

// --- Step 3: Intrinsic panel sizes ---

#[test]
fn set_panel_size_overrides_grow() {
    let mut rt = Layout::master_stack(["a", "b", "c"])
        .into_runtime()
        .unwrap();
    let pb = rt.sequence().get(1).unwrap();

    rt.set_panel_size(pb, 100.0, 50.0).unwrap();
    let frame = rt.resolve(400.0, 400.0).unwrap();

    let rb = frame.layout().get(pb).unwrap();
    assert!(
        (rb.w - 100.0).abs() < 1.0,
        "panel width {} should be ~100",
        rb.w
    );
    assert!(
        (rb.h - 50.0).abs() < 1.0,
        "panel height {} should be ~50",
        rb.h
    );
}

#[test]
fn clear_panel_size_reverts_to_constraints() {
    let mut rt = Layout::master_stack(["a", "b", "c"])
        .into_runtime()
        .unwrap();
    let pb = rt.sequence().get(1).unwrap();

    // Resolve without intrinsic size to get baseline
    let baseline = rt.resolve(400.0, 400.0).unwrap();
    let baseline_rect = *baseline.layout().get(pb).unwrap();

    // Set intrinsic size, resolve
    rt.set_panel_size(pb, 100.0, 50.0).unwrap();
    let _ = rt.resolve(400.0, 400.0).unwrap();

    // Clear and resolve again — should match baseline
    rt.clear_panel_size(pb).unwrap();
    let reverted = rt.resolve(400.0, 400.0).unwrap();
    let reverted_rect = reverted.layout().get(pb).unwrap();

    assert!(
        (reverted_rect.w - baseline_rect.w).abs() < 1.0,
        "reverted width {} should match baseline {}",
        reverted_rect.w,
        baseline_rect.w
    );
    assert!(
        (reverted_rect.h - baseline_rect.h).abs() < 1.0,
        "reverted height {} should match baseline {}",
        reverted_rect.h,
        baseline_rect.h
    );
}

#[test]
fn set_panel_size_with_max_height_clamps() {
    let mut tree = LayoutTree::new();
    let (pa, a) = tree.add_panel("a", grow(1.0).max_height(80.0)).unwrap();
    let (_, b) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_col(0.0, vec![a, b]).unwrap();
    tree.set_root(root);

    let mut rt = LayoutRuntime::new(tree);
    rt.set_panel_size(pa, 200.0, 200.0).unwrap();
    let frame = rt.resolve(400.0, 400.0).unwrap();

    let ra = frame.layout().get(pa).unwrap();
    assert!(
        ra.h <= 80.0,
        "panel height {} should be clamped to max_height 80.0",
        ra.h
    );
}

#[test]
fn set_panel_size_not_found_returns_error() {
    let mut rt = Layout::master_stack(["a", "b"]).into_runtime().unwrap();
    let bad_pid = panes::PanelId::from_raw(9999);

    let result = rt.set_panel_size(bad_pid, 100.0, 50.0);
    assert!(
        matches!(result, Err(PaneError::PanelNotFound(_))),
        "expected PanelNotFound, got {result:?}"
    );
}
