mod helpers;

use std::sync::Arc;

use helpers::build_row_tree;
use panes::runtime::LayoutRuntime;
use panes::{CardSpan, Layout, StrategyKind, fixed, grow};

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
fn resize_with_fixed_sibling() {
    let mut tree = panes::LayoutTree::new();
    let (p0, n0) = tree.add_panel("a", grow(1.0)).unwrap();
    let (p1, n1) = tree.add_panel("b", fixed(100.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(p0, 50.0).unwrap();

    let c1 = rt.tree().panel_constraints(p1).unwrap();
    assert!(
        (c1.fixed.unwrap() - 50.0).abs() < 0.01,
        "expected fixed ~50.0, got {}",
        c1.fixed.unwrap()
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

// --- Container-aware tests ---

fn taffy_col_style(flex_grow: f32) -> taffy::Style {
    taffy::Style {
        flex_direction: taffy::FlexDirection::Column,
        flex_grow,
        flex_basis: taffy::Dimension::length(0.0),
        flex_shrink: 1.0,
        ..Default::default()
    }
}

#[test]
fn panel_and_container_sibling() {
    let mut tree = panes::LayoutTree::new();
    let (p0, n0) = tree.add_panel("a", grow(0.6)).unwrap();
    let (_, np) = tree.add_panel("b", grow(1.0)).unwrap();
    let container = tree.add_taffy_node(taffy_col_style(0.4), vec![np]).unwrap();
    let root = tree.add_row(0.0, vec![n0, container]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(p0, 0.1).unwrap();

    let c0 = rt.tree().panel_constraints(p0).unwrap();
    assert!(
        (c0.grow.unwrap() - 0.7).abs() < 0.01,
        "expected ~0.7, got {}",
        c0.grow.unwrap()
    );
}

#[test]
fn centered_master_resize() {
    let mut tree = panes::LayoutTree::new();
    let left_child = tree.add_panel("left_inner", grow(1.0)).unwrap().1;
    let left = tree
        .add_taffy_node(taffy_col_style(0.25), vec![left_child])
        .unwrap();
    let (center_pid, center) = tree.add_panel("center", grow(0.5)).unwrap();
    let right_child = tree.add_panel("right_inner", grow(1.0)).unwrap().1;
    let right = tree
        .add_taffy_node(taffy_col_style(0.25), vec![right_child])
        .unwrap();
    let root = tree.add_row(0.0, vec![left, center, right]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(center_pid, 0.1).unwrap();

    let frame = rt.resolve(1000.0, 100.0).unwrap();
    let cw = frame.layout().get(center_pid).unwrap().w;
    let total = 1000.0;
    let ratio = cw / total;
    assert!(
        (ratio - 0.6).abs() < 0.02,
        "expected center ~0.6, got {ratio}"
    );
}

#[test]
fn skip_zero_flex_grow_decoration() {
    let mut tree = panes::LayoutTree::new();
    // Tab bar decoration with zero flex_grow
    let tab_bar = tree.add_taffy_node(taffy_col_style(0.0), vec![]).unwrap();
    let (p0, n0) = tree.add_panel("a", grow(1.0)).unwrap();
    let (p1, n1) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_col(0.0, vec![tab_bar, n0, n1]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(p0, 0.1).unwrap();

    let c0 = rt.tree().panel_constraints(p0).unwrap();
    let c1 = rt.tree().panel_constraints(p1).unwrap();
    assert!(c0.grow.unwrap() > c1.grow.unwrap());
}

#[test]
fn mixed_fixed_grow_container_resize() {
    let mut tree = panes::LayoutTree::new();
    let (p0, n0) = tree.add_panel("a", grow(0.5)).unwrap();
    let (p_fixed, n_fixed) = tree.add_panel("b", fixed(100.0)).unwrap();
    let inner = tree.add_panel("c", grow(1.0)).unwrap().1;
    let container = tree
        .add_taffy_node(taffy_col_style(0.5), vec![inner])
        .unwrap();
    let root = tree.add_row(0.0, vec![n0, n_fixed, container]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    // Grow panel resizes → nearest fixed neighbor shrinks
    rt.resize_boundary(p0, 30.0).unwrap();

    let cf = rt.tree().panel_constraints(p_fixed).unwrap();
    assert!(
        (cf.fixed.unwrap() - 70.0).abs() < 0.01,
        "expected fixed ~70.0, got {}",
        cf.fixed.unwrap()
    );
}

#[test]
fn insufficient_resizable_after_filter() {
    let mut tree = panes::LayoutTree::new();
    let tab_bar = tree.add_taffy_node(taffy_col_style(0.0), vec![]).unwrap();
    let (p0, n0) = tree.add_panel("a", grow(1.0)).unwrap();
    let root = tree.add_col(0.0, vec![tab_bar, n0]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    let err = rt.resize_boundary(p0, 0.1).unwrap_err();
    assert!(
        err.to_string().contains("only child"),
        "unexpected error: {err}"
    );
}

#[test]
fn master_stack_resize() {
    let mut rt = Layout::master_stack(["editor", "chat", "status"])
        .into_runtime()
        .unwrap();
    let master_pid = rt.tree().panels_by_kind("editor")[0];

    rt.resize_boundary(master_pid, 0.1).unwrap();

    let frame = rt.resolve(1000.0, 800.0).unwrap();
    let master_w = frame.layout().get(master_pid).unwrap().w;
    let ratio = master_w / 1000.0;
    // Default master_ratio is 0.5, after +0.1 should be ~0.6
    assert!(
        (ratio - 0.6).abs() < 0.05,
        "expected master ~0.6, got {ratio}"
    );
}

#[test]
fn deck_resize() {
    let mut rt = Layout::deck(["editor", "chat", "status"])
        .into_runtime()
        .unwrap();
    let master_pid = rt.tree().panels_by_kind("editor")[0];

    rt.resize_boundary(master_pid, 0.1).unwrap();

    let frame = rt.resolve(1000.0, 800.0).unwrap();
    let master_w = frame.layout().get(master_pid).unwrap().w;
    let ratio = master_w / 1000.0;
    assert!(
        (ratio - 0.6).abs() < 0.05,
        "expected master ~0.6, got {ratio}"
    );
}

// --- Fixed-value resize tests ---

#[test]
fn sidebar_resize_content() {
    let mut tree = panes::LayoutTree::new();
    let (_, n0) = tree.add_panel("sidebar", fixed(200.0)).unwrap();
    let (p1, n1) = tree.add_panel("content", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    // Growing content shrinks the nearest fixed neighbor (sidebar)
    rt.resize_boundary(p1, 50.0).unwrap();

    let c0 = rt
        .tree()
        .panel_constraints(rt.tree().panels_by_kind("sidebar")[0])
        .unwrap();
    assert!(
        (c0.fixed.unwrap() - 150.0).abs() < 0.01,
        "expected fixed ~150.0, got {}",
        c0.fixed.unwrap()
    );
}

#[test]
fn sidebar_resize_sidebar() {
    let mut tree = panes::LayoutTree::new();
    let (p0, n0) = tree.add_panel("sidebar", fixed(200.0)).unwrap();
    let (_, n1) = tree.add_panel("content", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(p0, 50.0).unwrap();

    let c0 = rt.tree().panel_constraints(p0).unwrap();
    assert!(
        (c0.fixed.unwrap() - 250.0).abs() < 0.01,
        "expected fixed ~250.0, got {}",
        c0.fixed.unwrap()
    );
}

#[test]
fn holy_grail_resize_header() {
    let mut tree = panes::LayoutTree::new();
    let (p_header, n_header) = tree.add_panel("header", fixed(50.0)).unwrap();
    let inner = tree.add_panel("main", grow(1.0)).unwrap().1;
    let middle = tree
        .add_taffy_node(taffy_col_style(1.0), vec![inner])
        .unwrap();
    let (_, n_footer) = tree.add_panel("footer", fixed(30.0)).unwrap();
    let root = tree.add_col(0.0, vec![n_header, middle, n_footer]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(p_header, 20.0).unwrap();

    let ch = rt.tree().panel_constraints(p_header).unwrap();
    assert!(
        (ch.fixed.unwrap() - 70.0).abs() < 0.01,
        "expected fixed ~70.0, got {}",
        ch.fixed.unwrap()
    );
}

#[test]
fn holy_grail_resize_main() {
    let mut tree = panes::LayoutTree::new();
    let (p_left, n_left) = tree.add_panel("left", fixed(200.0)).unwrap();
    let (p_main, n_main) = tree.add_panel("main", grow(1.0)).unwrap();
    let (_, n_right) = tree.add_panel("right", fixed(200.0)).unwrap();
    let root = tree.add_row(0.0, vec![n_left, n_main, n_right]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    // Growing main → nearest fixed before (left) shrinks
    rt.resize_boundary(p_main, 50.0).unwrap();

    let cl = rt.tree().panel_constraints(p_left).unwrap();
    assert!(
        (cl.fixed.unwrap() - 150.0).abs() < 0.01,
        "expected left fixed ~150.0, got {}",
        cl.fixed.unwrap()
    );
}

#[test]
fn holy_grail_resize_left_sidebar() {
    let mut tree = panes::LayoutTree::new();
    let (p_left, n_left) = tree.add_panel("left", fixed(200.0)).unwrap();
    let (_, n_main) = tree.add_panel("main", grow(1.0)).unwrap();
    let (_, n_right) = tree.add_panel("right", fixed(200.0)).unwrap();
    let root = tree.add_row(0.0, vec![n_left, n_main, n_right]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(p_left, -30.0).unwrap();

    let cl = rt.tree().panel_constraints(p_left).unwrap();
    assert!(
        (cl.fixed.unwrap() - 170.0).abs() < 0.01,
        "expected left fixed ~170.0, got {}",
        cl.fixed.unwrap()
    );
}

#[test]
fn fixed_clamp_min() {
    let mut tree = panes::LayoutTree::new();
    let (p0, n0) = tree.add_panel("a", fixed(200.0).min(100.0)).unwrap();
    let (_, n1) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(p0, -150.0).unwrap();

    let c0 = rt.tree().panel_constraints(p0).unwrap();
    assert!(
        (c0.fixed.unwrap() - 100.0).abs() < 0.01,
        "expected clamped to min 100.0, got {}",
        c0.fixed.unwrap()
    );
}

#[test]
fn fixed_clamp_floor() {
    let mut tree = panes::LayoutTree::new();
    let (p0, n0) = tree.add_panel("a", fixed(50.0)).unwrap();
    let (_, n1) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(p0, -200.0).unwrap();

    let c0 = rt.tree().panel_constraints(p0).unwrap();
    assert!(
        (c0.fixed.unwrap() - 1.0).abs() < 0.01,
        "expected clamped to floor 1.0, got {}",
        c0.fixed.unwrap()
    );
}

#[test]
fn fixed_preserves_min_max() {
    let mut tree = panes::LayoutTree::new();
    let (p0, n0) = tree
        .add_panel("a", fixed(200.0).min(50.0).max(500.0))
        .unwrap();
    let (_, n1) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(p0, 30.0).unwrap();

    let c0 = rt.tree().panel_constraints(p0).unwrap();
    assert_eq!(c0.min, Some(50.0));
    assert_eq!(c0.max, Some(500.0));
    assert!(
        (c0.fixed.unwrap() - 230.0).abs() < 0.01,
        "expected fixed ~230.0, got {}",
        c0.fixed.unwrap()
    );
}

#[test]
fn sidebar_preset_resize() {
    let mut rt = Layout::sidebar("nav", "main")
        .sidebar_width(200.0)
        .into_runtime()
        .unwrap();
    let nav_pid = rt.tree().panels_by_kind("nav")[0];
    let main_pid = rt.tree().panels_by_kind("main")[0];

    rt.resize_boundary(main_pid, 50.0).unwrap();

    let cn = rt.tree().panel_constraints(nav_pid).unwrap();
    assert!(
        (cn.fixed.unwrap() - 150.0).abs() < 0.01,
        "expected sidebar ~150.0, got {}",
        cn.fixed.unwrap()
    );
}

#[test]
fn holy_grail_preset_resize() {
    let mut rt = Layout::holy_grail("header", "footer", "left", "main", "right")
        .header_height(50.0)
        .footer_height(30.0)
        .sidebar_width(200.0)
        .into_runtime()
        .unwrap();
    let header_pid = rt.tree().panels_by_kind("header")[0];

    rt.resize_boundary(header_pid, 20.0).unwrap();

    let ch = rt.tree().panel_constraints(header_pid).unwrap();
    assert!(
        (ch.fixed.unwrap() - 70.0).abs() < 0.01,
        "expected header ~70.0, got {}",
        ch.fixed.unwrap()
    );
}

#[test]
fn stack_panel_resizes_container_against_master() {
    let mut rt = Layout::master_stack(["editor", "chat", "status"])
        .into_runtime()
        .unwrap();
    let chat_pid = rt.tree().panels_by_kind("chat")[0];
    let master_pid = rt.tree().panels_by_kind("editor")[0];

    rt.resize_boundary(chat_pid, 0.1).unwrap();

    let frame = rt.resolve(1000.0, 800.0).unwrap();
    let master_w = frame.layout().get(master_pid).unwrap().w;
    let ratio = master_w / 1000.0;
    // Default master_ratio=0.5, after stack panel grows by +0.1 → master shrinks to ~0.4
    assert!(
        ratio < 0.5,
        "master should be narrower than default 0.5, got {ratio}"
    );
    assert!(
        (ratio - 0.4).abs() < 0.05,
        "expected master ~0.4, got {ratio}"
    );
}

#[test]
fn stack_panel_resizes_container_negative_delta() {
    let mut rt = Layout::master_stack(["editor", "chat", "status"])
        .into_runtime()
        .unwrap();
    let chat_pid = rt.tree().panels_by_kind("chat")[0];
    let master_pid = rt.tree().panels_by_kind("editor")[0];

    rt.resize_boundary(chat_pid, -0.1).unwrap();

    let frame = rt.resolve(1000.0, 800.0).unwrap();
    let master_w = frame.layout().get(master_pid).unwrap().w;
    let ratio = master_w / 1000.0;
    // Stack container shrinks → master grows to ~0.6
    assert!(
        ratio > 0.5,
        "master should be wider than default 0.5, got {ratio}"
    );
    assert!(
        (ratio - 0.6).abs() < 0.05,
        "expected master ~0.6, got {ratio}"
    );
}

#[test]
fn deck_panel_resizes_container_against_master() {
    let mut rt = Layout::deck(["editor", "chat", "status"])
        .into_runtime()
        .unwrap();
    let chat_pid = rt.tree().panels_by_kind("chat")[0];
    let master_pid = rt.tree().panels_by_kind("editor")[0];

    rt.resize_boundary(chat_pid, 0.1).unwrap();

    let frame = rt.resolve(1000.0, 800.0).unwrap();
    let master_w = frame.layout().get(master_pid).unwrap().w;
    let ratio = master_w / 1000.0;
    assert!(
        ratio < 0.5,
        "master should be narrower than default 0.5, got {ratio}"
    );
    assert!(
        (ratio - 0.4).abs() < 0.05,
        "expected master ~0.4, got {ratio}"
    );
}

#[test]
fn centered_master_side_panel_resizes_container() {
    let mut rt = Layout::centered_master(["center", "left", "right"])
        .into_runtime()
        .unwrap();
    let left_pid = rt.tree().panels_by_kind("left")[0];

    rt.resize_boundary(left_pid, 0.1).unwrap();

    let frame = rt.resolve(1000.0, 800.0).unwrap();
    let left_w = frame.layout().get(left_pid).unwrap().w;
    let default_side_ratio = 0.25; // (1.0 - 0.5) / 2
    let default_left_w = 1000.0 * default_side_ratio;
    assert!(
        left_w > default_left_w,
        "left container should have grown: expected > {default_left_w}, got {left_w}"
    );
}

#[test]
fn stack_vertical_resize_stays_within_container() {
    // Col root with a TaffyPassthrough(Column) child → same axis → no escalation.
    // Resize should adjust chat vs status within the container, leaving master unchanged.
    let mut tree = panes::LayoutTree::new();
    let (master_pid, n_master) = tree.add_panel("editor", grow(0.5)).unwrap();
    let (chat_pid, n_chat) = tree.add_panel("chat", grow(1.0)).unwrap();
    let (status_pid, n_status) = tree.add_panel("status", grow(1.0)).unwrap();
    let container = tree
        .add_taffy_node(taffy_col_style(0.5), vec![n_chat, n_status])
        .unwrap();
    let root = tree.add_col(0.0, vec![n_master, container]).unwrap();
    tree.set_root(root);

    // Verify both stack panels have grow constraints
    let chat_c = tree.panel_constraints(chat_pid).unwrap();
    let status_c = tree.panel_constraints(status_pid).unwrap();
    assert!(chat_c.grow.is_some(), "chat should have grow constraint");
    assert!(
        status_c.grow.is_some(),
        "status should have grow constraint"
    );

    let master_before = tree.panel_constraints(master_pid).unwrap();
    let mut rt = LayoutRuntime::new(tree);

    rt.resize_boundary(chat_pid, 0.1).unwrap();

    // Master should be unchanged (no escalation)
    let master_after = rt.tree().panel_constraints(master_pid).unwrap();
    assert_eq!(
        master_before, master_after,
        "master constraints should be unchanged"
    );

    // Chat should have grown relative to status
    let chat_grow = rt.tree().panel_constraints(chat_pid).unwrap().grow.unwrap();
    let status_grow = rt
        .tree()
        .panel_constraints(status_pid)
        .unwrap()
        .grow
        .unwrap();
    assert!(
        chat_grow > status_grow,
        "chat should have grown relative to status: chat={chat_grow}, status={status_grow}"
    );
}

#[test]
fn dashboard_resize_boundary_error() {
    let kinds: Vec<Arc<str>> = (0..4).map(|i| Arc::from(format!("p{i}"))).collect();
    let spans: Arc<[CardSpan]> = vec![CardSpan::Columns(1); 4].into();
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::Dashboard {
            columns: 2,
            gap: 0.0,
            spans,
        },
        &kinds,
    )
    .unwrap();
    let p0 = rt.sequence().get(0).unwrap();

    let err = rt.resize_boundary(p0, 0.1).unwrap_err();
    assert!(
        err.to_string().contains("only child"),
        "dashboard resize should fail with OnlyChild, got: {err}"
    );
}
