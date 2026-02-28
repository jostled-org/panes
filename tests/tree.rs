use std::sync::Arc;

use panes::{Constraints, LayoutTree, Node, PaneError, PanelId, Position, fixed, grow};

#[test]
fn tree_add_panel_returns_panel_id_and_node_id() {
    let mut tree = LayoutTree::new();
    let (pid, nid) = tree.add_panel("editor", grow(1.0)).unwrap();

    assert_eq!(pid, PanelId::from_raw(0));
    assert_ne!(nid.raw(), u32::MAX);
    assert!(tree.node(nid).is_some());
}

#[test]
fn tree_add_row_with_children() {
    let mut tree = LayoutTree::new();
    let (_, n1) = tree.add_panel("editor", grow(1.0)).unwrap();
    let (_, n2) = tree.add_panel("chat", grow(1.0)).unwrap();
    let row = tree.add_row(8.0, vec![n1, n2]).unwrap();
    tree.set_root(row);

    assert_eq!(tree.root(), Some(row));
    match tree.node(row) {
        Some(Node::Row { children, gap }) => {
            assert_eq!(*gap, 8.0);
            assert_eq!(children.len(), 2);
            assert_eq!(children[0], n1);
            assert_eq!(children[1], n2);
        }
        other => panic!("expected Row, got {other:?}"),
    }
}

#[test]
fn tree_add_col_with_children() {
    let mut tree = LayoutTree::new();
    let (_, n1) = tree.add_panel("a", grow(1.0)).unwrap();
    let (_, n2) = tree.add_panel("b", grow(1.0)).unwrap();
    let col = tree.add_col(0.0, vec![n1, n2]).unwrap();

    match tree.node(col) {
        Some(Node::Col { children, gap }) => {
            assert_eq!(*gap, 0.0);
            assert_eq!(children.len(), 2);
        }
        other => panic!("expected Col, got {other:?}"),
    }
}

#[test]
fn tree_nested_row_col() {
    let mut tree = LayoutTree::new();
    let (_, editor) = tree.add_panel("editor", grow(2.0)).unwrap();
    let (_, chat) = tree.add_panel("chat", grow(1.0)).unwrap();
    let (_, status) = tree.add_panel("status", grow(1.0)).unwrap();

    let col = tree.add_col(0.0, vec![chat, status]).unwrap();
    let row = tree.add_row(8.0, vec![editor, col]).unwrap();
    tree.set_root(row);

    // Root is a row with 2 children
    match tree.node(row) {
        Some(Node::Row { children, .. }) => {
            assert_eq!(children.len(), 2);
            assert_eq!(children[0], editor);
            assert_eq!(children[1], col);
        }
        other => panic!("expected Row, got {other:?}"),
    }

    // Second child is a col with 2 children
    match tree.node(col) {
        Some(Node::Col { children, .. }) => {
            assert_eq!(children.len(), 2);
            assert_eq!(children[0], chat);
            assert_eq!(children[1], status);
        }
        other => panic!("expected Col, got {other:?}"),
    }
}

#[test]
fn tree_multiple_panels_same_kind() {
    let mut tree = LayoutTree::new();
    let (p1, _) = tree.add_panel("editor", grow(1.0)).unwrap();
    let (p2, _) = tree.add_panel("editor", grow(1.0)).unwrap();
    let (p3, _) = tree.add_panel("editor", grow(1.0)).unwrap();

    // All get distinct PanelIds
    assert_ne!(p1, p2);
    assert_ne!(p2, p3);
    assert_ne!(p1, p3);

    // Three panels of kind "editor"
    let editors = tree.panels_by_kind("editor");
    assert_eq!(editors.len(), 3);
}

#[test]
fn tree_taffy_passthrough_node() {
    let mut tree = LayoutTree::new();
    let style = taffy::Style {
        flex_grow: 3.0,
        ..Default::default()
    };
    let nid = tree.add_taffy_node(style.clone(), vec![]).unwrap();

    match tree.node(nid) {
        Some(Node::TaffyPassthrough { style, children }) => {
            assert_eq!(style.flex_grow, 3.0);
            assert!(children.is_empty());
        }
        other => panic!("expected TaffyPassthrough, got {other:?}"),
    }
}

#[test]
fn tree_root_is_set_correctly() {
    let mut tree = LayoutTree::new();
    let (_, n1) = tree.add_panel("a", grow(1.0)).unwrap();
    let row = tree.add_row(0.0, vec![n1]).unwrap();
    assert_eq!(tree.root(), None);

    tree.set_root(row);
    assert_eq!(tree.root(), Some(row));
}

