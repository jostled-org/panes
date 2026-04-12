#![allow(clippy::unwrap_used, clippy::panic)]
use panes::Layout;

#[test]
fn layout_tree_accessor_returns_correct_panel_count() {
    let layout = Layout::row(["a", "b", "c"]).unwrap();
    assert_eq!(layout.tree().panel_count(), 3);
}
