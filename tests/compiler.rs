use panes::compiler::{Axis, compile};
use panes::{Constraints, LayoutTree, PaneError, fixed, grow};

#[test]
fn grow_maps_to_flex_grow() {
    let style = panes::compiler::constraints_to_style(&grow(2.0), Axis::Horizontal);

    assert_eq!(style.flex_grow, 2.0);
    assert_eq!(style.flex_basis, taffy::Dimension::length(0.0));
    assert_eq!(style.flex_shrink, 1.0);
}

#[test]
fn fixed_maps_to_flex_basis() {
    let style = panes::compiler::constraints_to_style(&fixed(200.0), Axis::Horizontal);

    assert_eq!(style.flex_basis, taffy::Dimension::length(200.0));
    assert_eq!(style.flex_grow, 0.0);
    assert_eq!(style.flex_shrink, 0.0);
}

#[test]
fn grow_with_min_max_maps_correctly() {
    let c = grow(1.0).min(50.0).max(300.0);

    // Row parent → min/max on width
    let row_style = panes::compiler::constraints_to_style(&c, Axis::Horizontal);
    assert_eq!(row_style.min_size.width, taffy::Dimension::length(50.0));
    assert_eq!(row_style.max_size.width, taffy::Dimension::length(300.0));

    // Col parent → min/max on height
    let col_style = panes::compiler::constraints_to_style(&c, Axis::Vertical);
    assert_eq!(col_style.min_size.height, taffy::Dimension::length(50.0));
    assert_eq!(col_style.max_size.height, taffy::Dimension::length(300.0));
}

#[test]
fn fixed_with_min_maps_correctly() {
    let c = fixed(100.0).min(80.0);
    let style = panes::compiler::constraints_to_style(&c, Axis::Horizontal);

    assert_eq!(style.flex_basis, taffy::Dimension::length(100.0));
    assert_eq!(style.min_size.width, taffy::Dimension::length(80.0));
}

#[test]
fn default_constraints_map_to_grow_one() {
    let style = panes::compiler::constraints_to_style(&Constraints::default(), Axis::Horizontal);

    assert_eq!(style.flex_grow, 1.0);
    assert_eq!(style.flex_basis, taffy::Dimension::length(0.0));
    assert_eq!(style.flex_shrink, 1.0);
}

// --- Step 2: Tree Compilation ---

fn build_single_panel_tree() -> LayoutTree {
    let mut tree = LayoutTree::new();
    let (_, nid) = tree.add_panel("a", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![nid]).unwrap();
    tree.set_root(root);
    tree
}

#[test]
fn compile_single_panel_tree() {
    let tree = build_single_panel_tree();
    let result = compile(&tree).unwrap();

    // Root exists in taffy tree
    let root_style = result.taffy_tree.style(result.root).unwrap();
    assert_eq!(root_style.flex_direction, taffy::FlexDirection::Row);

    // Panel node mapped with correct flex_grow
    let taffy_children = result.taffy_tree.children(result.root).unwrap();
    assert_eq!(taffy_children.len(), 1);

    let child_style = result.taffy_tree.style(taffy_children[0]).unwrap();
    assert_eq!(child_style.flex_grow, 1.0);
}

#[test]
fn compile_row_with_two_panels() {
    let mut tree = LayoutTree::new();
    let (_, a) = tree.add_panel("a", grow(1.0)).unwrap();
    let (_, b) = tree.add_panel("b", grow(2.0)).unwrap();
    let root = tree.add_row(8.0, vec![a, b]).unwrap();
    tree.set_root(root);

    let result = compile(&tree).unwrap();

    let root_style = result.taffy_tree.style(result.root).unwrap();
    assert_eq!(root_style.flex_direction, taffy::FlexDirection::Row);
    assert_eq!(root_style.gap.width, taffy::LengthPercentage::length(8.0));

    let children = result.taffy_tree.children(result.root).unwrap();
    assert_eq!(children.len(), 2);

    let s0 = result.taffy_tree.style(children[0]).unwrap();
    assert_eq!(s0.flex_grow, 1.0);

    let s1 = result.taffy_tree.style(children[1]).unwrap();
    assert_eq!(s1.flex_grow, 2.0);
}

#[test]
fn compile_col_with_gap() {
    let mut tree = LayoutTree::new();
    let (_, x) = tree.add_panel("x", fixed(100.0)).unwrap();
    let (_, y) = tree.add_panel("y", grow(1.0)).unwrap();
    let root = tree.add_col(4.0, vec![x, y]).unwrap();
    tree.set_root(root);

    let result = compile(&tree).unwrap();

    let root_style = result.taffy_tree.style(result.root).unwrap();
    assert_eq!(root_style.flex_direction, taffy::FlexDirection::Column);
    assert_eq!(root_style.gap.height, taffy::LengthPercentage::length(4.0));

    let children = result.taffy_tree.children(result.root).unwrap();
    let s0 = result.taffy_tree.style(children[0]).unwrap();
    assert_eq!(s0.flex_basis, taffy::Dimension::length(100.0));
    assert_eq!(s0.flex_grow, 0.0);

    let s1 = result.taffy_tree.style(children[1]).unwrap();
    assert_eq!(s1.flex_grow, 1.0);
}

