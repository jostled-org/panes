#![allow(clippy::unwrap_used, clippy::panic)]
mod helpers;

use std::sync::Arc;

use helpers::build_row_tree;
use panes::runtime::LayoutRuntime;
use panes::{
    Constraints, FocusOutcome, FocusRejection, Grid, Layout, LayoutBuilder, PanelId, fixed, grow,
};

#[test]
fn runtime_first_resolve_all_added() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    let frame = rt.resolve(100.0, 100.0).unwrap();

    // First frame: all panels should be added
    assert_eq!(rt.last_diff().added.len(), 2);
    assert!(rt.last_diff().added.contains(&pids[0]));
    assert!(rt.last_diff().added.contains(&pids[1]));
    assert!(rt.last_diff().removed.is_empty());
    assert!(rt.last_diff().moved.is_empty());
    assert!(rt.last_diff().resized.is_empty());
    assert!(rt.last_diff().unchanged.is_empty());

    // Layout rects should be correct
    let r0 = frame.layout().get(pids[0]).unwrap();
    assert_eq!(r0.w, 50.0);
    assert_eq!(r0.h, 100.0);
}

#[test]
fn runtime_second_resolve_no_changes() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    let _ = rt.resolve(100.0, 100.0).unwrap();
    let _frame = rt.resolve(100.0, 100.0).unwrap();

    // Second frame at same dimensions: all unchanged
    assert!(rt.last_diff().added.is_empty());
    assert!(rt.last_diff().removed.is_empty());
    assert!(rt.last_diff().moved.is_empty());
    assert!(rt.last_diff().resized.is_empty());
    assert_eq!(rt.last_diff().unchanged.len(), 2);
    assert!(rt.last_diff().unchanged.contains(&pids[0]));
    assert!(rt.last_diff().unchanged.contains(&pids[1]));
}

#[test]
fn runtime_resolve_different_size_shows_resize() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    let _ = rt.resolve(100.0, 100.0).unwrap();
    let _frame = rt.resolve(200.0, 100.0).unwrap();

    // Both panels resized (width changed from 50 to 100)
    assert_eq!(rt.last_diff().resized.len(), 2);

    // Second panel also moved (its x position changed)
    assert_eq!(rt.last_diff().moved.len(), 1);
    let moved_ids: Vec<PanelId> = rt.last_diff().moved.iter().map(|c| c.id).collect();
    assert!(moved_ids.contains(&pids[1]));
}

#[test]
fn runtime_remove_panel_in_diff() {
    let (tree, pids) = build_row_tree(3, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    let _ = rt.resolve(90.0, 100.0).unwrap();

    // Remove middle panel
    rt.tree_mut().remove_panel(pids[1]).unwrap();
    let _frame = rt.resolve(90.0, 100.0).unwrap();

    // Middle panel removed
    assert_eq!(rt.last_diff().removed.len(), 1);
    assert!(rt.last_diff().removed.contains(&pids[1]));

    // Remaining panels resized (grew from 30px to 45px)
    assert_eq!(rt.last_diff().resized.len(), 2);
}

#[test]
fn runtime_set_constraints_in_diff() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    let _ = rt.resolve(100.0, 100.0).unwrap();

    // Change one panel to grow(2)
    rt.tree_mut().set_constraints(pids[0], grow(2.0)).unwrap();
    let _frame = rt.resolve(100.0, 100.0).unwrap();

    // Both panels resized (proportions changed from 50/50 to ~67/33)
    assert_eq!(rt.last_diff().resized.len(), 2);
}

// --- Step 3 tests: Collapse, Scroll, Active ---

#[test]
fn collapse_panel_zero_size() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    let frame = rt.resolve(100.0, 100.0).unwrap();
    let r0 = frame.layout().get(pids[0]).unwrap();
    let r1 = frame.layout().get(pids[1]).unwrap();
    assert!((r0.w - 50.0).abs() < 0.1);
    assert!((r1.w - 50.0).abs() < 0.1);

    // Collapse the first panel
    rt.toggle_collapsed(pids[0]).unwrap();
    let frame = rt.resolve(100.0, 100.0).unwrap();

    let r0 = frame.layout().get(pids[0]).unwrap();
    let r1 = frame.layout().get(pids[1]).unwrap();
    assert!(
        r0.w < 0.1,
        "collapsed panel should have zero width, got {}",
        r0.w
    );
    assert!(
        (r1.w - 100.0).abs() < 0.1,
        "remaining panel should fill space, got {}",
        r1.w
    );
}

