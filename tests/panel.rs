use panes::{Constraints, PanelIdGenerator, fixed, grow};
use std::collections::HashSet;

#[test]
fn panel_id_auto_increments() {
    let mut id_gen = PanelIdGenerator::new();
    let a = id_gen.next_id().unwrap();
    let b = id_gen.next_id().unwrap();
    let c = id_gen.next_id().unwrap();
    assert_ne!(a, b);
    assert_ne!(b, c);
    assert_ne!(a, c);
}

#[test]
fn panel_id_is_copy_and_hashable() {
    let mut id_gen = PanelIdGenerator::new();
    let id = id_gen.next_id().unwrap();
    let copy = id;
    let mut set = HashSet::new();
    set.insert(id);
    set.insert(copy);
    assert_eq!(set.len(), 1);
}

#[test]
fn grow_creates_correct_constraint() {
    let c = grow(2.0);
    assert_eq!(c.grow, Some(2.0));
    assert_eq!(c.fixed, None);
    assert_eq!(c.min, None);
    assert_eq!(c.max, None);
}

#[test]
fn fixed_creates_correct_constraint() {
    let c = fixed(3.0);
    assert_eq!(c.fixed, Some(3.0));
    assert_eq!(c.grow, None);
    assert_eq!(c.min, None);
    assert_eq!(c.max, None);
}

#[test]
fn constraints_chain_min_max() {
    let c = grow(1.0).min(10.0).max(100.0);
    assert_eq!(c.grow, Some(1.0));
    assert_eq!(c.min, Some(10.0));
    assert_eq!(c.max, Some(100.0));
}

#[test]
fn default_constraints_are_empty() {
    let c = Constraints::default();
    assert_eq!(c.grow, None);
    assert_eq!(c.fixed, None);
    assert_eq!(c.min, None);
    assert_eq!(c.max, None);
}

#[test]
fn panel_kind_accepts_str_literal() {
    let kind: std::sync::Arc<str> = "editor".into();
    assert_eq!(&*kind, "editor");
}

#[test]
fn panel_kind_accepts_string() {
    let kind: std::sync::Arc<str> = String::from("editor").into();
    assert_eq!(&*kind, "editor");
}
