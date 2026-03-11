#[cfg(feature = "serde")]
use panes::LayoutSnapshot;
use panes::runtime::LayoutRuntime;
use panes::{Layout, SnapshotSource, StrategyConfig};

#[test]
fn strategy_snapshot_round_trip() {
    let mut rt = Layout::master_stack(["editor", "chat", "status"])
        .master_ratio(0.6)
        .gap(1.0)
        .into_runtime()
        .unwrap();
    rt.focus_next(); // focus "chat"

    let snap = rt.snapshot().unwrap();

    // Verify snapshot contents
    assert_eq!(snap.focused(), Some("chat"));
    match snap.source() {
        SnapshotSource::Strategy { strategy, panels } => {
            assert!(matches!(strategy, StrategyConfig::MasterStack { .. }));
            let kinds: Vec<&str> = panels.iter().map(|s| &**s).collect();
            assert_eq!(kinds, &["editor", "chat", "status"]);
        }
        SnapshotSource::Tree { .. } => panic!("expected Strategy source"),
    }

    // Restore and verify
    let mut rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    assert_eq!(rt2.focused_kind(), Some("chat"));
    let frame = rt2.resolve(800.0, 600.0).unwrap();
    assert_eq!(frame.layout().panels().count(), 3);
}

#[test]
fn strategy_snapshot_preserves_sequence_order() {
    let mut rt = Layout::master_stack(["a", "b", "c"])
        .gap(1.0)
        .into_runtime()
        .unwrap();
    rt.add_panel("d".into()).unwrap();

    let snap = rt.snapshot().unwrap();
    match snap.source() {
        SnapshotSource::Strategy { panels, .. } => {
            assert_eq!(panels.len(), 4);
            assert!(panels.iter().any(|s| &**s == "d"));
        }
        _ => panic!("expected Strategy source"),
    }

    let rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    assert_eq!(rt2.sequence().len(), 4);
}

#[test]
fn tree_snapshot_round_trip() {
    let layout = panes::layout! {
        row(gap: 4.0) {
            panel("editor", grow: 2.0)
            col {
                panel("chat")
                panel("status", fixed: 3.0)
            }
        }
    }
    .unwrap();

    let rt = LayoutRuntime::from(layout);
    let snap = rt.snapshot().unwrap();

    match snap.source() {
        SnapshotSource::Tree { root } => {
            // Root is a row
            assert!(matches!(root, panes::SnapshotNode::Row { .. }));
        }
        _ => panic!("expected Tree source"),
    }

    let mut rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    let frame = rt2.resolve(800.0, 600.0).unwrap();
    assert_eq!(frame.layout().panels().count(), 3);
}

#[test]
fn tree_snapshot_preserves_constraints() {
    let layout = panes::layout! {
        row {
            panel("left", fixed: 100.0)
            panel("right", grow: 1.0)
        }
    }
    .unwrap();

    let rt = LayoutRuntime::from(layout);
    let snap = rt.snapshot().unwrap();

    let mut rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    let frame = rt2.resolve(800.0, 600.0).unwrap();

    // The left panel should be fixed at 100px
    let left = frame.layout().panels().find(|e| e.kind == "left").unwrap();
    assert!((left.rect.w - 100.0).abs() < 1.0);
}

#[test]
fn snapshot_restores_focus() {
    let mut rt = Layout::master_stack(["a", "b", "c"])
        .gap(1.0)
        .into_runtime()
        .unwrap();
    // Focus "c"
    rt.focus_next();
    rt.focus_next();

    let snap = rt.snapshot().unwrap();
    assert_eq!(snap.focused(), Some("c"));

    let rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    assert_eq!(rt2.focused_kind(), Some("c"));
}