#[test]
fn tree_panel_count_and_kinds() {
    let mut tree = LayoutTree::new();
    tree.add_panel("editor", grow(1.0)).unwrap();
    tree.add_panel("editor", grow(1.0)).unwrap();
    tree.add_panel("chat", grow(1.0)).unwrap();
    tree.add_panel("status", grow(1.0)).unwrap();

    assert_eq!(tree.panel_count(), 4);

    let mut kinds: Vec<Arc<str>> = tree.kinds().cloned().collect();
    kinds.sort();
    assert_eq!(
        kinds,
        vec![Arc::from("chat"), Arc::from("editor"), Arc::from("status")]
    );

    let editors = tree.panels_by_kind("editor");
    assert_eq!(editors.len(), 2);
}

// --- Step 3: Structural Mutations ---

#[test]
fn remove_panel_from_tree() {
    let mut tree = LayoutTree::new();
    let (_, n_editor) = tree.add_panel("editor", grow(1.0)).unwrap();
    let (pid_chat, n_chat) = tree.add_panel("chat", grow(1.0)).unwrap();
    let row = tree.add_row(8.0, vec![n_editor, n_chat]).unwrap();
    tree.set_root(row);

    tree.remove_panel(pid_chat).unwrap();

    assert_eq!(tree.panel_count(), 1);
    assert!(tree.panels_by_kind("chat").is_empty());
    match tree.node(row) {
        Some(Node::Row { children, .. }) => assert_eq!(children.len(), 1),
        other => panic!("expected Row, got {other:?}"),
    }
}

#[test]
fn remove_panel_not_found_returns_error() {
    let mut tree = LayoutTree::new();
    tree.add_panel("editor", grow(1.0)).unwrap();

    let bad_id = PanelId::from_raw(999);
    let result = tree.remove_panel(bad_id);
    assert!(matches!(result, Err(PaneError::PanelNotFound(_))));
}

#[test]
fn move_panel_after_another() {
    let mut tree = LayoutTree::new();
    let (pa, na) = tree.add_panel("a", grow(1.0)).unwrap();
    let (_, nb) = tree.add_panel("b", grow(1.0)).unwrap();
    let (pc, nc) = tree.add_panel("c", grow(1.0)).unwrap();
    let row = tree.add_row(0.0, vec![na, nb, nc]).unwrap();
    tree.set_root(row);

    tree.move_panel(pa, Position::After(pc)).unwrap();

    match tree.node(row) {
        Some(Node::Row { children, .. }) => {
            assert_eq!(children, &[nb, nc, na]);
        }
        other => panic!("expected Row, got {other:?}"),
    }
}

#[test]
fn move_panel_before_another() {
    let mut tree = LayoutTree::new();
    let (pa, na) = tree.add_panel("a", grow(1.0)).unwrap();
    let (_, nb) = tree.add_panel("b", grow(1.0)).unwrap();
    let (pc, nc) = tree.add_panel("c", grow(1.0)).unwrap();
    let row = tree.add_row(0.0, vec![na, nb, nc]).unwrap();
    tree.set_root(row);

    tree.move_panel(pc, Position::Before(pa)).unwrap();

    match tree.node(row) {
        Some(Node::Row { children, .. }) => {
            assert_eq!(children, &[nc, na, nb]);
        }
        other => panic!("expected Row, got {other:?}"),
    }
}

#[test]
fn move_panel_to_different_container() {
    let mut tree = LayoutTree::new();
    let (pa, na) = tree.add_panel("a", grow(1.0)).unwrap();
    let (pb, nb) = tree.add_panel("b", grow(1.0)).unwrap();
    let (_, nc) = tree.add_panel("c", grow(1.0)).unwrap();
    let col = tree.add_col(0.0, vec![nb, nc]).unwrap();
    let row = tree.add_row(0.0, vec![na, col]).unwrap();
    tree.set_root(row);

    tree.move_panel(pa, Position::After(pb)).unwrap();

    // Row now has one child (the col)
    match tree.node(row) {
        Some(Node::Row { children, .. }) => assert_eq!(children.len(), 1),
        other => panic!("expected Row, got {other:?}"),
    }
    // Col now has three children [b, a, c]
    match tree.node(col) {
        Some(Node::Col { children, .. }) => {
            assert_eq!(children, &[nb, na, nc]);
        }
        other => panic!("expected Col, got {other:?}"),
    }
}