#[test]
fn uncollapse_restores_size() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    let _ = rt.resolve(100.0, 100.0).unwrap();

    // Collapse then uncollapse
    rt.toggle_collapsed(pids[0]).unwrap();
    let _ = rt.resolve(100.0, 100.0).unwrap();

    rt.toggle_collapsed(pids[0]).unwrap();
    let frame = rt.resolve(100.0, 100.0).unwrap();

    let r0 = frame.layout().get(pids[0]).unwrap();
    let r1 = frame.layout().get(pids[1]).unwrap();
    assert!(
        (r0.w - 50.0).abs() < 0.1,
        "uncollapsed panel should restore to 50px, got {}",
        r0.w
    );
    assert!(
        (r1.w - 50.0).abs() < 0.1,
        "other panel should restore to 50px, got {}",
        r1.w
    );
}

#[test]
fn uncollapse_failure_preserves_restore_state() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    rt.toggle_collapsed(pids[0]).unwrap();

    let saved = rt.viewport().saved_constraints.get(&pids[0]).copied();
    assert_eq!(saved, Some(grow(1.0)));
    assert!(rt.viewport().collapsed.contains(&pids[0]));

    rt.tree_mut().remove_panel(pids[0]).unwrap();

    let result = rt.toggle_collapsed(pids[0]);
    assert!(matches!(result, Err(panes::PaneError::PanelNotFound(pid)) if pid == pids[0]));

    assert_eq!(
        rt.viewport().saved_constraints.get(&pids[0]).copied(),
        saved
    );
    assert!(rt.viewport().collapsed.contains(&pids[0]));
}

#[test]
fn scroll_by_shifts_x() {
    let layout = Layout::split("a", "b").build().unwrap();
    let mut rt = LayoutRuntime::from(layout);

    let frame = rt.resolve(100.0, 100.0).unwrap();
    let a_pid = frame.layout().by_kind("a")[0];
    let base_x = frame.layout().get(a_pid).unwrap().x;

    rt.scroll_by(40.0).unwrap();
    let frame = rt.resolve(100.0, 100.0).unwrap();
    let new_x = frame.layout().get(a_pid).unwrap().x;
    assert!((new_x - (base_x - 40.0)).abs() < 0.1);
}

#[test]
fn scroll_to_absolute() {
    let layout = Layout::split("a", "b").build().unwrap();
    let mut rt = LayoutRuntime::from(layout);

    let frame = rt.resolve(100.0, 100.0).unwrap();
    let a_pid = frame.layout().by_kind("a")[0];
    let base_x = frame.layout().get(a_pid).unwrap().x;

    rt.scroll_to(80.0).unwrap();
    let frame = rt.resolve(100.0, 100.0).unwrap();
    let new_x = frame.layout().get(a_pid).unwrap().x;
    assert!((new_x - (base_x - 80.0)).abs() < 0.1);
}

#[test]
fn set_focus_unchecked_queryable() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    assert!(rt.focused().is_none());

    rt.set_focus_unchecked(pids[0]);
    assert_eq!(rt.focused(), Some(pids[0]));

    rt.set_focus_unchecked(pids[1]);
    assert_eq!(rt.focused(), Some(pids[1]));
}

#[test]
fn focus_rejects_missing_panel_without_strategy() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    assert_eq!(rt.focus(pids[0]), FocusOutcome::Applied);
    assert_eq!(rt.focused(), Some(pids[0]));

    let missing = PanelId::from_raw(999);
    assert_eq!(
        rt.focus(missing),
        FocusOutcome::Rejected(FocusRejection::PanelNotFound)
    );
    assert_eq!(rt.focused(), Some(pids[0]));
}

