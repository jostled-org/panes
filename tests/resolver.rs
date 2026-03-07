use panes::compiler::{compile, compute_layout};
use panes::resolver::resolve;
use panes::{LayoutTree, Rect, fixed, grow};

fn compile_and_resolve(tree: &LayoutTree, w: f32, h: f32) -> panes::resolver::ResolvedLayout {
    let mut result = compile(tree).unwrap();
    compute_layout(&mut result, w, h).unwrap();
    resolve(&result, tree).unwrap()
}

#[test]
fn single_panel_fills_viewport() {
    let mut tree = LayoutTree::new();
    let (pid, nid) = tree.add_panel("editor", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![nid]).unwrap();
    tree.set_root(root);

    let resolved = compile_and_resolve(&tree, 80.0, 24.0);
    let r = resolved.get(pid).unwrap();

    assert_eq!(
        *r,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 80.0,
            h: 24.0
        }
    );
}

#[test]
fn two_equal_grow_panels_split_width() {
    let mut tree = LayoutTree::new();
    let (p1, n1) = tree.add_panel("a", grow(1.0)).unwrap();
    let (p2, n2) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n1, n2]).unwrap();
    tree.set_root(root);

    let resolved = compile_and_resolve(&tree, 100.0, 50.0);

    assert_eq!(
        *resolved.get(p1).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 50.0,
            h: 50.0
        }
    );
    assert_eq!(
        *resolved.get(p2).unwrap(),
        Rect {
            x: 50.0,
            y: 0.0,
            w: 50.0,
            h: 50.0
        }
    );
}

#[test]
fn grow_ratio_distributes_proportionally() {
    let mut tree = LayoutTree::new();
    let (p1, n1) = tree.add_panel("a", grow(2.0)).unwrap();
    let (p2, n2) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n1, n2]).unwrap();
    tree.set_root(root);

    let resolved = compile_and_resolve(&tree, 90.0, 30.0);

    assert_eq!(
        *resolved.get(p1).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 60.0,
            h: 30.0
        }
    );
    assert_eq!(
        *resolved.get(p2).unwrap(),
        Rect {
            x: 60.0,
            y: 0.0,
            w: 30.0,
            h: 30.0
        }
    );
}

#[test]
fn fixed_plus_grow_sizing() {
    let mut tree = LayoutTree::new();
    let (p_fixed, n_fixed) = tree.add_panel("sidebar", fixed(20.0)).unwrap();
    let (p_grow, n_grow) = tree.add_panel("main", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n_fixed, n_grow]).unwrap();
    tree.set_root(root);

    let resolved = compile_and_resolve(&tree, 100.0, 40.0);

    assert_eq!(
        *resolved.get(p_fixed).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 20.0,
            h: 40.0
        }
    );
    assert_eq!(
        *resolved.get(p_grow).unwrap(),
        Rect {
            x: 20.0,
            y: 0.0,
            w: 80.0,
            h: 40.0
        }
    );
}

#[test]
fn nested_row_col_positions() {
    let mut tree = LayoutTree::new();
    // Row: [editor(grow=2), Col: [chat(grow=1), status(grow=1)]]
    let (p_editor, n_editor) = tree.add_panel("editor", grow(2.0)).unwrap();
    let (p_chat, n_chat) = tree.add_panel("chat", grow(1.0)).unwrap();
    let (p_status, n_status) = tree.add_panel("status", grow(1.0)).unwrap();
    let col = tree.add_col(0.0, vec![n_chat, n_status]).unwrap();
    let root = tree.add_row(0.0, vec![n_editor, col]).unwrap();
    tree.set_root(root);

    let resolved = compile_and_resolve(&tree, 90.0, 24.0);

    assert_eq!(
        *resolved.get(p_editor).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 60.0,
            h: 24.0
        }
    );
    assert_eq!(
        *resolved.get(p_chat).unwrap(),
        Rect {
            x: 60.0,
            y: 0.0,
            w: 30.0,
            h: 12.0
        }
    );
    assert_eq!(
        *resolved.get(p_status).unwrap(),
        Rect {
            x: 60.0,
            y: 12.0,
            w: 30.0,
            h: 12.0
        }
    );
}

#[test]
fn gap_reduces_available_space() {
    let mut tree = LayoutTree::new();
    let (p1, n1) = tree.add_panel("a", grow(1.0)).unwrap();
    let (p2, n2) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(10.0, vec![n1, n2]).unwrap();
    tree.set_root(root);

    let resolved = compile_and_resolve(&tree, 100.0, 40.0);
    let r1 = resolved.get(p1).unwrap();
    let r2 = resolved.get(p2).unwrap();

    // 100 - 10 gap = 90 split equally = 45 each
    assert_eq!(r1.w, 45.0);
    assert_eq!(r2.w, 45.0);
    // Second panel starts after first + gap
    assert_eq!(r2.x, 55.0);
}