#[test]
fn snapshot_restores_collapsed() {
    let mut rt = Layout::master_stack(["editor", "chat", "status"])
        .gap(1.0)
        .into_runtime()
        .unwrap();

    // Collapse "chat"
    let chat_pid = rt
        .sequence()
        .iter()
        .find(|&pid| rt.tree().panel_kind(pid).ok() == Some("chat"))
        .unwrap();
    rt.toggle_collapsed(chat_pid).unwrap();

    let snap = rt.snapshot().unwrap();
    assert!(snap.collapsed().iter().any(|s| &**s == "chat"));

    let rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    let chat_pid2 = rt2
        .sequence()
        .iter()
        .find(|&pid| rt2.tree().panel_kind(pid).ok() == Some("chat"))
        .unwrap();
    assert!(rt2.viewport().collapsed.contains(&chat_pid2));
}

#[test]
fn tabbed_snapshot_round_trip() {
    let rt = Layout::tabbed(["a", "b", "c"]).into_runtime().unwrap();

    let snap = rt.snapshot().unwrap();
    match snap.source() {
        SnapshotSource::Strategy { strategy, panels } => {
            assert!(matches!(strategy, StrategyConfig::ActivePanel { .. }));
            let kinds: Vec<&str> = panels.iter().map(|s| &**s).collect();
            assert_eq!(kinds, &["a", "b", "c"]);
        }
        _ => panic!("expected Strategy"),
    }

    let mut rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    let frame = rt2.resolve(800.0, 600.0).unwrap();
    // Tabbed creates decoration panels (_tab) in addition to content
    assert!(frame.layout().panels().count() > 3);
}

#[test]
fn deck_snapshot_round_trip() {
    let rt = Layout::deck(["a", "b", "c"])
        .master_ratio(0.6)
        .gap(2.0)
        .into_runtime()
        .unwrap();

    let snap = rt.snapshot().unwrap();
    let mut rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    let frame = rt2.resolve(800.0, 600.0).unwrap();
    assert!(frame.layout().panels().count() >= 3);
}

#[test]
fn spiral_snapshot_round_trip() {
    let rt = Layout::spiral(["a", "b", "c", "d"])
        .ratio(0.5)
        .gap(1.0)
        .into_runtime()
        .unwrap();

    let snap = rt.snapshot().unwrap();
    match snap.source() {
        SnapshotSource::Strategy { strategy, .. } => {
            assert!(matches!(
                strategy,
                StrategyConfig::BinarySplit { spiral: true, .. }
            ));
        }
        _ => panic!("expected Strategy"),
    }

    let mut rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    let frame = rt2.resolve(800.0, 600.0).unwrap();
    assert_eq!(frame.layout().panels().count(), 4);
}

#[test]
fn grid_snapshot_round_trip() {
    let rt = Layout::grid(2, ["a", "b", "c", "d"])
        .gap(1.0)
        .into_runtime()
        .unwrap();

    let snap = rt.snapshot().unwrap();
    let mut rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    let frame = rt2.resolve(800.0, 600.0).unwrap();
    assert_eq!(frame.layout().panels().count(), 4);
}

#[test]
fn empty_tree_snapshot_errors() {
    // A runtime from an empty tree has no serializable root
    let tree = panes::LayoutTree::new();
    let rt = LayoutRuntime::new(tree);
    assert!(rt.snapshot().is_err());
}

#[test]
fn nested_tree_snapshot_round_trip() {
    let layout = panes::layout! {
        col(gap: 2.0) {
            row(gap: 1.0) {
                panel("a", grow: 1.0)
                panel("b", grow: 2.0)
            }
            row {
                panel("c", fixed: 50.0)
                col(gap: 3.0) {
                    panel("d")
                    panel("e")
                }
            }
        }
    }
    .unwrap();

    let rt = LayoutRuntime::from(layout);
    let snap = rt.snapshot().unwrap();

    let mut rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    let frame = rt2.resolve(800.0, 600.0).unwrap();
    assert_eq!(frame.layout().panels().count(), 5);

    // Verify the fixed panel preserved its constraint
    let c = frame.layout().panels().find(|e| e.kind == "c").unwrap();
    assert!((c.rect.w - 50.0).abs() < 1.0);
}