#[test]
fn cached_compile_reused_on_dimension_change() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    // First resolve compiles and caches
    let frame1 = rt.resolve(100.0, 100.0).unwrap();
    assert_eq!(frame1.layout().get(pids[0]).unwrap().w, 50.0);

    // Tree is not dirty — resolving at different dimensions reuses cached compile
    assert!(!rt.tree().is_dirty());
    let frame2 = rt.resolve(200.0, 100.0).unwrap();
    assert_eq!(frame2.layout().get(pids[0]).unwrap().w, 100.0);
    assert_eq!(frame2.layout().get(pids[1]).unwrap().w, 100.0);
}

#[test]
fn tree_mutation_invalidates_compile_cache() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    let _ = rt.resolve(100.0, 100.0).unwrap();

    // Mutate tree — dirty flag set, cache invalidated on next resolve
    rt.tree_mut().set_constraints(pids[0], fixed(30.0)).unwrap();
    assert!(rt.tree().is_dirty());

    let frame = rt.resolve(100.0, 100.0).unwrap();
    assert_eq!(frame.layout().get(pids[0]).unwrap().w, 30.0);
    assert_eq!(frame.layout().get(pids[1]).unwrap().w, 70.0);
}

#[test]
fn scroll_by_rejects_nan() {
    let layout = Layout::split("a", "b").build().unwrap();
    let mut rt = LayoutRuntime::from(layout);
    assert!(rt.scroll_by(f32::NAN).is_err());
}

#[test]
fn scroll_to_rejects_infinity() {
    let layout = Layout::split("a", "b").build().unwrap();
    let mut rt = LayoutRuntime::from(layout);
    assert!(rt.scroll_to(f32::INFINITY).is_err());
}

#[test]
fn set_window_panel_count_rejects_zero() {
    let mut tree = panes::LayoutTree::new();
    assert!(tree.set_window_panel_count(0).is_err());
    assert!(tree.set_window_panel_count(2).is_ok());
}

#[test]
fn add_overlay_rejects_nan_margin() {
    let mut rt = Layout::master_stack(["a", "b"]).into_runtime().unwrap();
    let result = rt.add_overlay("bad", panes::overlay::Overlay::top(f32::NAN));
    assert!(result.is_err());
}

#[test]
fn add_overlay_rejects_min_exceeds_max() {
    let mut rt = Layout::master_stack(["a", "b"]).into_runtime().unwrap();
    let result = rt.add_overlay(
        "bad",
        panes::overlay::Overlay::center().clamp_width(200.0, 100.0),
    );
    assert!(result.is_err());
}

