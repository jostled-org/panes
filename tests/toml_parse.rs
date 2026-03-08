#![cfg(feature = "toml")]

use panes::{Layout, TomlError};

// -- Error cases --

#[test]
fn unknown_strategy_returns_error() {
    let toml = r#"
[layout]
strategy = "nonexistent"
panels = ["a"]
"#;
    let err = Layout::from_toml(toml).unwrap_err();
    assert!(matches!(err, TomlError::UnknownStrategy(ref s) if s.as_ref() == "nonexistent"));
}

#[test]
fn malformed_toml_returns_parse_error() {
    let err = Layout::from_toml("not valid toml [[[").unwrap_err();
    assert!(matches!(err, TomlError::Parse(_)));
}

#[test]
fn missing_strategy_returns_parse_error() {
    let toml = r#"
[layout]
panels = ["a"]
"#;
    let err = Layout::from_toml(toml).unwrap_err();
    assert!(matches!(err, TomlError::Parse(_)));
}

#[test]
fn missing_panels_returns_error() {
    let toml = r#"
[layout]
strategy = "master-stack"
"#;
    let err = Layout::from_toml(toml).unwrap_err();
    assert!(matches!(err, TomlError::MissingField(ref f) if f.as_ref() == "panels"));
}

#[test]
fn empty_panels_returns_error() {
    let toml = r#"
[layout]
strategy = "master-stack"
panels = []
"#;
    let err = Layout::from_toml(toml).unwrap_err();
    assert!(matches!(err, TomlError::InvalidValue { .. }));
}

// -- master-stack --

#[test]
fn master_stack_basic() {
    let toml = r#"
[layout]
strategy = "master-stack"
panels = ["editor", "chat", "status"]
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 3);
}

#[test]
fn master_stack_with_options() {
    let toml = r#"
[layout]
strategy = "master-stack"
panels = ["editor", "chat"]
master_ratio = 0.7
gap = 2.0
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(100.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 2);
}

// -- centered-master --

#[test]
fn centered_master_basic() {
    let toml = r#"
[layout]
strategy = "centered-master"
panels = ["main", "left", "right"]
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 3);
}

#[test]
fn centered_master_with_options() {
    let toml = r#"
[layout]
strategy = "centered-master"
panels = ["main", "a", "b", "c"]
master_ratio = 0.6
gap = 1.0
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 4);
}

// -- monocle --

#[test]
fn monocle_basic() {
    let toml = r#"
[layout]
strategy = "monocle"
panels = ["a", "b", "c"]
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 3);
}

#[test]
fn monocle_with_active() {
    let toml = r#"
[layout]
strategy = "monocle"
panels = ["a", "b", "c"]
active = 1
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 3);
}

// -- scrollable --

#[test]
fn scrollable_basic() {
    let toml = r#"
[layout]
strategy = "scrollable"
panels = ["a", "b", "c"]
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(200.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 3);
}

#[test]
fn scrollable_with_options() {
    let toml = r#"
[layout]
strategy = "scrollable"
panels = ["a", "b"]
active = 1
gap = 2.0
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(200.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 2);
}

// -- dwindle --

#[test]
fn dwindle_basic() {
    let toml = r#"
[layout]
strategy = "dwindle"
panels = ["a", "b", "c"]
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 80.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 3);
}

#[test]
fn dwindle_with_options() {
    let toml = r#"
[layout]
strategy = "dwindle"
panels = ["a", "b", "c", "d"]
ratio = 0.6
gap = 1.0
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 80.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 4);
}

// -- spiral --

#[test]
fn spiral_basic() {
    let toml = r#"
[layout]
strategy = "spiral"
panels = ["a", "b", "c"]
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 80.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 3);
}

#[test]
fn spiral_with_options() {
    let toml = r#"
[layout]
strategy = "spiral"
panels = ["a", "b", "c", "d"]
ratio = 0.6
gap = 1.0
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 80.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 4);
}

// -- deck --

#[test]
fn deck_basic() {
    let toml = r#"
[layout]
strategy = "deck"
panels = ["main", "a", "b"]
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 3);
}

#[test]
fn deck_with_options() {
    let toml = r#"
[layout]
strategy = "deck"
panels = ["main", "a", "b"]
master_ratio = 0.6
active = 1
gap = 1.0
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 3);
}

// -- tabbed --