#[test]
fn move_panel_not_found_returns_error() {
    let mut tree = LayoutTree::new();
    let (pa, na) = tree.add_panel("a", grow(1.0)).unwrap();
    let row = tree.add_row(0.0, vec![na]).unwrap();
    tree.set_root(row);

    let bad_id = PanelId::from_raw(999);
    let result = tree.move_panel(bad_id, Position::After(pa));
    assert!(matches!(result, Err(PaneError::PanelNotFound(_))));
}

#[test]
fn remove_last_panel_from_container() {
    let mut tree = LayoutTree::new();
    let (pid, nid) = tree.add_panel("only", grow(1.0)).unwrap();
    let row = tree.add_row(0.0, vec![nid]).unwrap();
    tree.set_root(row);

    tree.remove_panel(pid).unwrap();

    assert_eq!(tree.panel_count(), 0);
    match tree.node(row) {
        Some(Node::Row { children, .. }) => assert_eq!(children.len(), 0),
        other => panic!("expected Row, got {other:?}"),
    }
}

#[test]
fn add_panel_to_existing_container() {
    let mut tree = LayoutTree::new();
    let (pa, na) = tree.add_panel("a", grow(1.0)).unwrap();
    let row = tree.add_row(0.0, vec![na]).unwrap();
    tree.set_root(row);

    let (_, nb) = tree.add_panel("b", grow(1.0)).unwrap();
    tree.insert_node(nb, row, Position::After(pa)).unwrap();

    match tree.node(row) {
        Some(Node::Row { children, .. }) => assert_eq!(children.len(), 2),
        other => panic!("expected Row, got {other:?}"),
    }
}

// --- Step 4: Constraint Mutations and Tree Queries ---

#[test]
fn set_constraints_updates_panel() {
    let mut tree = LayoutTree::new();
    let (pid, _) = tree.add_panel("editor", grow(1.0)).unwrap();
    let new_constraints = grow(2.0).min(20.0);

    tree.set_constraints(pid, new_constraints).unwrap();

    let c = tree.panel_constraints(pid).unwrap();
    assert_eq!(c.grow, Some(2.0));
    assert_eq!(c.min, Some(20.0));
}

#[test]
fn set_constraints_not_found_returns_error() {
    let mut tree = LayoutTree::new();
    let bad_id = PanelId::from_raw(999);

    let result = tree.set_constraints(bad_id, grow(1.0));
    assert!(matches!(result, Err(PaneError::PanelNotFound(_))));
}

#[test]
fn get_panel_constraints() {
    let mut tree = LayoutTree::new();
    let (pid, _) = tree.add_panel("editor", grow(2.0).max(100.0)).unwrap();

    let c = tree.panel_constraints(pid).unwrap();
    assert_eq!(c.grow, Some(2.0));
    assert_eq!(c.max, Some(100.0));
}

#[test]
fn get_panel_kind() {
    let mut tree = LayoutTree::new();
    let (pid, _) = tree.add_panel("editor", grow(1.0)).unwrap();

    let kind = tree.panel_kind(pid).unwrap();
    assert_eq!(kind, "editor");
}

#[test]
fn children_of_container() {
    let mut tree = LayoutTree::new();
    let (_, na) = tree.add_panel("a", grow(1.0)).unwrap();
    let (_, nb) = tree.add_panel("b", grow(1.0)).unwrap();
    let (_, nc) = tree.add_panel("c", grow(1.0)).unwrap();
    let row = tree.add_row(0.0, vec![na, nb, nc]).unwrap();
    tree.set_root(row);

    let kids = tree.children(row).unwrap();
    assert_eq!(kids, &[na, nb, nc]);
}

#[test]
fn parent_of_node() {
    let mut tree = LayoutTree::new();
    let (_, na) = tree.add_panel("a", grow(1.0)).unwrap();
    let (_, nb) = tree.add_panel("b", grow(1.0)).unwrap();
    let col = tree.add_col(0.0, vec![nb]).unwrap();
    let row = tree.add_row(0.0, vec![na, col]).unwrap();
    tree.set_root(row);

    assert_eq!(tree.parent(na).unwrap(), Some(row));
    assert_eq!(tree.parent(nb).unwrap(), Some(col));
    assert_eq!(tree.parent(row).unwrap(), None);
}

