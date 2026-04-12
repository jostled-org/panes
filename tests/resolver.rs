#![allow(clippy::unwrap_used, clippy::panic)]
use panes::compiler::{compile, compute_layout};
use panes::resolver::resolve;
use panes::{Layout, LayoutTree, PanelId, Rect, fixed, grow};

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

// -- kind query methods --

#[test]
fn kind_of_returns_correct_kind_for_each_panel() {
    let mut tree = LayoutTree::new();
    let (p_ed, n_ed) = tree.add_panel("editor", grow(1.0)).unwrap();
    let (p_term, n_term) = tree.add_panel("terminal", grow(1.0)).unwrap();
    let (p_chat, n_chat) = tree.add_panel("chat", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n_ed, n_term, n_chat]).unwrap();
    tree.set_root(root);

    let resolved = compile_and_resolve(&tree, 120.0, 40.0);

    assert_eq!(resolved.kind_of(p_ed), Some("editor"));
    assert_eq!(resolved.kind_of(p_term), Some("terminal"));
    assert_eq!(resolved.kind_of(p_chat), Some("chat"));

    // Absent panel returns None
    assert_eq!(resolved.kind_of(PanelId::from_raw(999)), None);
}

#[test]
fn kind_index_of_panel_matches_panels_iterator() {
    let mut tree = LayoutTree::new();
    let (_, na) = tree.add_panel("a", grow(1.0)).unwrap();
    let (_, nb) = tree.add_panel("b", grow(1.0)).unwrap();
    let (_, na2) = tree.add_panel("a", grow(1.0)).unwrap();
    let (_, nc) = tree.add_panel("c", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![na, nb, na2, nc]).unwrap();
    tree.set_root(root);

    let resolved = compile_and_resolve(&tree, 120.0, 40.0);

    // Collect from panels() iterator
    let panel_entries: Vec<_> = resolved.panels().collect();
    for entry in &panel_entries {
        assert_eq!(
            resolved.kind_index_of_panel(entry.id),
            Some(entry.kind_index),
            "kind_index_of_panel mismatch for {:?}",
            entry.id
        );
    }

    // Absent panel returns None
    assert_eq!(resolved.kind_index_of_panel(PanelId::from_raw(999)), None);
}

#[test]
fn kind_index_of_matches_sorted_kind_keys_position() {
    let mut tree = LayoutTree::new();
    let (_, nc) = tree.add_panel("chat", grow(1.0)).unwrap();
    let (_, ne) = tree.add_panel("editor", grow(1.0)).unwrap();
    let (_, nt) = tree.add_panel("terminal", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![nc, ne, nt]).unwrap();
    tree.set_root(root);

    let resolved = compile_and_resolve(&tree, 120.0, 40.0);

    for (i, kind) in resolved.sorted_kind_keys().iter().enumerate() {
        assert_eq!(
            resolved.kind_index_of(kind),
            Some(i),
            "kind_index_of mismatch for {kind}"
        );
    }
    assert_eq!(resolved.kind_index_of("nonexistent"), None);
}

#[test]
fn decoration_entries_returns_matching_decorations_with_kind_index() {
    let mut rt = Layout::tabbed(["editor", "chat"]).into_runtime().unwrap();
    let frame = rt.resolve(80.0, 24.0).unwrap();
    let resolved = frame.layout();

    let entries: Vec<_> = resolved.decoration_entries("editor").collect();
    // Tabbed layout produces tab decorations for each kind
    assert!(
        entries
            .iter()
            .any(|(info, _)| info.content_kind.as_ref() == "editor"),
        "expected at least one decoration for editor"
    );
    // kind_index should match kind_index_of("editor")
    let expected_ki = resolved.kind_index_of("editor").unwrap();
    for (_, ki) in &entries {
        assert_eq!(*ki, expected_ki);
    }

    // Nonexistent kind yields nothing
    assert_eq!(resolved.decoration_entries("nonexistent").count(), 0);
}

#[test]
fn decoration_role_returns_indexed_role_for_decoration_panel() {
    let mut rt = Layout::tabbed(["editor", "chat"]).into_runtime().unwrap();
    let frame = rt.resolve(80.0, 24.0).unwrap();
    let resolved = frame.layout();

    let decoration = resolved
        .decoration_panels()
        .first()
        .expect("tabbed layout should expose decorations");

    assert_eq!(
        resolved.decoration_role(decoration.id),
        Some(decoration.role)
    );
    assert_eq!(resolved.decoration_role(PanelId::from_raw(999)), None);
}

#[test]
fn kind_queries_survive_layout_only_reframe() {
    let mut rt = Layout::master_stack(["a", "b", "c"])
        .into_runtime()
        .unwrap();
    let frame1 = rt.resolve(80.0, 24.0).unwrap();

    // Record kind_of results
    let first_kinds: Vec<_> = frame1
        .layout()
        .panel_ids()
        .map(|pid| (pid, frame1.layout().kind_of(pid).map(str::to_owned)))
        .collect();

    // Trigger layout-only dirty (topology unchanged) by changing constraints
    let any_pid = first_kinds[0].0;
    rt.set_constraints(any_pid, fixed(10.0)).unwrap();
    let frame2 = rt.resolve(80.0, 24.0).unwrap();

    for (pid, expected_kind) in &first_kinds {
        assert_eq!(
            frame2.layout().kind_of(*pid).map(str::to_owned).as_ref(),
            expected_kind.as_ref(),
            "kind_of mismatch after layout-only reframe for {:?}",
            pid
        );
    }
}

#[test]
fn kind_queries_correct_on_interpolated_layout() {
    let mut tree_a = LayoutTree::new();
    let (_, na) = tree_a.add_panel("a", grow(1.0)).unwrap();
    let (_, nb) = tree_a.add_panel("b", grow(1.0)).unwrap();
    let (_, nc) = tree_a.add_panel("c", grow(1.0)).unwrap();
    let root = tree_a.add_row(0.0, vec![na, nb, nc]).unwrap();
    tree_a.set_root(root);
    let layout1 = compile_and_resolve(&tree_a, 90.0, 30.0);

    let mut tree_b = LayoutTree::new();
    let (_, na) = tree_b.add_panel("a", grow(1.0)).unwrap();
    let (_, nb) = tree_b.add_panel("b", grow(2.0)).unwrap();
    let (_, nc) = tree_b.add_panel("c", grow(1.0)).unwrap();
    let root = tree_b.add_row(0.0, vec![na, nb, nc]).unwrap();
    tree_b.set_root(root);
    let layout2 = compile_and_resolve(&tree_b, 120.0, 30.0);

    let interpolated = layout1.lerp(&layout2, 0.5);

    // kind_of and kind_index_of_panel should match source layout
    for pid in interpolated.panel_ids() {
        assert_eq!(
            interpolated.kind_of(pid),
            layout1.kind_of(pid),
            "kind_of mismatch on interpolated layout for {:?}",
            pid
        );
        assert_eq!(
            interpolated.kind_index_of_panel(pid),
            layout1.kind_index_of_panel(pid),
            "kind_index_of_panel mismatch on interpolated layout for {:?}",
            pid
        );
    }
}
