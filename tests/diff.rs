mod helpers;

use helpers::build_row_tree;
use panes::diff::diff;
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