// ---------------------------------------------------------------------------
// Serde round-trip tests (JSON serialize → deserialize → restore)
// ---------------------------------------------------------------------------

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;

    fn json_round_trip(snap: &LayoutSnapshot) -> LayoutSnapshot {
        let json = serde_json::to_string(snap).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn strategy_json_round_trip() {
        let mut rt = Layout::master_stack(["editor", "chat", "status"])
            .master_ratio(0.6)
            .gap(1.0)
            .into_runtime()
            .unwrap();
        rt.focus_next();

        let snap = rt.snapshot().unwrap();
        let restored = json_round_trip(&snap);

        assert_eq!(restored.focused(), Some("chat"));
        match restored.source() {
            SnapshotSource::Strategy { panels, .. } => {
                let kinds: Vec<&str> = panels.iter().map(|s| &**s).collect();
                assert_eq!(kinds, &["editor", "chat", "status"]);
            }
            _ => panic!("expected Strategy"),
        }

        let mut rt2 = LayoutRuntime::from_snapshot(restored).unwrap();
        let frame = rt2.resolve(800.0, 600.0).unwrap();
        assert_eq!(frame.layout().panels().count(), 3);
        assert_eq!(rt2.focused_kind(), Some("chat"));
    }

    #[test]
    fn tree_json_round_trip() {
        let layout = panes::layout! {
            row(gap: 4.0) {
                panel("left", fixed: 100.0)
                col {
                    panel("top", grow: 2.0)
                    panel("bottom", fixed: 30.0)
                }
            }
        }
        .unwrap();

        let rt = LayoutRuntime::from(layout);
        let snap = rt.snapshot().unwrap();
        let restored = json_round_trip(&snap);

        let mut rt2 = LayoutRuntime::from_snapshot(restored).unwrap();
        let frame = rt2.resolve(800.0, 600.0).unwrap();
        assert_eq!(frame.layout().panels().count(), 3);

        let left = frame.layout().panels().find(|e| e.kind == "left").unwrap();
        assert!((left.rect.w - 100.0).abs() < 1.0);
    }

    #[test]
    fn tabbed_json_round_trip() {
        let rt = Layout::tabbed(["a", "b", "c"]).into_runtime().unwrap();
        let snap = rt.snapshot().unwrap();
        let restored = json_round_trip(&snap);

        match restored.source() {
            SnapshotSource::Strategy { strategy, panels } => {
                assert!(matches!(strategy, StrategyConfig::ActivePanel { .. }));
                let kinds: Vec<&str> = panels.iter().map(|s| &**s).collect();
                assert_eq!(kinds, &["a", "b", "c"]);
            }
            _ => panic!("expected Strategy"),
        }

        let mut rt2 = LayoutRuntime::from_snapshot(restored).unwrap();
        let frame = rt2.resolve(800.0, 600.0).unwrap();
        assert!(frame.layout().panels().count() > 3);
    }

    #[test]
    fn collapsed_json_round_trip() {
        let mut rt = Layout::master_stack(["a", "b", "c"])
            .gap(1.0)
            .into_runtime()
            .unwrap();

        let b_pid = rt
            .sequence()
            .iter()
            .find(|&pid| rt.tree().panel_kind(pid).ok() == Some("b"))
            .unwrap();
        rt.toggle_collapsed(b_pid).unwrap();

        let snap = rt.snapshot().unwrap();
        let restored = json_round_trip(&snap);
        assert!(restored.collapsed().iter().any(|s| &**s == "b"));

        let rt2 = LayoutRuntime::from_snapshot(restored).unwrap();
        let b_pid2 = rt2
            .sequence()
            .iter()
            .find(|&pid| rt2.tree().panel_kind(pid).ok() == Some("b"))
            .unwrap();
        assert!(rt2.viewport().collapsed.contains(&b_pid2));
    }

    #[test]
    fn json_output_is_readable() {
        let rt = Layout::master_stack(["editor", "chat"])
            .master_ratio(0.6)
            .gap(1.0)
            .into_runtime()
            .unwrap();
        let snap = rt.snapshot().unwrap();
        let json = serde_json::to_string_pretty(&snap).unwrap();

        // Verify key fields appear in the JSON
        assert!(json.contains("MasterStack"));
        assert!(json.contains("editor"));
        assert!(json.contains("chat"));
        assert!(json.contains("master_ratio"));
    }
}
