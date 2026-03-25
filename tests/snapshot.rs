#[cfg(feature = "serde")]
use panes::LayoutSnapshot;
use panes::runtime::LayoutRuntime;
use panes::{GridColumnMode, Layout, SnapshotSource, StrategyConfig};

// ---------------------------------------------------------------------------
// Shared setup helpers
// ---------------------------------------------------------------------------

fn strategy_runtime_with_focus() -> LayoutRuntime {
    let mut rt = Layout::master_stack(["editor", "chat", "status"])
        .master_ratio(0.6)
        .gap(1.0)
        .into_runtime()
        .unwrap();
    rt.focus_next(); // focus "chat"
    rt
}

fn tabbed_runtime() -> LayoutRuntime {
    Layout::tabbed(["a", "b", "c"]).into_runtime().unwrap()
}

fn collapsed_runtime() -> LayoutRuntime {
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
    rt
}

fn cross_axis_layout() -> Layout {
    panes::layout! {
        col(gap: 2.0) {
            panel("top", grow: 1.0, max_height: 100.0)
            panel("bottom", grow: 1.0)
        }
    }
    .unwrap()
}

fn alignment_layout() -> Layout {
    panes::layout! {
        row {
            panel("a", fixed: 100.0, align: center)
        }
    }
    .unwrap()
}

fn size_mode_layout() -> Layout {
    panes::layout! {
        row {
            panel("a", grow: 1.0, size_mode: fit_content(200.0))
            panel("b", grow: 1.0)
        }
    }
    .unwrap()
}

// ---------------------------------------------------------------------------
// Snapshot round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn strategy_snapshot_round_trip() {
    let rt = strategy_runtime_with_focus();
    let snap = rt.snapshot().unwrap();

    assert_eq!(snap.focused(), Some("chat"));
    match snap.source() {
        SnapshotSource::Strategy { strategy, panels } => {
            assert!(matches!(strategy, StrategyConfig::MasterStack { .. }));
            let kinds: Vec<&str> = panels.iter().map(|s| &**s).collect();
            assert_eq!(kinds, &["editor", "chat", "status"]);
        }
        SnapshotSource::Tree { .. } | _ => panic!("expected Strategy source"),
    }

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

    let left = frame.layout().panels().find(|e| e.kind == "left").unwrap();
    assert!((left.rect.w - 100.0).abs() < 1.0);
}

#[test]
fn snapshot_restores_focus() {
    let mut rt = Layout::master_stack(["a", "b", "c"])
        .gap(1.0)
        .into_runtime()
        .unwrap();
    rt.focus_next();
    rt.focus_next();

    let snap = rt.snapshot().unwrap();
    assert_eq!(snap.focused(), Some("c"));

    let rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    assert_eq!(rt2.focused_kind(), Some("c"));
}

#[test]
fn snapshot_restores_collapsed() {
    let rt = collapsed_runtime();

    let snap = rt.snapshot().unwrap();
    assert!(snap.collapsed().iter().any(|s| &**s == "b"));

    let rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    let b_pid2 = rt2
        .sequence()
        .iter()
        .find(|&pid| rt2.tree().panel_kind(pid).ok() == Some("b"))
        .unwrap();
    assert!(rt2.viewport().collapsed.contains(&b_pid2));
}

