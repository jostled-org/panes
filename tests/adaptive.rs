use std::sync::Arc;

use panes::{Layout, Strategy};

#[test]
fn basic_breakpoint_selection() {
    let mut rt = Layout::adaptive(["a", "b", "c"])
        .at(0, Strategy::stacked())
        .at(600, Strategy::master_stack().master_ratio(0.6).gap(1.0))
        .at(1200, Strategy::dashboard())
        .into_runtime()
        .unwrap();

    // Narrow → stacked (bp 0)
    rt.resolve(400.0, 300.0).unwrap();
    assert_eq!(rt.active_breakpoint_index(), 0);

    // Medium → master-stack (bp 1)
    rt.resolve(800.0, 600.0).unwrap();
    assert_eq!(rt.active_breakpoint_index(), 1);

    // Wide → columns (bp 2)
    rt.resolve(1400.0, 900.0).unwrap();
    assert_eq!(rt.active_breakpoint_index(), 2);
}

#[test]
fn breakpoint_switch_preserves_panels() {
    let mut rt = Layout::adaptive(["editor", "chat", "status"])
        .at(0, Strategy::stacked())
        .at(600, Strategy::dashboard())
        .into_runtime()
        .unwrap();

    rt.resolve(400.0, 300.0).unwrap();
    let kinds_narrow: Vec<String> = rt
        .sequence()
        .iter()
        .filter_map(|pid| rt.tree().panel_kind(pid).ok().map(String::from))
        .collect();

    rt.resolve(800.0, 600.0).unwrap();
    let kinds_wide: Vec<String> = rt
        .sequence()
        .iter()
        .filter_map(|pid| rt.tree().panel_kind(pid).ok().map(String::from))
        .collect();

    assert_eq!(kinds_narrow, vec!["editor", "chat", "status"]);
    assert_eq!(kinds_wide, vec!["editor", "chat", "status"]);
}

#[test]
fn focus_preserved_across_switch() {
    let mut rt = Layout::adaptive(["a", "b", "c"])
        .at(0, Strategy::stacked())
        .at(600, Strategy::dashboard())
        .into_runtime()
        .unwrap();

    rt.resolve(400.0, 300.0).unwrap();
    // Focus "b"
    let b_pid = rt
        .sequence()
        .iter()
        .find(|&pid| rt.tree().panel_kind(pid).ok() == Some("b"))
        .unwrap();
    rt.focus(b_pid);
    assert_eq!(rt.focused_kind(), Some("b"));

    // Switch breakpoint
    rt.resolve(800.0, 600.0).unwrap();
    assert_eq!(rt.focused_kind(), Some("b"));
}

#[test]
fn add_panel_survives_switch() {
    let mut rt = Layout::adaptive(["a", "b"])
        .at(0, Strategy::stacked())
        .at(600, Strategy::dashboard())
        .into_runtime()
        .unwrap();

    rt.resolve(800.0, 600.0).unwrap();
    rt.add_panel(Arc::from("c")).unwrap();

    let count_wide = rt.sequence().len();

    rt.resolve(400.0, 300.0).unwrap();
    let kinds: Vec<String> = rt
        .sequence()
        .iter()
        .filter_map(|pid| rt.tree().panel_kind(pid).ok().map(String::from))
        .collect();

    assert_eq!(count_wide, 3);
    assert!(kinds.contains(&"c".to_string()));
}

#[test]
fn remove_panel_survives_switch() {
    let mut rt = Layout::adaptive(["a", "b", "c"])
        .at(0, Strategy::stacked())
        .at(600, Strategy::dashboard())
        .into_runtime()
        .unwrap();

    rt.resolve(800.0, 600.0).unwrap();
    let b_pid = rt
        .sequence()
        .iter()
        .find(|&pid| rt.tree().panel_kind(pid).ok() == Some("b"))
        .unwrap();
    rt.remove_panel(b_pid).unwrap();

    rt.resolve(400.0, 300.0).unwrap();
    let kinds: Vec<String> = rt
        .sequence()
        .iter()
        .filter_map(|pid| rt.tree().panel_kind(pid).ok().map(String::from))
        .collect();

    assert_eq!(kinds.len(), 2);
    assert!(!kinds.contains(&"b".to_string()));
}

#[test]
fn snapshot_round_trip() {
    let mut rt = Layout::adaptive(["x", "y"])
        .at(0, Strategy::stacked())
        .at(800, Strategy::master_stack())
        .into_runtime()
        .unwrap();

    rt.resolve(1000.0, 600.0).unwrap();
    let snap = rt.snapshot().unwrap();

    let mut rt2 = panes::runtime::LayoutRuntime::from_snapshot(snap).unwrap();
    assert!(rt2.breakpoints().is_some());
    assert_eq!(rt2.breakpoints().unwrap().len(), 2);

    let kinds: Vec<String> = rt2
        .sequence()
        .iter()
        .filter_map(|pid| rt2.tree().panel_kind(pid).ok().map(String::from))
        .collect();
    assert_eq!(kinds, vec!["x", "y"]);

    // Verify the restored runtime resolves
    rt2.resolve(500.0, 400.0).unwrap();
    assert_eq!(rt2.active_breakpoint_index(), 0);
}

#[test]
fn single_breakpoint() {
    let mut rt = Layout::adaptive(["a", "b"])
        .at(0, Strategy::dashboard())
        .into_runtime()
        .unwrap();

    rt.resolve(800.0, 600.0).unwrap();
    assert_eq!(rt.active_breakpoint_index(), 0);
    assert_eq!(rt.sequence().len(), 2);
}

#[test]
fn same_breakpoint_no_rebuild() {
    let mut rt = Layout::adaptive(["a", "b"])
        .at(0, Strategy::stacked())
        .at(600, Strategy::dashboard())
        .into_runtime()
        .unwrap();

    rt.resolve(800.0, 600.0).unwrap();
    rt.resolve(900.0, 600.0).unwrap();

    // Same breakpoint — diff should show no topology change
    let diff = rt.last_diff();
    assert!(diff.added.is_empty());
    assert!(diff.removed.is_empty());
}

#[cfg(feature = "serde")]
#[test]
fn empty_breakpoints_snapshot_returns_error() {
    let json = r#"{
        "source": {
            "Adaptive": {
                "breakpoints": [],
                "panels": ["a", "b"],
                "active_index": 0
            }
        },
        "focused": null,
        "collapsed": []
    }"#;
    let snap: panes::LayoutSnapshot = serde_json::from_str(json).unwrap();
    let result = panes::runtime::LayoutRuntime::from_snapshot(snap);
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(
        matches!(err, panes::PaneError::InvalidTree(ref e) if e.to_string().contains("breakpoint"))
    );
}

#[cfg(feature = "toml")]
#[test]
fn toml_breakpoints() {
    let toml = r#"
[layout]
strategy = "stacked"
panels = ["a", "b", "c"]

[[layout.breakpoints]]
min_width = 0
strategy = "stacked"

[[layout.breakpoints]]
min_width = 800
strategy = "master-stack"
master_ratio = 0.6
gap = 2.0
"#;
    let mut rt = Layout::from_toml_runtime(toml).unwrap();
    assert!(rt.breakpoints().is_some());
    assert_eq!(rt.breakpoints().unwrap().len(), 2);

    rt.resolve(1000.0, 600.0).unwrap();
    assert_eq!(rt.active_breakpoint_index(), 1);
}