#[test]
fn tabbed_basic() {
    let toml = r#"
[layout]
strategy = "tabbed"
panels = ["a", "b", "c"]
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();
    // tabbed creates tab panels + content panels
    assert_eq!(resolved.panel_ids().count(), 6);
}

#[test]
fn tabbed_with_options() {
    let toml = r#"
[layout]
strategy = "tabbed"
panels = ["a", "b"]
active = 1
tab_height = 2.0
gap = 1.0
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 4);
}

// -- stacked --

#[test]
fn stacked_basic() {
    let toml = r#"
[layout]
strategy = "stacked"
panels = ["a", "b", "c"]
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();
    // stacked creates title panels + content panels
    assert_eq!(resolved.panel_ids().count(), 6);
}

#[test]
fn stacked_with_options() {
    let toml = r#"
[layout]
strategy = "stacked"
panels = ["a", "b"]
active = 1
title_height = 2.0
gap = 1.0
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 4);
}

// -- columns --

#[test]
fn columns_basic() {
    let toml = r#"
[layout]
strategy = "columns"
columns = 3
panels = ["a", "b", "c", "d", "e", "f"]
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 6);
}

#[test]
fn columns_with_gap() {
    let toml = r#"
[layout]
strategy = "columns"
columns = 2
panels = ["a", "b", "c"]
gap = 2.0
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 3);
}

#[test]
fn columns_missing_columns_field() {
    let toml = r#"
[layout]
strategy = "columns"
panels = ["a", "b"]
"#;
    let err = Layout::from_toml(toml).unwrap_err();
    assert!(matches!(err, TomlError::MissingField(ref f) if f.as_ref() == "columns"));
}

// -- grid --

#[test]
fn grid_basic() {
    let toml = r#"
[layout]
strategy = "grid"
columns = 3
panels = ["a", "b", "c", "d", "e", "f"]
gap = 1.0
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 80.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 6);
}

#[test]
fn grid_missing_columns_field() {
    let toml = r#"
[layout]
strategy = "grid"
panels = ["a", "b"]
"#;
    let err = Layout::from_toml(toml).unwrap_err();
    assert!(matches!(err, TomlError::MissingField(ref f) if f.as_ref() == "columns"));
}

// -- sidebar --

#[test]
fn sidebar_basic() {
    let toml = r#"
[layout]
strategy = "sidebar"
sidebar = "nav"
content = "main"
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 2);
}

#[test]
fn sidebar_with_options() {
    let toml = r#"
[layout]
strategy = "sidebar"
sidebar = "nav"
content = "main"
sidebar_width = 30.0
gap = 1.0
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 2);
}

#[test]
fn sidebar_missing_sidebar_field() {
    let toml = r#"
[layout]
strategy = "sidebar"
content = "main"
"#;
    let err = Layout::from_toml(toml).unwrap_err();
    assert!(matches!(err, TomlError::MissingField(ref f) if f.as_ref() == "sidebar"));
}

#[test]
fn sidebar_missing_content_field() {
    let toml = r#"
[layout]
strategy = "sidebar"
sidebar = "nav"
"#;
    let err = Layout::from_toml(toml).unwrap_err();
    assert!(matches!(err, TomlError::MissingField(ref f) if f.as_ref() == "content"));
}

// -- split --

#[test]
fn split_basic() {
    let toml = r#"
[layout]
strategy = "split"
first = "editor"
second = "terminal"
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 2);
}

#[test]
fn split_vertical_with_options() {
    let toml = r#"
[layout]
strategy = "split"
first = "editor"
second = "terminal"
ratio = 0.6
direction = "vertical"
gap = 1.0
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 2);
}

#[test]
fn split_horizontal_explicit() {
    let toml = r#"
[layout]
strategy = "split"
first = "a"
second = "b"
direction = "horizontal"
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 2);
}

#[test]
fn split_invalid_direction() {
    let toml = r#"
[layout]
strategy = "split"
first = "a"
second = "b"
direction = "diagonal"
"#;
    let err = Layout::from_toml(toml).unwrap_err();
    assert!(
        matches!(err, TomlError::InvalidValue { ref field, .. } if field.as_ref() == "direction")
    );
}

#[test]
fn split_missing_first() {
    let toml = r#"
[layout]
strategy = "split"
second = "b"
"#;
    let err = Layout::from_toml(toml).unwrap_err();
    assert!(matches!(err, TomlError::MissingField(ref f) if f.as_ref() == "first"));
}