#[test]
fn compile_nested_row_col() {
    let mut tree = LayoutTree::new();
    let (_, editor) = tree.add_panel("editor", grow(2.0)).unwrap();
    let (_, chat) = tree.add_panel("chat", grow(1.0)).unwrap();
    let (_, status) = tree.add_panel("status", grow(1.0)).unwrap();
    let col = tree.add_col(0.0, vec![chat, status]).unwrap();
    let root = tree.add_row(0.0, vec![editor, col]).unwrap();
    tree.set_root(root);

    let result = compile(&tree).unwrap();

    // Root is Row with 2 taffy children
    let root_style = result.taffy_tree.style(result.root).unwrap();
    assert_eq!(root_style.flex_direction, taffy::FlexDirection::Row);

    let root_children = result.taffy_tree.children(result.root).unwrap();
    assert_eq!(root_children.len(), 2);

    // Second child is Column with 2 taffy children
    let col_style = result.taffy_tree.style(root_children[1]).unwrap();
    assert_eq!(col_style.flex_direction, taffy::FlexDirection::Column);

    let col_children = result.taffy_tree.children(root_children[1]).unwrap();
    assert_eq!(col_children.len(), 2);
}

#[test]
fn compile_taffy_passthrough() {
    let mut tree = LayoutTree::new();
    let custom_style = taffy::Style {
        flex_grow: 5.0,
        ..Default::default()
    };
    let (_, panel) = tree.add_panel("inner", grow(1.0)).unwrap();
    let passthrough = tree.add_taffy_node(custom_style, vec![panel]).unwrap();
    let root = tree.add_row(0.0, vec![passthrough]).unwrap();
    tree.set_root(root);

    let result = compile(&tree).unwrap();

    let root_children = result.taffy_tree.children(result.root).unwrap();
    let pt_style = result.taffy_tree.style(root_children[0]).unwrap();
    assert_eq!(pt_style.flex_grow, 5.0);
}

#[test]
fn compile_empty_tree_returns_error() {
    let tree = LayoutTree::new();
    assert!(compile(&tree).is_err());
}

#[test]
fn compile_min_max_axis_awareness() {
    // Row parent → min/max on width
    let mut row_tree = LayoutTree::new();
    let (_, a) = row_tree
        .add_panel("a", grow(1.0).min(50.0).max(300.0))
        .unwrap();
    let root = row_tree.add_row(0.0, vec![a]).unwrap();
    row_tree.set_root(root);

    let result = compile(&row_tree).unwrap();
    let children = result.taffy_tree.children(result.root).unwrap();
    let style = result.taffy_tree.style(children[0]).unwrap();
    assert_eq!(style.min_size.width, taffy::Dimension::length(50.0));
    assert_eq!(style.max_size.width, taffy::Dimension::length(300.0));

    // Col parent → min/max on height
    let mut col_tree = LayoutTree::new();
    let (_, b) = col_tree
        .add_panel("b", grow(1.0).min(50.0).max(300.0))
        .unwrap();
    let root = col_tree.add_col(0.0, vec![b]).unwrap();
    col_tree.set_root(root);

    let result = compile(&col_tree).unwrap();
    let children = result.taffy_tree.children(result.root).unwrap();
    let style = result.taffy_tree.style(children[0]).unwrap();
    assert_eq!(style.min_size.height, taffy::Dimension::length(50.0));
    assert_eq!(style.max_size.height, taffy::Dimension::length(300.0));
}

// --- Step 3: End-to-End Compilation and Compute ---

use panes::compiler::{compute_layout, panel_layout};

#[test]
fn two_equal_grow_panels_split_width() {
    let mut tree = LayoutTree::new();
    let (pa, a) = tree.add_panel("a", grow(1.0)).unwrap();
    let (pb, b) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![a, b]).unwrap();
    tree.set_root(root);

    let mut result = compile(&tree).unwrap();
    compute_layout(&mut result, 80.0, 24.0).unwrap();

    let la = panel_layout(&result, &tree, pa).unwrap();
    assert_eq!(la.location.x, 0.0);
    assert_eq!(la.size.width, 40.0);
    assert_eq!(la.size.height, 24.0);

    let lb = panel_layout(&result, &tree, pb).unwrap();
    assert_eq!(lb.location.x, 40.0);
    assert_eq!(lb.size.width, 40.0);
    assert_eq!(lb.size.height, 24.0);
}