#[test]
fn tabbed_snapshot_round_trip() {
    let rt = tabbed_runtime();

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
fn empty_tree_snapshot_errors() {
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

    let c = frame.layout().panels().find(|e| e.kind == "c").unwrap();
    assert!((c.rect.w - 50.0).abs() < 1.0);
}

#[test]
fn dashboard_auto_fill_round_trips() {
    let mut rt = Layout::dashboard([("a", 1), ("b", 1), ("c", 1)])
        .auto_fill(200.0)
        .into_runtime()
        .unwrap();
    rt.focus_next();

    let snap = rt.snapshot().unwrap();
    match snap.source() {
        SnapshotSource::Strategy { strategy, .. } => {
            assert!(matches!(
                strategy,
                StrategyConfig::Dashboard {
                    columns: GridColumnMode::AutoFill { .. },
                    ..
                }
            ));
        }
        SnapshotSource::Tree { .. } | _ => panic!("expected Strategy source"),
    }

    let mut rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    let frame = rt2.resolve(800.0, 600.0).unwrap();
    assert_eq!(frame.layout().panels().count(), 3);
}

#[test]
fn snapshot_preserves_cross_axis_constraints() {
    let rt = LayoutRuntime::from(cross_axis_layout());
    let snap = rt.snapshot().unwrap();

    let mut rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    let frame = rt2.resolve(400.0, 400.0).unwrap();

    let top = frame.layout().panels().find(|e| e.kind == "top").unwrap();
    assert!(
        top.rect.h <= 100.0 + 0.5,
        "max_height constraint lost after snapshot: h={}",
        top.rect.h
    );
}

#[test]
fn snapshot_preserves_alignment() {
    let rt = LayoutRuntime::from(alignment_layout());
    let snap = rt.snapshot().unwrap();

    let mut rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    let frame = rt2.resolve(400.0, 400.0).unwrap();

    let a = frame.layout().panels().find(|e| e.kind == "a").unwrap();
    assert!(
        a.rect.h < 400.0,
        "alignment lost after snapshot: panel stretched to h={}",
        a.rect.h
    );
}

#[test]
fn snapshot_preserves_size_mode() {
    let rt = LayoutRuntime::from(size_mode_layout());
    let snap = rt.snapshot().unwrap();

    let mut rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    let frame = rt2.resolve(400.0, 400.0).unwrap();

    assert_eq!(frame.layout().panels().count(), 2);
    let a = frame.layout().panels().find(|e| e.kind == "a").unwrap();
    assert!(a.rect.w > 0.0, "size_mode constraint lost after snapshot");
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
        let rt = strategy_runtime_with_focus();
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
        let rt = tabbed_runtime();
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
        let rt = collapsed_runtime();

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
    fn cross_axis_constraints_json_round_trip() {
        let rt = LayoutRuntime::from(cross_axis_layout());
        let snap = rt.snapshot().unwrap();
        let restored = json_round_trip(&snap);

        let mut rt2 = LayoutRuntime::from_snapshot(restored).unwrap();
        let frame = rt2.resolve(400.0, 400.0).unwrap();

        let top = frame.layout().panels().find(|e| e.kind == "top").unwrap();
        assert!(
            top.rect.h <= 100.0 + 0.5,
            "max_height lost after JSON round-trip: h={}",
            top.rect.h
        );
    }

    #[test]
    fn alignment_json_round_trip() {
        let rt = LayoutRuntime::from(alignment_layout());
        let snap = rt.snapshot().unwrap();
        let restored = json_round_trip(&snap);

        let mut rt2 = LayoutRuntime::from_snapshot(restored).unwrap();
        let frame = rt2.resolve(400.0, 400.0).unwrap();

        let a = frame.layout().panels().find(|e| e.kind == "a").unwrap();
        assert!(
            a.rect.h < 400.0,
            "alignment lost after JSON round-trip: panel stretched to h={}",
            a.rect.h
        );
    }

    #[test]
    fn size_mode_json_round_trip() {
        let rt = LayoutRuntime::from(size_mode_layout());
        let snap = rt.snapshot().unwrap();
        let restored = json_round_trip(&snap);

        let mut rt2 = LayoutRuntime::from_snapshot(restored).unwrap();
        let frame = rt2.resolve(400.0, 400.0).unwrap();
        assert_eq!(frame.layout().panels().count(), 2);
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

        assert!(json.contains("MasterStack"));
        assert!(json.contains("editor"));
        assert!(json.contains("chat"));
        assert!(json.contains("master_ratio"));
    }
}