// -- holy-grail --

#[test]
fn holy_grail_basic() {
    let toml = r#"
[layout]
strategy = "holy-grail"
header = "header"
footer = "footer"
left = "nav"
main = "content"
right = "aside"
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 5);
}

#[test]
fn holy_grail_with_options() {
    let toml = r#"
[layout]
strategy = "holy-grail"
header = "header"
footer = "footer"
left = "nav"
main = "content"
right = "aside"
header_height = 3.0
footer_height = 1.0
sidebar_width = 20.0
gap = 1.0
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 5);
}

#[test]
fn holy_grail_missing_main() {
    let toml = r#"
[layout]
strategy = "holy-grail"
header = "header"
footer = "footer"
left = "nav"
right = "aside"
"#;
    let err = Layout::from_toml(toml).unwrap_err();
    assert!(matches!(err, TomlError::MissingField(ref f) if f.as_ref() == "main"));
}

// -- dashboard --

#[test]
fn dashboard_with_cards() {
    let toml = r#"
[layout]
strategy = "dashboard"
columns = 3
gap = 1.0

[[layout.panels]]
kind = "chart"
span = 2

[[layout.panels]]
kind = "stats"
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 80.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 2);
}

#[test]
fn dashboard_with_string_panels() {
    let toml = r#"
[layout]
strategy = "dashboard"
columns = 2
panels = ["a", "b", "c", "d"]
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 80.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 4);
}

#[test]
fn dashboard_missing_panels() {
    let toml = r#"
[layout]
strategy = "dashboard"
columns = 3
"#;
    let err = Layout::from_toml(toml).unwrap_err();
    assert!(matches!(err, TomlError::MissingField(ref f) if f.as_ref() == "panels"));
}

// -- custom tree --

#[test]
fn custom_simple_row() {
    let toml = r#"
[layout]
strategy = "custom"

[layout.root]
type = "row"
gap = 4.0

[[layout.root.children]]
kind = "editor"
grow = 2.0

[[layout.root.children]]
kind = "sidebar"
fixed = 30.0
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(100.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 2);
    let sidebar_ids = resolved.by_kind("sidebar");
    assert_eq!(sidebar_ids.len(), 1);
    let sidebar_rect = resolved.get(sidebar_ids[0]).unwrap();
    assert!((sidebar_rect.w - 30.0).abs() < 1.0);
}

#[test]
fn custom_nested_tree() {
    let toml = r#"
[layout]
strategy = "custom"

[layout.root]
type = "row"
gap = 8.0

[[layout.root.children]]
kind = "editor"
grow = 2.0

[[layout.root.children]]
type = "col"

[[layout.root.children.children]]
kind = "chat"

[[layout.root.children.children]]
kind = "status"
fixed = 3.0
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 3);
    let status_ids = resolved.by_kind("status");
    assert_eq!(status_ids.len(), 1);
    let status_rect = resolved.get(status_ids[0]).unwrap();
    assert!((status_rect.h - 3.0).abs() < 1.0);
}

#[test]
fn custom_with_min_max_constraints() {
    let toml = r#"
[layout]
strategy = "custom"

[layout.root]
type = "row"

[[layout.root.children]]
kind = "main"
grow = 1.0
min = 20.0
max = 80.0
"#;
    let layout = Layout::from_toml(toml).unwrap();
    let resolved = layout.resolve(200.0, 40.0).unwrap();
    let main_ids = resolved.by_kind("main");
    assert_eq!(main_ids.len(), 1);
    let main_rect = resolved.get(main_ids[0]).unwrap();
    assert!(main_rect.w <= 80.0 + 0.01);
}

#[test]
fn custom_missing_root() {
    let toml = r#"
[layout]
strategy = "custom"
"#;
    let err = Layout::from_toml(toml).unwrap_err();
    assert!(matches!(err, TomlError::MissingField(ref f) if f.as_ref() == "root"));
}

#[test]
fn custom_node_both_kind_and_type() {
    let toml = r#"
[layout]
strategy = "custom"

[layout.root]
type = "row"

[[layout.root.children]]
kind = "editor"
type = "col"
"#;
    let err = Layout::from_toml(toml).unwrap_err();
    assert!(matches!(err, TomlError::InvalidValue { .. }));
}