#[test]
fn grow_ratio_distributes_proportionally() {
    let mut tree = LayoutTree::new();
    let (pa, a) = tree.add_panel("a", grow(2.0)).unwrap();
    let (pb, b) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![a, b]).unwrap();
    tree.set_root(root);

    let mut result = compile(&tree).unwrap();
    compute_layout(&mut result, 90.0, 30.0).unwrap();

    let la = panel_layout(&result, &tree, pa).unwrap();
    assert!((la.size.width - 60.0).abs() < 0.1);

    let lb = panel_layout(&result, &tree, pb).unwrap();
    assert!((lb.size.width - 30.0).abs() < 0.1);
}

#[test]
fn fixed_panel_takes_exact_size() {
    let mut tree = LayoutTree::new();
    let (ps, s) = tree.add_panel("side", fixed(20.0)).unwrap();
    let (pm, m) = tree.add_panel("main", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![s, m]).unwrap();
    tree.set_root(root);

    let mut result = compile(&tree).unwrap();
    compute_layout(&mut result, 100.0, 50.0).unwrap();

    let ls = panel_layout(&result, &tree, ps).unwrap();
    assert_eq!(ls.size.width, 20.0);

    let lm = panel_layout(&result, &tree, pm).unwrap();
    assert_eq!(lm.size.width, 80.0);
}

#[test]
fn gap_reduces_available_space() {
    let mut tree = LayoutTree::new();
    let (pa, a) = tree.add_panel("a", grow(1.0)).unwrap();
    let (pb, b) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(10.0, vec![a, b]).unwrap();
    tree.set_root(root);

    let mut result = compile(&tree).unwrap();
    compute_layout(&mut result, 100.0, 50.0).unwrap();

    let la = panel_layout(&result, &tree, pa).unwrap();
    assert_eq!(la.size.width, 45.0);

    let lb = panel_layout(&result, &tree, pb).unwrap();
    assert_eq!(lb.size.width, 45.0);
}

#[test]
fn nested_layout_computes_correctly() {
    let mut tree = LayoutTree::new();
    let (pe, editor) = tree.add_panel("editor", grow(2.0)).unwrap();
    let (pc, chat) = tree.add_panel("chat", grow(1.0)).unwrap();
    let (ps, status) = tree.add_panel("status", grow(1.0)).unwrap();
    let col = tree.add_col(0.0, vec![chat, status]).unwrap();
    let root = tree.add_row(0.0, vec![editor, col]).unwrap();
    tree.set_root(root);

    let mut result = compile(&tree).unwrap();
    compute_layout(&mut result, 90.0, 24.0).unwrap();

    let le = panel_layout(&result, &tree, pe).unwrap();
    assert_eq!(le.size.width, 60.0);

    let lc = panel_layout(&result, &tree, pc).unwrap();
    assert_eq!(lc.size.height, 12.0);

    let ls = panel_layout(&result, &tree, ps).unwrap();
    assert_eq!(ls.size.height, 12.0);
}

#[test]
fn min_constraint_enforced() {
    let mut tree = LayoutTree::new();
    let (pa, a) = tree.add_panel("a", grow(1.0).min(60.0)).unwrap();
    let (_pb, b) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![a, b]).unwrap();
    tree.set_root(root);

    let mut result = compile(&tree).unwrap();
    compute_layout(&mut result, 100.0, 50.0).unwrap();

    let la = panel_layout(&result, &tree, pa).unwrap();
    assert!(la.size.width >= 60.0);
}

#[test]
fn max_constraint_enforced() {
    let mut tree = LayoutTree::new();
    let (pa, a) = tree.add_panel("a", grow(1.0).max(30.0)).unwrap();
    let (_pb, b) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![a, b]).unwrap();
    tree.set_root(root);

    let mut result = compile(&tree).unwrap();
    compute_layout(&mut result, 100.0, 50.0).unwrap();

    let la = panel_layout(&result, &tree, pa).unwrap();
    assert!(la.size.width <= 30.0);
}

// --- Viewport validation ---

#[test]
fn compute_layout_rejects_nan_dimensions() {
    let mut tree = LayoutTree::new();
    let (_, a) = tree.add_panel("a", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![a]).unwrap();
    tree.set_root(root);

    let mut result = compile(&tree).unwrap();
    let err = compute_layout(&mut result, f32::NAN, 24.0).unwrap_err();
    assert!(matches!(err, PaneError::InvalidViewport(_)));
}

// --- Layout::compile() convenience ---

#[test]
fn layout_compile_returns_valid_result() {
    use panes::Layout;

    let layout = Layout::row(["a", "b", "c"]).unwrap();
    let result = layout.compile().unwrap();

    let children = result.taffy_tree.children(result.root).unwrap();
    assert_eq!(children.len(), 3);
}
