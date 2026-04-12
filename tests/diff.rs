#![allow(clippy::unwrap_used, clippy::panic)]
mod helpers;

use helpers::build_row_tree;
use panes::diff::diff;
use panes::runtime::LayoutRuntime;
use panes::{LayoutTree, PanelId, Rect, fixed, grow};

#[test]
fn diff_identical_layouts_all_unchanged() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let old = tree.resolve(100.0, 100.0).unwrap();
    let new = tree.resolve(100.0, 100.0).unwrap();

    let scratch = diff(&old, &new);
    let d = scratch.as_diff();

    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
    assert!(d.moved.is_empty());
    assert!(d.resized.is_empty());
    assert_eq!(d.unchanged.len(), 2);
    assert!(d.unchanged.contains(&pids[0]));
    assert!(d.unchanged.contains(&pids[1]));
}

#[test]
fn diff_removed_panel() {
    let mut tree = LayoutTree::new();
    let (_, n0) = tree.add_panel("p0", grow(1.0)).unwrap();
    let (p1, n1) = tree.add_panel("p1", grow(1.0)).unwrap();
    let (_, n2) = tree.add_panel("p2", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1, n2]).unwrap();
    tree.set_root(root);

    let old = tree.resolve(90.0, 100.0).unwrap();

    // Remove middle panel
    tree.remove_panel(p1).unwrap();
    let new = tree.resolve(90.0, 100.0).unwrap();

    let scratch = diff(&old, &new);
    let d = scratch.as_diff();

    // p1 was removed
    assert_eq!(d.removed.len(), 1);
    assert!(d.removed.contains(&p1));

    // p0 and p2 grew from 30px to 45px wide — they are resized
    assert_eq!(d.resized.len(), 2);

    assert!(d.added.is_empty());
}

#[test]
fn diff_resized_panels() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let old = tree.resolve(100.0, 100.0).unwrap();
    let new = tree.resolve(200.0, 100.0).unwrap();

    let scratch = diff(&old, &new);
    let d = scratch.as_diff();

    // Both panels resized (width changed from 50 to 100)
    assert_eq!(d.resized.len(), 2);
    let p0_resize = d.resized.iter().find(|c| c.id == pids[0]).unwrap();
    assert_eq!(
        p0_resize.from,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 50.0,
            h: 100.0
        }
    );
    assert_eq!(
        p0_resize.to,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0
        }
    );

    // Second panel moved (x changed from 50 to 100) — it's in moved
    assert_eq!(d.moved.len(), 1);
    let p1_move = d.moved.iter().find(|c| c.id == pids[1]).unwrap();
    assert_eq!(p1_move.from.x, 50.0);
    assert_eq!(p1_move.to.x, 100.0);

    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
    assert!(d.unchanged.is_empty());
}

#[test]
fn diff_moved_not_resized() {
    let mut tree = LayoutTree::new();
    let (p0, n0) = tree.add_panel("p0", fixed(20.0)).unwrap();
    let (_, n1) = tree.add_panel("p1", fixed(20.0)).unwrap();
    let (p2, n2) = tree.add_panel("p2", fixed(20.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1, n2]).unwrap();
    tree.set_root(root);

    let old = tree.resolve(60.0, 100.0).unwrap();

    // Move first panel to after last: order becomes [p1, p2, p0]
    tree.move_panel(p0, panes::Position::After(p2)).unwrap();
    let new = tree.resolve(60.0, 100.0).unwrap();

    let scratch = diff(&old, &new);
    let d = scratch.as_diff();

    // All three panels moved (positions changed)
    assert_eq!(d.moved.len(), 3);

    // None resized (fixed 20px each, same viewport)
    assert!(d.resized.is_empty());
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
}

#[test]
fn diff_moved_and_resized() {
    let mut tree = LayoutTree::new();
    let (p0, n0) = tree.add_panel("p0", grow(1.0)).unwrap();
    let (_, n1) = tree.add_panel("p1", grow(1.0)).unwrap();
    let (p2, n2) = tree.add_panel("p2", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1, n2]).unwrap();
    tree.set_root(root);

    let old = tree.resolve(60.0, 100.0).unwrap();

    // Move p0 to after p2: [p1, p2, p0], then resolve at wider viewport
    tree.move_panel(p0, panes::Position::After(p2)).unwrap();
    let new = tree.resolve(90.0, 100.0).unwrap();

    let scratch = diff(&old, &new);
    let d = scratch.as_diff();

    // Panels that moved should also be in resized (viewport grew too)
    let moved_ids: Vec<PanelId> = d.moved.iter().map(|c| c.id).collect();
    let resized_ids: Vec<PanelId> = d.resized.iter().map(|c| c.id).collect();

    // p0 definitely moved and resized
    assert!(moved_ids.contains(&p0));
    assert!(resized_ids.contains(&p0));
}

#[test]
fn diff_first_frame() {
    let (tree, pids) = build_row_tree(2, grow(1.0));
    let layout = tree.resolve(100.0, 100.0).unwrap();

    let scratch = diff(&layout, &layout);
    let d = scratch.as_diff();

    // When diffing layout against itself, all panels should be unchanged
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
    assert!(d.moved.is_empty());
    assert!(d.resized.is_empty());
    assert_eq!(d.unchanged.len(), 2);
    assert!(d.unchanged.contains(&pids[0]));
    assert!(d.unchanged.contains(&pids[1]));
}

