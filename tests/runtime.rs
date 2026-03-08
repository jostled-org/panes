mod helpers;

use helpers::build_row_tree;
use panes::runtime::LayoutRuntime;
use panes::{Layout, PanelId, fixed, grow};

#[test]
fn runtime_first_resolve_all_added() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    let frame = rt.resolve(100.0, 100.0).unwrap();

    // First frame: all panels should be added
    assert_eq!(frame.diff().added.len(), 2);
    assert!(frame.diff().added.contains(&pids[0]));
    assert!(frame.diff().added.contains(&pids[1]));
    assert!(frame.diff().removed.is_empty());
    assert!(frame.diff().moved.is_empty());
    assert!(frame.diff().resized.is_empty());
    assert!(frame.diff().unchanged.is_empty());

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
    let frame = rt.resolve(100.0, 100.0).unwrap();

    // Second frame at same dimensions: all unchanged
    assert!(frame.diff().added.is_empty());
    assert!(frame.diff().removed.is_empty());
    assert!(frame.diff().moved.is_empty());
    assert!(frame.diff().resized.is_empty());
    assert_eq!(frame.diff().unchanged.len(), 2);
    assert!(frame.diff().unchanged.contains(&pids[0]));
    assert!(frame.diff().unchanged.contains(&pids[1]));
}

#[test]
fn runtime_resolve_different_size_shows_resize() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    let _ = rt.resolve(100.0, 100.0).unwrap();
    let frame = rt.resolve(200.0, 100.0).unwrap();

    // Both panels resized (width changed from 50 to 100)
    assert_eq!(frame.diff().resized.len(), 2);

    // Second panel also moved (its x position changed)
    assert_eq!(frame.diff().moved.len(), 1);
    let moved_ids: Vec<PanelId> = frame.diff().moved.iter().map(|c| c.id).collect();
    assert!(moved_ids.contains(&pids[1]));
}

#[test]
fn runtime_remove_panel_in_diff() {
    let (tree, pids) = build_row_tree(3, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    let _ = rt.resolve(90.0, 100.0).unwrap();

    // Remove middle panel
    rt.tree_mut().remove_panel(pids[1]).unwrap();
    let frame = rt.resolve(90.0, 100.0).unwrap();

    // Middle panel removed
    assert_eq!(frame.diff().removed.len(), 1);
    assert!(frame.diff().removed.contains(&pids[1]));

    // Remaining panels resized (grew from 30px to 45px)
    assert_eq!(frame.diff().resized.len(), 2);
}

#[test]
fn runtime_set_constraints_in_diff() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    let _ = rt.resolve(100.0, 100.0).unwrap();

    // Change one panel to grow(2)
    rt.tree_mut().set_constraints(pids[0], grow(2.0)).unwrap();
    let frame = rt.resolve(100.0, 100.0).unwrap();

    // Both panels resized (proportions changed from 50/50 to ~67/33)
    assert_eq!(frame.diff().resized.len(), 2);
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
fn scroll_by_shifts_x() {
    let layout = Layout::split("a", "b").build().unwrap();
    let mut rt = LayoutRuntime::from(layout);

    let frame = rt.resolve(100.0, 100.0).unwrap();
    let a_pid = frame.layout().by_kind("a")[0];
    let base_x = frame.layout().get(a_pid).unwrap().x;

    rt.scroll_by(40.0);
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

    rt.scroll_to(80.0);
    let frame = rt.resolve(100.0, 100.0).unwrap();
    let new_x = frame.layout().get(a_pid).unwrap().x;
    assert!((new_x - (base_x - 80.0)).abs() < 0.1);
}

#[test]
fn set_active_queryable() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let mut rt = LayoutRuntime::from(tree);

    assert!(rt.active_panel().is_none());

    rt.set_active(pids[0]);
    assert_eq!(rt.active_panel(), Some(pids[0]));

    rt.set_active(pids[1]);
    assert_eq!(rt.active_panel(), Some(pids[1]));
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
