use panes::{NodeId, PanelId, PanelIdGenerator};
use std::collections::HashSet;

#[test]
fn node_id_from_raw_and_back() {
    let id = NodeId::from_raw(42);
    assert_eq!(id.raw(), 42);
}

#[test]
fn node_id_display_shows_inner_value() {
    let id = NodeId::from_raw(7);
    assert_eq!(format!("{id}"), "7");
}

#[test]
fn node_id_is_copy_and_hashable() {
    let id = NodeId::from_raw(1);
    let copy = id;
    let mut set = HashSet::new();
    set.insert(id);
    set.insert(copy);
    assert_eq!(set.len(), 1);
}

#[test]
fn panel_id_still_works_after_macro_migration() {
    let mut id_gen = PanelIdGenerator::new();
    let id = id_gen.next_id().unwrap();
    let raw = id.raw();
    let round_tripped = PanelId::from_raw(raw);
    assert_eq!(id, round_tripped);
    assert_eq!(format!("{id}"), format!("{raw}"));
}

#[test]
fn node_id_and_panel_id_are_distinct_types() {
    fn accepts_panel_id(_id: PanelId) {}
    // NodeId::from_raw(1) cannot be passed to accepts_panel_id — type system enforces this.
    // This test verifies PanelId works; the type distinction is a compile-time guarantee.
    accepts_panel_id(PanelId::from_raw(1));
}