#[test]
fn same_panel_diff_ignores_retired_panel_holes_after_remove_and_resize() {
    // Build a tree with panels, including some that will be removed to create
    // retired PanelId holes in the sparse rects array.
    let mut tree = LayoutTree::new();
    let (_, n0) = tree.add_panel("a", grow(1.0)).unwrap();
    let (hole1, n_hole1) = tree.add_panel("tmp1", grow(1.0)).unwrap();
    let (_, n1) = tree.add_panel("b", grow(1.0)).unwrap();
    let (hole2, n_hole2) = tree.add_panel("tmp2", grow(1.0)).unwrap();
    let (_, n2) = tree.add_panel("c", grow(1.0)).unwrap();
    let root = tree
        .add_row(0.0, vec![n0, n_hole1, n1, n_hole2, n2])
        .unwrap();
    tree.set_root(root);

    // Remove the temporary panels to create retired PanelId holes.
    tree.remove_panel(hole1).unwrap();
    tree.remove_panel(hole2).unwrap();

    let mut rt = LayoutRuntime::new(tree);

    // First resolve: topology dirty, establishes the baseline frame.
    rt.resolve(200.0, 200.0).unwrap();

    // Second resolve at same dimensions: layout-only, same panel set.
    // This uses diff_same_panels_reuse which should iterate only live panels.
    rt.resolve(200.0, 200.0).unwrap();

    // Layout-only resolve at different viewport: triggers same-panel diff path.
    let frame = rt.resolve(300.0, 200.0).unwrap();
    let d = rt.last_diff();

    // No panels were added or removed — this was layout-only.
    assert!(d.added.is_empty(), "expected no added, got {:?}", d.added);
    assert!(
        d.removed.is_empty(),
        "expected no removed, got {:?}",
        d.removed
    );

    // All live panels should be classified. Note: moved and resized can overlap
    // (a panel that both moves and resizes appears in both), so count unique IDs.
    let live_count = frame.layout().panels().count();
    let mut classified_ids: Vec<PanelId> = Vec::new();
    classified_ids.extend(d.moved.iter().map(|c| c.id));
    classified_ids.extend(d.resized.iter().map(|c| c.id));
    classified_ids.extend(d.unchanged.iter().copied());
    classified_ids.sort_by_key(|p| p.raw());
    classified_ids.dedup();
    assert_eq!(
        classified_ids.len(),
        live_count,
        "diff classified {} unique panels but {live_count} are live",
        classified_ids.len(),
    );
}

#[test]
fn topology_dirty_frames_still_report_add_remove_over_live_panel_iteration_changes() {
    let mut tree = LayoutTree::new();
    let (_, n0) = tree.add_panel("a", grow(1.0)).unwrap();
    let (_, n1) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1]).unwrap();
    tree.set_root(root);

    let mut rt = LayoutRuntime::new(tree);
    rt.resolve(200.0, 200.0).unwrap();

    // Topology mutation: add a new panel via tree_mut (marks topology dirty).
    let (new_pid, new_nid) = rt.tree_mut().add_panel("c", grow(1.0)).unwrap();
    rt.tree_mut().insert_child_at(root, 2, new_nid).unwrap();

    let frame = rt.resolve(200.0, 200.0).unwrap();
    let d = rt.last_diff();

    // The new panel should appear in added.
    assert!(d.added.contains(&new_pid), "new panel should be in added");
    assert!(d.removed.is_empty(), "no panels were removed");

    // All live panels must be accounted for (moved/resized can overlap).
    let live_count = frame.layout().panels().count();
    let mut ids: Vec<PanelId> = Vec::new();
    ids.extend(d.added.iter().copied());
    ids.extend(d.moved.iter().map(|c| c.id));
    ids.extend(d.resized.iter().map(|c| c.id));
    ids.extend(d.unchanged.iter().copied());
    ids.sort_by_key(|p| p.raw());
    ids.dedup();
    assert_eq!(ids.len(), live_count, "all live panels must be classified");

    // Now remove it — topology dirty again.
    rt.tree_mut().remove_panel(new_pid).unwrap();
    let frame2 = rt.resolve(200.0, 200.0).unwrap();
    let d2 = rt.last_diff();

    assert!(
        d2.removed.contains(&new_pid),
        "removed panel should be in removed"
    );
    assert!(d2.added.is_empty(), "no panels were added");

    let live_count2 = frame2.layout().panels().count();
    let mut ids2: Vec<PanelId> = Vec::new();
    ids2.extend(d2.moved.iter().map(|c| c.id));
    ids2.extend(d2.resized.iter().map(|c| c.id));
    ids2.extend(d2.unchanged.iter().copied());
    ids2.sort_by_key(|p| p.raw());
    ids2.dedup();
    assert_eq!(
        ids2.len(),
        live_count2,
        "all live panels must be classified"
    );
}
