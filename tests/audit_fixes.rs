use panes::{Layout, LayoutTree, PaneError, grow};

// -- Step 1: tree alloc returns Result --

#[test]
fn tree_alloc_normal_case_succeeds() {
    let mut tree = LayoutTree::new();
    let (pid, nid) = tree.add_panel("editor", grow(1.0)).unwrap();
    let row = tree.add_row(0.0, vec![nid]).unwrap();
    tree.set_root(row);
    tree.validate().unwrap();

    assert!(tree.node(nid).is_some());
    assert_eq!(tree.panel_count(), 1);
    assert_eq!(pid.raw(), 0);
}

// -- Step 3: active index out of bounds --

#[test]
fn monocle_active_out_of_bounds() {
    let err = Layout::monocle(["a", "b", "c"])
        .active(5)
        .build()
        .unwrap_err();
    assert!(matches!(err, PaneError::InvalidTree(_)));
}

#[test]
fn deck_active_out_of_bounds() {
    let err = Layout::deck(["a", "b", "c"]).active(5).build().unwrap_err();
    assert!(matches!(err, PaneError::InvalidTree(_)));
}

#[test]
fn tabbed_active_out_of_bounds() {
    let err = Layout::tabbed(["a", "b", "c"])
        .active(5)
        .build()
        .unwrap_err();
    assert!(matches!(err, PaneError::InvalidTree(_)));
}

#[test]
fn stacked_active_out_of_bounds() {
    let err = Layout::stacked(["a", "b", "c"])
        .active(5)
        .build()
        .unwrap_err();
    assert!(matches!(err, PaneError::InvalidTree(_)));
}

// -- Step 3: grid cols == 0 --

#[test]
fn grid_zero_cols() {
    let err = Layout::grid(0, ["a", "b"]).build().unwrap_err();
    assert!(matches!(err, PaneError::InvalidTree(_)));
}

// -- node_count maintained counter --

#[test]
fn node_count_tracks_additions_and_removals() {
    let mut tree = LayoutTree::new();
    assert_eq!(tree.node_count(), 0);

    let (pid, nid) = tree.add_panel("a", grow(1.0)).unwrap();
    let (pid2, nid2) = tree.add_panel("b", grow(1.0)).unwrap();
    let row = tree.add_row(0.0, vec![nid, nid2]).unwrap();
    tree.set_root(row);
    assert_eq!(tree.node_count(), 3);

    tree.remove_panel(pid).unwrap();
    assert_eq!(tree.node_count(), 2);

    tree.remove_panel(pid2).unwrap();
    assert_eq!(tree.node_count(), 1);
}

// -- dashboard span overflow --

#[test]
fn dashboard_span_overflow_u16() {
    let err = Layout::dashboard([("big", usize::from(u16::MAX) + 1)])
        .build()
        .unwrap_err();
    assert!(matches!(err, PaneError::InvalidConstraint(_)));
}

// -- f32 param validation: NaN, negative, infinite --

#[test]
fn master_stack_nan_ratio() {
    let err = Layout::master_stack(["a", "b"])
        .master_ratio(f32::NAN)
        .build()
        .unwrap_err();
    assert!(matches!(err, PaneError::InvalidConstraint(_)));
}

#[test]
fn split_negative_ratio() {
    let err = Layout::split("a", "b").ratio(-0.5).build().unwrap_err();
    assert!(matches!(err, PaneError::InvalidConstraint(_)));
}

#[test]
fn sidebar_infinite_width() {
    let err = Layout::sidebar("nav", "main")
        .sidebar_width(f32::INFINITY)
        .build()
        .unwrap_err();
    assert!(matches!(err, PaneError::InvalidConstraint(_)));
}

#[test]
fn holy_grail_nan_header() {
    let err = Layout::holy_grail("h", "f", "l", "m", "r")
        .header_height(f32::NAN)
        .build()
        .unwrap_err();
    assert!(matches!(err, PaneError::InvalidConstraint(_)));
}

#[test]
fn scrollable_active_out_of_bounds() {
    let err = Layout::scrollable(["a"]).active(5).build().unwrap_err();
    assert!(matches!(err, PaneError::InvalidTree(_)));
}

#[test]
fn tabbed_infinite_tab_height() {
    let err = Layout::tabbed(["a", "b"])
        .tab_height(f32::INFINITY)
        .build()
        .unwrap_err();
    assert!(matches!(err, PaneError::InvalidConstraint(_)));
}

#[test]
fn stacked_nan_title_height() {
    let err = Layout::stacked(["a", "b"])
        .title_height(f32::NAN)
        .build()
        .unwrap_err();
    assert!(matches!(err, PaneError::InvalidConstraint(_)));
}

#[test]
fn dwindle_negative_ratio() {
    let err = Layout::dwindle(["a", "b"]).ratio(-1.0).build().unwrap_err();
    assert!(matches!(err, PaneError::InvalidConstraint(_)));
}

#[test]
fn spiral_infinite_ratio() {
    let err = Layout::spiral(["a", "b"])
        .ratio(f32::INFINITY)
        .build()
        .unwrap_err();
    assert!(matches!(err, PaneError::InvalidConstraint(_)));
}

// -- empty kinds / cards validation --

#[test]
fn columns_empty_kinds() {
    let err = Layout::columns(2, std::iter::empty::<&str>())
        .build()
        .unwrap_err();
    assert!(matches!(err, PaneError::InvalidTree(_)));
}

#[test]
fn dashboard_empty_cards() {
    let err = Layout::dashboard(std::iter::empty::<(&str, usize)>())
        .build()
        .unwrap_err();
    assert!(matches!(err, PaneError::InvalidTree(_)));
}

#[test]
fn dashboard_zero_columns() {
    let err = Layout::dashboard([("a", 1)])
        .columns(0)
        .build()
        .unwrap_err();
    assert!(matches!(err, PaneError::InvalidTree(_)));
}