#[test]
fn kind_index_works() {
    let mut tree = LayoutTree::new();
    let (pb1, nb1) = tree.add_panel("btn", grow(1.0)).unwrap();
    let (pb2, nb2) = tree.add_panel("btn", grow(1.0)).unwrap();
    let (pd, nd) = tree.add_panel("dialog", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![nb1, nb2, nd]).unwrap();
    tree.set_root(root);

    let resolved = compile_and_resolve(&tree, 90.0, 30.0);

    let btns = resolved.by_kind("btn");
    assert_eq!(btns.len(), 2);
    assert!(btns.contains(&pb1));
    assert!(btns.contains(&pb2));

    let dialogs = resolved.by_kind("dialog");
    assert_eq!(dialogs.len(), 1);
    assert!(dialogs.contains(&pd));

    let kinds: Vec<&str> = resolved.kinds().collect();
    assert_eq!(kinds.len(), 2);
    assert!(kinds.contains(&"btn"));
    assert!(kinds.contains(&"dialog"));

    // Missing kind returns empty
    assert!(resolved.by_kind("nonexistent").is_empty());
}

#[test]
fn iter_and_panel_ids_yield_all_entries() {
    let mut tree = LayoutTree::new();
    let (p1, n1) = tree.add_panel("a", grow(1.0)).unwrap();
    let (p2, n2) = tree.add_panel("b", grow(1.0)).unwrap();
    let (p3, n3) = tree.add_panel("c", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n1, n2, n3]).unwrap();
    tree.set_root(root);

    let resolved = compile_and_resolve(&tree, 90.0, 30.0);

    let ids: Vec<_> = resolved.panel_ids().collect();
    assert_eq!(ids.len(), 3);
    assert!(ids.contains(&p1));
    assert!(ids.contains(&p2));
    assert!(ids.contains(&p3));

    let iter_entries: Vec<_> = resolved.iter().collect();
    assert_eq!(iter_entries.len(), 3);
    for (pid, rect) in &iter_entries {
        assert_eq!(resolved.get(*pid), Some(*rect));
    }
}

// -- lerp --

fn two_panel_layouts() -> (
    panes::resolver::ResolvedLayout,
    panes::resolver::ResolvedLayout,
) {
    let mut tree_a = LayoutTree::new();
    let (_, n1) = tree_a.add_panel("a", grow(1.0)).unwrap();
    let (_, n2) = tree_a.add_panel("b", grow(1.0)).unwrap();
    let root = tree_a.add_row(0.0, vec![n1, n2]).unwrap();
    tree_a.set_root(root);
    let from = compile_and_resolve(&tree_a, 100.0, 50.0);

    let mut tree_b = LayoutTree::new();
    let (_, n1) = tree_b.add_panel("a", grow(1.0)).unwrap();
    let (_, n2) = tree_b.add_panel("b", grow(3.0)).unwrap();
    let root = tree_b.add_row(0.0, vec![n1, n2]).unwrap();
    tree_b.set_root(root);
    let to = compile_and_resolve(&tree_b, 100.0, 50.0);

    (from, to)
}

#[test]
fn lerp_at_zero_returns_from() {
    let (from, to) = two_panel_layouts();
    let result = from.lerp(&to, 0.0);
    for (pid, rect) in from.iter() {
        let r = result.get(pid).unwrap();
        assert!((r.x - rect.x).abs() < 0.01);
        assert!((r.y - rect.y).abs() < 0.01);
        assert!((r.w - rect.w).abs() < 0.01);
        assert!((r.h - rect.h).abs() < 0.01);
    }
}

#[test]
fn lerp_at_one_returns_to() {
    let (from, to) = two_panel_layouts();
    let result = from.lerp(&to, 1.0);
    for (pid, rect) in to.iter() {
        let r = result.get(pid).unwrap();
        assert!((r.x - rect.x).abs() < 0.01);
        assert!((r.y - rect.y).abs() < 0.01);
        assert!((r.w - rect.w).abs() < 0.01);
        assert!((r.h - rect.h).abs() < 0.01);
    }
}

#[test]
fn lerp_midpoint() {
    let (from, to) = two_panel_layouts();
    let result = from.lerp(&to, 0.5);
    for (pid, from_rect) in from.iter() {
        let to_rect = to.get(pid).unwrap();
        let r = result.get(pid).unwrap();
        assert!((r.x - (from_rect.x + to_rect.x) / 2.0).abs() < 0.01);
        assert!((r.w - (from_rect.w + to_rect.w) / 2.0).abs() < 0.01);
    }
}