#[test]
fn set_panel_size_rejects_negative_and_non_finite_values() {
    let (tree, pids) = build_row_tree(1, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    assert!(rt.set_panel_size(pids[0], -1.0, 10.0).is_err());
    assert!(rt.set_panel_size(pids[0], 10.0, f32::NAN).is_err());
    assert!(rt.set_panel_size(pids[0], f32::INFINITY, 10.0).is_err());
}

#[test]
fn insert_child_at_rejects_oob() {
    let (mut tree, _pids) = build_row_tree(2, grow(1.0));
    let root = tree.root().unwrap();
    let (_, new_nid) = tree.add_panel("extra", grow(1.0)).unwrap();
    // Root has 2 children; index 3 is out of bounds
    let result = tree.insert_child_at(root, 3, new_nid);
    assert!(result.is_err());
}

#[test]
fn set_panel_size_reuses_kind_cache_on_next_resolve() {
    let layout = Layout::split("left", "right").build().unwrap();
    let mut rt = LayoutRuntime::from(layout);

    // First resolve: establish baseline
    let frame1 = rt.resolve(100.0, 100.0).unwrap();
    let left_pid = frame1.layout().by_kind("left")[0];
    let right_pid = frame1.layout().by_kind("right")[0];
    assert_eq!(rt.last_diff().added.len(), 2);

    // Second resolve: baseline established, everything unchanged
    let _ = rt.resolve(100.0, 100.0).unwrap();
    assert_eq!(rt.last_diff().unchanged.len(), 2);

    // Mutate intrinsic size (constraint-only, no topology change)
    rt.set_panel_size(left_pid, 30.0, 100.0).unwrap();

    // Third resolve: geometry should change, kind lookup should still work
    let frame3 = rt.resolve(100.0, 100.0).unwrap();

    // Kind-oriented output still correct
    assert_eq!(frame3.layout().by_kind("left"), &[left_pid]);
    assert_eq!(frame3.layout().by_kind("right"), &[right_pid]);

    // Geometry changed: left panel got the intrinsic size
    let left_rect = frame3.layout().get(left_pid).unwrap();
    assert!(
        (left_rect.w - 30.0).abs() < 1.0,
        "left should be ~30px, got {}",
        left_rect.w
    );

    // Diff should show resized, not added/removed (same-panel path)
    let diff = rt.last_diff();
    assert!(diff.added.is_empty(), "no panels added");
    assert!(diff.removed.is_empty(), "no panels removed");
    assert!(!diff.resized.is_empty(), "panels should be resized");
}

#[test]
fn set_constraints_diff_path_preserves_same_panel_behavior() {
    let layout = Layout::split("left", "right").build().unwrap();
    let mut rt = LayoutRuntime::from(layout);

    // First resolve: establish baseline
    let frame1 = rt.resolve(100.0, 100.0).unwrap();
    let left_pid = frame1.layout().by_kind("left")[0];
    let right_pid = frame1.layout().by_kind("right")[0];

    // Second resolve: everything settled
    let _ = rt.resolve(100.0, 100.0).unwrap();
    assert_eq!(rt.last_diff().unchanged.len(), 2);

    // Update constraints through the runtime API (constraint-only, no topology change)
    rt.set_constraints(left_pid, grow(2.0)).unwrap();

    // Third resolve: geometry should change
    let frame3 = rt.resolve(100.0, 100.0).unwrap();

    // Kind lookups still correct
    assert_eq!(frame3.layout().by_kind("left"), &[left_pid]);
    assert_eq!(frame3.layout().by_kind("right"), &[right_pid]);

    // Layout changed: left should be ~67px (grow 2 vs grow 1)
    let left_rect = frame3.layout().get(left_pid).unwrap();
    assert!(
        left_rect.w > 60.0,
        "left should be ~67px with grow(2), got {}",
        left_rect.w
    );

    // Diff should show resized/moved but NOT added/removed
    let diff = rt.last_diff();
    assert!(
        diff.added.is_empty(),
        "no panels added on constraint change"
    );
    assert!(
        diff.removed.is_empty(),
        "no panels removed on constraint change"
    );
    assert!(!diff.resized.is_empty(), "panels should be resized");
}

#[test]
fn runtime_resolve_and_diff_work_for_generic_grid_nodes() {
    // Build a row with a sidebar and a nested grid
    let layout = {
        let mut b = panes::LayoutBuilder::new();
        b.row(|r| {
            r.panel_with("sidebar", fixed(100.0));
            r.grid(Grid::columns(2).gap(4.0), |g| {
                g.panel("card-a");
                g.panel("card-b");
            });
        })
        .unwrap();
        b.build().unwrap()
    };

    let mut rt = LayoutRuntime::from(layout);

    // First resolve: all panels added
    let frame1 = rt.resolve(500.0, 300.0).unwrap();
    assert_eq!(rt.last_diff().added.len(), 3);
    assert!(rt.last_diff().removed.is_empty());

    let sidebar = frame1.layout().by_kind("sidebar")[0];
    let card_a = frame1.layout().by_kind("card-a")[0];

    let sidebar_rect = frame1.layout().get(sidebar).unwrap();
    assert!((sidebar_rect.w - 100.0).abs() < 1.0);

    let card_a_rect = frame1.layout().get(card_a).unwrap();
    assert!(card_a_rect.w > 0.0);

    // Second resolve at different viewport: panels resize/move
    let frame2 = rt.resolve(800.0, 300.0).unwrap();
    let diff = rt.last_diff();
    assert!(diff.added.is_empty(), "no new panels on resize");
    assert!(diff.removed.is_empty(), "no panels removed on resize");

    // At least the grid cards should have resized (more space available)
    let card_a_rect2 = frame2.layout().get(card_a).unwrap();
    assert!(
        (card_a_rect2.w - card_a_rect.w).abs() > 1.0,
        "card-a should resize when viewport changes: was {}, now {}",
        card_a_rect.w,
        card_a_rect2.w
    );

    // Sidebar stays fixed
    let sidebar_rect2 = frame2.layout().get(sidebar).unwrap();
    assert!(
        (sidebar_rect2.w - 100.0).abs() < 1.0,
        "sidebar should remain 100px: got {}",
        sidebar_rect2.w
    );
}

#[test]
fn add_panel_invalidates_topology_caches_and_reports_added_panel() {
    let mut rt = Layout::master_stack(["a", "b"]).into_runtime().unwrap();

    // First resolve: establish baseline with 2 panels
    let _ = rt.resolve(400.0, 300.0).unwrap();
    assert_eq!(rt.last_diff().added.len(), 2);

    // Second resolve: settle into unchanged state
    let _ = rt.resolve(400.0, 300.0).unwrap();
    assert_eq!(rt.last_diff().unchanged.len(), 2);

    // Topology mutation: add a panel (strategy rebuilds tree with fresh PanelIds)
    let _new_pid = rt.add_panel(Arc::from("c")).unwrap();

    // Third resolve: topology dirty — full diff path used
    let frame3 = rt.resolve(400.0, 300.0).unwrap();

    // Strategy rebuild creates a fresh tree, so the diff sees new PanelIds as added.
    // The key invariant: at least one panel is reported as added (the truly new ID).
    let diff = rt.last_diff();
    assert!(
        !diff.added.is_empty(),
        "topology-dirty diff must report added panels after tree rebuild"
    );

    // Kind-index caches were rebuilt: all three kinds are discoverable
    let a_pids = frame3.layout().by_kind("a");
    let b_pids = frame3.layout().by_kind("b");
    let c_pids = frame3.layout().by_kind("c");
    assert_eq!(a_pids.len(), 1, "kind 'a' must have exactly one panel");
    assert_eq!(b_pids.len(), 1, "kind 'b' must have exactly one panel");
    assert_eq!(c_pids.len(), 1, "kind 'c' must have exactly one panel");

    // All three panels have non-zero layout rects
    assert!(frame3.layout().get(a_pids[0]).unwrap().w > 0.0);
    assert!(frame3.layout().get(b_pids[0]).unwrap().w > 0.0);
    assert!(frame3.layout().get(c_pids[0]).unwrap().w > 0.0);
}

#[test]
fn move_panel_invalidates_topology_caches_without_stale_kind_lookup() {
    let mut rt = Layout::master_stack(["a", "b", "c"])
        .into_runtime()
        .unwrap();

    // First resolve: establish baseline
    let frame1 = rt.resolve(400.0, 300.0).unwrap();
    let a_pid = frame1.layout().by_kind("a")[0];
    assert_eq!(rt.last_diff().added.len(), 3);

    // Second resolve: settle into unchanged state
    let _ = rt.resolve(400.0, 300.0).unwrap();
    assert_eq!(rt.last_diff().unchanged.len(), 3);

    // Topology mutation: move first panel to the end (strategy rebuilds tree)
    rt.move_panel(a_pid, 2).unwrap();

    // Third resolve: topology dirty — full diff path used
    let frame3 = rt.resolve(400.0, 300.0).unwrap();

    // Kind-index caches rebuilt: all three kinds still discoverable post-move
    let a_pids = frame3.layout().by_kind("a");
    let b_pids = frame3.layout().by_kind("b");
    let c_pids = frame3.layout().by_kind("c");
    assert_eq!(a_pids.len(), 1, "kind 'a' must have exactly one panel");
    assert_eq!(b_pids.len(), 1, "kind 'b' must have exactly one panel");
    assert_eq!(c_pids.len(), 1, "kind 'c' must have exactly one panel");

    // All three panels are distinct
    assert_ne!(a_pids[0], b_pids[0]);
    assert_ne!(b_pids[0], c_pids[0]);
    assert_ne!(a_pids[0], c_pids[0]);

    // The topology-dirty diff path was used (diff_reuse, not diff_same_panels_reuse).
    // In this case master_stack geometry is symmetric, so panels may report as
    // unchanged even though the diff ran the full topology path. The key invariant
    // is that kind caches were rebuilt (verified above) and no panels were
    // spuriously added or removed.
    let diff = rt.last_diff();
    assert!(
        diff.removed.is_empty(),
        "move should not remove panels: {:?}",
        diff.removed
    );

    // All panels have valid layout rects
    assert!(frame3.layout().get(a_pids[0]).unwrap().w > 0.0);
    assert!(frame3.layout().get(b_pids[0]).unwrap().w > 0.0);
    assert!(frame3.layout().get(c_pids[0]).unwrap().w > 0.0);
}

#[test]
fn constrained_child_columns_resolve_like_weighted_workspace_layout() {
    // Build: row [ sidebar(fixed:200) | col_with(grow:2) [ editor ] | col_with(grow:1) [ chat ] ]
    let mut b = LayoutBuilder::new();
    b.row(|r| {
        r.panel_with("sidebar", fixed(200.0));
        r.col_with(
            Constraints {
                grow: Some(2.0),
                ..Default::default()
            },
            |c| {
                c.panel("editor");
            },
        );
        r.col_with(
            Constraints {
                grow: Some(1.0),
                ..Default::default()
            },
            |c| {
                c.panel("chat");
            },
        );
    })
    .unwrap();
    let layout = b.build().unwrap();

    let mut rt = LayoutRuntime::from(layout);
    let frame = rt.resolve(800.0, 600.0).unwrap();

    // Sidebar should be 200px fixed
    let sidebar = frame.layout().by_kind("sidebar")[0];
    let sidebar_rect = frame.layout().get(sidebar).unwrap();
    assert!(
        (sidebar_rect.w - 200.0).abs() < 1.0,
        "sidebar should be 200px, got {}",
        sidebar_rect.w
    );

    // Remaining 600px split 2:1 between columns → 400 and 200
    let editor = frame.layout().by_kind("editor")[0];
    let editor_rect = frame.layout().get(editor).unwrap();
    assert!(
        (editor_rect.w - 400.0).abs() < 2.0,
        "editor column should be ~400px, got {}",
        editor_rect.w
    );

    let chat = frame.layout().by_kind("chat")[0];
    let chat_rect = frame.layout().get(chat).unwrap();
    assert!(
        (chat_rect.w - 200.0).abs() < 2.0,
        "chat column should be ~200px, got {}",
        chat_rect.w
    );

    // Panels should fill their respective columns vertically
    assert!(
        (editor_rect.h - 600.0).abs() < 1.0,
        "editor should fill height, got {}",
        editor_rect.h
    );
    assert!(
        (chat_rect.h - 600.0).abs() < 1.0,
        "chat should fill height, got {}",
        chat_rect.h
    );
}

#[test]
fn focus_reports_applied_unchanged_and_rejected_outcomes() {
    let (tree, pids) = build_row_tree(3, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    // Focusing an unfocused panel: Applied
    assert_eq!(rt.focus(pids[0]), FocusOutcome::Applied);
    assert_eq!(rt.focused(), Some(pids[0]));

    // Focusing the same panel again: Unchanged
    assert_eq!(rt.focus(pids[0]), FocusOutcome::Unchanged);
    assert_eq!(rt.focused(), Some(pids[0]));

    // Focusing a different panel: Applied
    assert_eq!(rt.focus(pids[1]), FocusOutcome::Applied);
    assert_eq!(rt.focused(), Some(pids[1]));

    // Focusing a missing panel: Rejected(PanelNotFound)
    let missing = PanelId::from_raw(999);
    assert_eq!(
        rt.focus(missing),
        FocusOutcome::Rejected(FocusRejection::PanelNotFound)
    );
    // Focus should remain on previous panel
    assert_eq!(rt.focused(), Some(pids[1]));
}