#[test]
fn custom_node_neither_kind_nor_type() {
    let toml = r#"
[layout]
strategy = "custom"

[layout.root]
type = "row"

[[layout.root.children]]
grow = 1.0
"#;
    let err = Layout::from_toml(toml).unwrap_err();
    assert!(matches!(err, TomlError::InvalidValue { .. }));
}

#[test]
fn custom_root_missing_type() {
    let toml = r#"
[layout]
strategy = "custom"

[layout.root]
kind = "panel"
"#;
    let err = Layout::from_toml(toml).unwrap_err();
    assert!(matches!(err, TomlError::InvalidValue { .. }));
}

// -- Equivalence tests: TOML-parsed vs Rust API --

fn assert_rects_equal(toml_layout: &Layout, api_layout: &Layout, kinds: &[&str], w: f32, h: f32) {
    let toml_resolved = toml_layout.resolve(w, h).unwrap();
    let api_resolved = api_layout.resolve(w, h).unwrap();
    for kind in kinds {
        let toml_ids = toml_resolved.by_kind(kind);
        let api_ids = api_resolved.by_kind(kind);
        assert_eq!(
            toml_ids.len(),
            api_ids.len(),
            "panel count mismatch for {kind}"
        );
        let toml_rect = toml_resolved.get(toml_ids[0]).unwrap();
        let api_rect = api_resolved.get(api_ids[0]).unwrap();
        assert!(
            (toml_rect.x - api_rect.x).abs() < 0.01,
            "x mismatch for {kind}"
        );
        assert!(
            (toml_rect.y - api_rect.y).abs() < 0.01,
            "y mismatch for {kind}"
        );
        assert!(
            (toml_rect.w - api_rect.w).abs() < 0.01,
            "w mismatch for {kind}"
        );
        assert!(
            (toml_rect.h - api_rect.h).abs() < 0.01,
            "h mismatch for {kind}"
        );
    }
}

#[test]
fn equivalence_master_stack() {
    let toml_layout = Layout::from_toml(
        r#"
[layout]
strategy = "master-stack"
panels = ["editor", "chat", "status"]
master_ratio = 0.6
gap = 1.0
"#,
    )
    .unwrap();

    let api_layout = Layout::master_stack(["editor", "chat", "status"])
        .master_ratio(0.6)
        .gap(1.0)
        .build()
        .unwrap();

    assert_rects_equal(
        &toml_layout,
        &api_layout,
        &["editor", "chat", "status"],
        120.0,
        40.0,
    );
}

#[test]
fn equivalence_sidebar() {
    let toml_layout = Layout::from_toml(
        r#"
[layout]
strategy = "sidebar"
sidebar = "nav"
content = "main"
sidebar_width = 25.0
gap = 2.0
"#,
    )
    .unwrap();

    let api_layout = Layout::sidebar("nav", "main")
        .sidebar_width(25.0)
        .gap(2.0)
        .build()
        .unwrap();

    assert_rects_equal(&toml_layout, &api_layout, &["nav", "main"], 100.0, 50.0);
}

#[test]
fn equivalence_split_vertical() {
    let toml_layout = Layout::from_toml(
        r#"
[layout]
strategy = "split"
first = "top"
second = "bottom"
ratio = 0.7
direction = "vertical"
"#,
    )
    .unwrap();

    let api_layout = Layout::split("top", "bottom")
        .ratio(0.7)
        .vertical()
        .build()
        .unwrap();

    assert_rects_equal(&toml_layout, &api_layout, &["top", "bottom"], 80.0, 60.0);
}

// -- from_toml_file --

#[test]
fn from_toml_file_reads_and_parses() {
    let dir = std::env::temp_dir().join("panes_test_from_toml_file");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("layout.toml");
    std::fs::write(
        &path,
        r#"
[layout]
strategy = "split"
first = "a"
second = "b"
"#,
    )
    .unwrap();
    let layout = Layout::from_toml_file(&path).unwrap();
    let resolved = layout.resolve(100.0, 50.0).unwrap();
    assert_eq!(resolved.panel_ids().count(), 2);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn from_toml_file_missing_file_returns_io_error() {
    let err = Layout::from_toml_file("/tmp/panes_nonexistent_42.toml").unwrap_err();
    assert!(matches!(err, TomlError::Io(_)));
}