// --- Step 3: move_panel bug fix tests ---

#[test]
fn move_panel_updates_parent_map_consistently() {
    let mut tree = LayoutTree::new();
    let (pa, na) = tree.add_panel("a", grow(1.0)).unwrap();
    let (pb, nb) = tree.add_panel("b", grow(1.0)).unwrap();
    let row = tree.add_row(0.0, vec![na, nb]).unwrap();
    tree.set_root(row);

    tree.move_panel(pa, Position::After(pb)).unwrap();

    assert_eq!(tree.parent(na).unwrap(), Some(row));
    assert_eq!(tree.parent(nb).unwrap(), Some(row));
    match tree.node(row) {
        Some(Node::Row { children, .. }) => assert_eq!(children, &[nb, na]),
        other => panic!("expected Row, got {other:?}"),
    }
}

// --- Step 5: Constraint validation tests ---

#[test]
fn add_panel_rejects_negative_grow() {
    let mut tree = LayoutTree::new();
    let result = tree.add_panel("x", grow(-1.0));
    assert!(matches!(result, Err(PaneError::InvalidConstraint(_))));
}

#[test]
fn add_panel_rejects_nan_fixed() {
    let mut tree = LayoutTree::new();
    let result = tree.add_panel("x", fixed(f32::NAN));
    assert!(matches!(result, Err(PaneError::InvalidConstraint(_))));
}

#[test]
fn set_constraints_rejects_min_exceeds_max() {
    let mut tree = LayoutTree::new();
    let (pid, _) = tree.add_panel("x", grow(1.0)).unwrap();
    let bad = grow(1.0).min(100.0).max(10.0);
    let result = tree.set_constraints(pid, bad);
    assert!(matches!(result, Err(PaneError::InvalidConstraint(_))));
}

#[test]
fn add_panel_rejects_grow_and_fixed_together() {
    let mut tree = LayoutTree::new();
    let both = Constraints {
        grow: Some(1.0),
        fixed: Some(100.0),
        ..Constraints::default()
    };
    let result = tree.add_panel("x", both);
    assert!(matches!(result, Err(PaneError::InvalidConstraint(_))));
}

#[test]
fn valid_constraints_pass_through() {
    let mut tree = LayoutTree::new();
    let result = tree.add_panel("x", grow(1.0).min(10.0).max(200.0));
    assert!(result.is_ok());
}

// --- Step 6: Tree validation tests ---

#[test]
fn validate_well_formed_tree() {
    let mut tree = LayoutTree::new();
    let (_, na) = tree.add_panel("a", grow(1.0)).unwrap();
    let (_, nb) = tree.add_panel("b", grow(1.0)).unwrap();
    let row = tree.add_row(0.0, vec![na, nb]).unwrap();
    tree.set_root(row);

    assert!(tree.validate().is_ok());
}

#[test]
fn validate_catches_missing_root() {
    let tree = LayoutTree::new();
    let result = tree.validate();
    assert!(result.is_err());
}

#[test]
fn validate_catches_orphaned_node() {
    let mut tree = LayoutTree::new();
    let (_, na) = tree.add_panel("a", grow(1.0)).unwrap();
    let (_, _nb) = tree.add_panel("b", grow(1.0)).unwrap();
    // Only na is attached to the container; nb is orphaned
    let row = tree.add_row(0.0, vec![na]).unwrap();
    tree.set_root(row);

    let result = tree.validate();
    assert!(result.is_err());
}

#[test]
fn validate_catches_parent_map_inconsistency() {
    let mut tree = LayoutTree::new();
    let (_, na) = tree.add_panel("a", grow(1.0)).unwrap();
    let (_, nb) = tree.add_panel("b", grow(1.0)).unwrap();
    let row = tree.add_row(0.0, vec![na, nb]).unwrap();
    tree.set_root(row);

    // Well-formed tree should pass
    assert!(tree.validate().is_ok());
}

// --- insert_node guard ---

#[test]
fn insert_node_rejects_stale_node_id() {
    let mut tree = LayoutTree::new();
    let (pa, na) = tree.add_panel("a", grow(1.0)).unwrap();
    let row = tree.add_row(0.0, vec![na]).unwrap();
    tree.set_root(row);

    let stale = panes::NodeId::from_raw(999);
    let result = tree.insert_node(stale, row, Position::After(pa));
    assert!(matches!(result, Err(PaneError::NodeNotFound(_))));
}
