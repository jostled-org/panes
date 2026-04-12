#![allow(clippy::unwrap_used, clippy::panic)]
#[cfg(feature = "serde")]
use panes::LayoutSnapshot;
use panes::runtime::LayoutRuntime;
use panes::{
    Constraints, GridColumnMode, Layout, LayoutBuilder, NodeId, PaneError, PanelId, PanelSequence,
    SnapshotSource, StrategyConfig, TreeError, fixed, grow,
};

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
        _ => panic!("expected Strategy source"),
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
fn snapshot_restore_preserves_root_container_constraints() {
    let mut tree = panes::LayoutTree::new();
    let (_, left) = tree.add_panel("left", fixed(120.0)).unwrap();
    let (_, right) = tree.add_panel("right", grow(1.0)).unwrap();
    let root_constraints = Constraints {
        grow: Some(2.0),
        ..Default::default()
    };
    let root = tree
        .add_row_constrained(6.0, Some(root_constraints), vec![left, right])
        .unwrap();
    tree.set_root(root);

    let rt = LayoutRuntime::new(tree);
    let snap = rt.snapshot().unwrap();

    let restored = LayoutRuntime::from_snapshot(snap).unwrap();
    let restored_snap = restored.snapshot().unwrap();

    match restored_snap.source() {
        SnapshotSource::Tree { root } => match root {
            panes::SnapshotNode::Row {
                gap,
                constraints,
                children,
            } => {
                assert_eq!(*gap, 6.0);
                assert_eq!(*constraints, Some(root_constraints));
                assert_eq!(children.len(), 2);
            }
            _ => panic!("expected row root"),
        },
        _ => panic!("expected Tree source"),
    }
}

#[test]
fn snapshot_restore_preserves_root_panel_topology() {
    let mut tree = panes::LayoutTree::new();
    let (_, root) = tree.add_panel("solo", grow(1.0)).unwrap();
    tree.set_root(root);

    let rt = LayoutRuntime::new(tree);
    let snap = rt.snapshot().unwrap();

    let restored = LayoutRuntime::from_snapshot(snap).unwrap();
    let restored_snap = restored.snapshot().unwrap();

    match restored_snap.source() {
        SnapshotSource::Tree { root } => {
            assert!(matches!(
                root,
                panes::SnapshotNode::Panel { kind, .. } if &**kind == "solo"
            ));
        }
        _ => panic!("expected Tree source"),
    }
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
    // 3 content panels + 3 tab decoration panels
    assert_eq!(frame.layout().panels().count(), 3);
    assert_eq!(frame.layout().decoration_panels().len(), 3);
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
fn tree_snapshot_errors_on_missing_row_child() {
    let mut tree = panes::LayoutTree::new();
    let (_, child) = tree.add_panel("a", grow(1.0)).unwrap();
    let row = tree.add_row(0.0, vec![child]).unwrap();
    tree.set_root(row);
    tree.insert_child_at(row, 1, NodeId::from_raw(999)).unwrap();

    let rt = LayoutRuntime::new(tree);
    assert!(rt.snapshot().is_err());
}

#[test]
fn tree_snapshot_errors_on_deep_tree() {
    let mut tree = panes::LayoutTree::new();
    let (_, leaf) = tree.add_panel("a", grow(1.0)).unwrap();
    let mut root = leaf;

    for _ in 0..=64 {
        root = tree.add_row(0.0, vec![root]).unwrap();
    }

    tree.set_root(root);

    let rt = LayoutRuntime::new(tree);
    let err = rt.snapshot().unwrap_err();
    assert!(matches!(
        err,
        panes::PaneError::InvalidTree(panes::TreeError::SnapshotTooDeep(64))
    ));
}

#[test]
fn tree_snapshot_errors_on_unsupported_root_passthrough() {
    let mut tree = panes::LayoutTree::new();
    let (_, panel) = tree.add_panel("a", grow(1.0)).unwrap();
    let root = tree
        .add_taffy_node(taffy::Style::default(), vec![panel])
        .unwrap();
    tree.set_root(root);

    let rt = LayoutRuntime::new(tree);
    let err = rt.snapshot().unwrap_err();
    assert!(matches!(
        err,
        panes::PaneError::InvalidTree(panes::TreeError::UnsupportedSnapshotNode(id)) if id == root
    ));
}

#[test]
fn tree_snapshot_errors_on_unsupported_nested_passthrough() {
    let mut tree = panes::LayoutTree::new();
    let (_, panel) = tree.add_panel("a", grow(1.0)).unwrap();
    let passthrough = tree
        .add_taffy_node(taffy::Style::default(), vec![panel])
        .unwrap();
    let root = tree.add_row(0.0, vec![passthrough]).unwrap();
    tree.set_root(root);

    let rt = LayoutRuntime::new(tree);
    let err = rt.snapshot().unwrap_err();
    assert!(matches!(
        err,
        panes::PaneError::InvalidTree(panes::TreeError::UnsupportedSnapshotNode(id))
            if id == passthrough
    ));
}

#[test]
fn strategy_snapshot_errors_on_missing_focused_panel() {
    let mut rt = Layout::master_stack(["a", "b"]).into_runtime().unwrap();
    rt.set_focus_unchecked(PanelId::from_raw(999));

    assert!(rt.snapshot().is_err());
}

#[test]
fn strategy_snapshot_errors_on_missing_sequence_panel() {
    let mut rt = Layout::master_stack(["a", "b"]).into_runtime().unwrap();
    let removed = rt.sequence().iter().next().unwrap();
    rt.tree_mut().remove_panel(removed).unwrap();

    assert!(rt.snapshot().is_err());
}

#[test]
fn strategy_snapshot_errors_when_focused_panel_is_missing_from_sequence() {
    let mut tree = panes::LayoutTree::new();
    let (p0, n0) = tree.add_panel("a", grow(1.0)).unwrap();
    let (_, n1) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1]).unwrap();
    tree.set_root(root);

    let mut sequence = PanelSequence::default();
    sequence.push(p0);

    let mut rt = LayoutRuntime::from_tree_and_sequence(tree, sequence);
    rt.set_focus_unchecked(PanelId::from_raw(1));

    let err = rt.snapshot().unwrap_err();
    assert!(matches!(
        err,
        PaneError::InvalidTree(TreeError::SnapshotFocusedMissingFromSequence(pid))
            if pid == PanelId::from_raw(1)
    ));
}

#[test]
fn strategy_snapshot_errors_when_collapsed_panel_is_missing_from_sequence() {
    let mut tree = panes::LayoutTree::new();
    let (p0, n0) = tree.add_panel("a", grow(1.0)).unwrap();
    let (p1, n1) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0, n1]).unwrap();
    tree.set_root(root);

    let mut sequence = PanelSequence::default();
    sequence.push(p0);

    let mut rt = LayoutRuntime::from_tree_and_sequence(tree, sequence);
    rt.toggle_collapsed(p1).unwrap();

    let err = rt.snapshot().unwrap_err();
    assert!(matches!(
        err,
        PaneError::InvalidTree(TreeError::SnapshotCollapsedMissingFromSequence(pid)) if pid == p1
    ));
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
        _ => panic!("expected Strategy source"),
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
// Generic grid snapshot tests
// ---------------------------------------------------------------------------

#[test]
fn generic_grid_layout_snapshot_round_trip_preserves_geometry() {
    // Build a layout with nested grid, spans, and row content
    let build = || {
        panes::layout! {
            row(gap: 4.0) {
                panel("sidebar", fixed: 100.0)
                grid(columns: 2, gap: 8.0) {
                    panel("a")
                    panel("b")
                    panel("c", full_width: true)
                    panel("d")
                }
            }
        }
        .unwrap()
    };

    let rt = LayoutRuntime::from(build());
    let snap = rt.snapshot().unwrap();

    match snap.source() {
        SnapshotSource::Tree { root } => {
            assert!(matches!(root, panes::SnapshotNode::Row { .. }));
        }
        _ => panic!("expected Tree source"),
    }

    let mut rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    let frame_original = {
        let mut rt_orig = LayoutRuntime::from(build());
        rt_orig.resolve(800.0, 600.0).unwrap()
    };
    let frame_restored = rt2.resolve(800.0, 600.0).unwrap();

    // Both should have the same panel count
    assert_eq!(
        frame_original.layout().panels().count(),
        frame_restored.layout().panels().count(),
    );

    // Sidebar geometry preserved
    let orig_sidebar = frame_original
        .layout()
        .panels()
        .find(|e| e.kind == "sidebar")
        .unwrap();
    let rest_sidebar = frame_restored
        .layout()
        .panels()
        .find(|e| e.kind == "sidebar")
        .unwrap();
    assert!(
        (orig_sidebar.rect.w - rest_sidebar.rect.w).abs() < 1.0,
        "sidebar width diverged: {} vs {}",
        orig_sidebar.rect.w,
        rest_sidebar.rect.w
    );

    // Full-width span panel should span entire grid width
    let rest_c = frame_restored
        .layout()
        .panels()
        .find(|e| e.kind == "c")
        .unwrap();
    let rest_a = frame_restored
        .layout()
        .panels()
        .find(|e| e.kind == "a")
        .unwrap();
    assert!(
        rest_c.rect.w > rest_a.rect.w * 1.5,
        "full-width panel c should be wider than single-column a: c.w={}, a.w={}",
        rest_c.rect.w,
        rest_a.rect.w
    );
}

#[test]
fn generic_grid_runtime_snapshot_round_trip_preserves_sequence_and_focus() {
    let layout = Layout::build_grid(panes::Grid::columns(2).gap(4.0), |g| {
        g.panel("alpha");
        g.panel("beta");
        g.panel("gamma");
    })
    .unwrap();

    let mut rt = LayoutRuntime::from(layout);
    rt.resolve(800.0, 600.0).unwrap();

    // Focus "beta" by panel id
    let beta_pid = rt.tree().panels_by_kind("beta")[0];
    assert!(rt.focus(beta_pid).is_on_target());

    let snap = rt.snapshot().unwrap();
    assert_eq!(snap.focused(), Some("beta"));

    let mut rt2 = LayoutRuntime::from_snapshot(snap).unwrap();
    assert_eq!(rt2.focused_kind(), Some("beta"));

    // Restored tree has same panel count
    let frame = rt2.resolve(800.0, 600.0).unwrap();
    assert_eq!(frame.layout().panels().count(), 3);
}

// ---------------------------------------------------------------------------
// Constrained container snapshot tests
// ---------------------------------------------------------------------------

#[test]
fn constrained_container_tree_snapshot_round_trip_preserves_nested_weights() {
    // Build: row [ sidebar(fixed:200) | col_with(grow:2) [ editor ] | col_with(grow:1) [ chat ] ]
    let build_layout = || {
        let mut b = LayoutBuilder::new();
        b.row(|r| {
            r.panel_with("sidebar", fixed(200.0));
            r.col_with(
                Constraints {
                    grow: Some(2.0),
                    ..Default::default()
                },
                |c| {
                    c.panel("editor");
                },
            );
            r.col_with(
                Constraints {
                    grow: Some(1.0),
                    ..Default::default()
                },
                |c| {
                    c.panel("chat");
                },
            );
        })
        .unwrap();
        b.build().unwrap()
    };

    let rt = LayoutRuntime::from(build_layout());
    let snap = rt.snapshot().unwrap();

    // Verify it uses Tree source
    assert!(matches!(snap.source(), SnapshotSource::Tree { .. }));

    // Restore and resolve both
    let mut rt_orig = LayoutRuntime::from(build_layout());
    let frame_orig = rt_orig.resolve(800.0, 600.0).unwrap();

    let mut rt_restored = LayoutRuntime::from_snapshot(snap).unwrap();
    let frame_restored = rt_restored.resolve(800.0, 600.0).unwrap();

    // Same panel count
    assert_eq!(
        frame_orig.layout().panels().count(),
        frame_restored.layout().panels().count(),
    );

    // Compare geometry for each kind
    for kind in &["sidebar", "editor", "chat"] {
        let orig = frame_orig
            .layout()
            .panels()
            .find(|e| e.kind == *kind)
            .unwrap();
        let rest = frame_restored
            .layout()
            .panels()
            .find(|e| e.kind == *kind)
            .unwrap();
        assert!(
            (orig.rect.w - rest.rect.w).abs() < 1.0,
            "{kind} width diverged: orig={}, restored={}",
            orig.rect.w,
            rest.rect.w
        );
        assert!(
            (orig.rect.h - rest.rect.h).abs() < 1.0,
            "{kind} height diverged: orig={}, restored={}",
            orig.rect.h,
            rest.rect.h
        );
    }
}

#[test]
fn constrained_container_snapshot_preserves_gap_and_constraints() {
    // Build: col(gap:10) [ row_gap_with(gap:5, grow:1) [ a, b ] | row_gap_with(gap:5, fixed:100) [ c ] ]
    let build_layout = || {
        let mut b = LayoutBuilder::new();
        b.col_gap(10.0, |c| {
            c.row_gap_with(
                5.0,
                Constraints {
                    grow: Some(1.0),
                    ..Default::default()
                },
                |r| {
                    r.panel("a");
                    r.panel("b");
                },
            );
            c.row_gap_with(
                5.0,
                Constraints {
                    fixed: Some(100.0),
                    ..Default::default()
                },
                |r| {
                    r.panel("c");
                },
            );
        })
        .unwrap();
        b.build().unwrap()
    };

    let rt = LayoutRuntime::from(build_layout());
    let snap = rt.snapshot().unwrap();

    let mut rt_orig = LayoutRuntime::from(build_layout());
    let frame_orig = rt_orig.resolve(800.0, 600.0).unwrap();

    let mut rt_restored = LayoutRuntime::from_snapshot(snap).unwrap();
    let frame_restored = rt_restored.resolve(800.0, 600.0).unwrap();

    // The fixed row should be 100px tall in both
    let orig_c = frame_orig
        .layout()
        .panels()
        .find(|e| e.kind == "c")
        .unwrap();
    let rest_c = frame_restored
        .layout()
        .panels()
        .find(|e| e.kind == "c")
        .unwrap();
    assert!(
        (orig_c.rect.h - 100.0).abs() < 1.0,
        "original c height should be 100, got {}",
        orig_c.rect.h
    );
    assert!(
        (rest_c.rect.h - 100.0).abs() < 1.0,
        "restored c height should be 100, got {}",
        rest_c.rect.h
    );

    // Panels a and b should have gap between them (gap-sensitive placement)
    let rest_a = frame_restored
        .layout()
        .panels()
        .find(|e| e.kind == "a")
        .unwrap();
    let rest_b = frame_restored
        .layout()
        .panels()
        .find(|e| e.kind == "b")
        .unwrap();
    let gap_between = rest_b.rect.x - (rest_a.rect.x + rest_a.rect.w);
    assert!(
        (gap_between - 5.0).abs() < 1.0,
        "gap between a and b should be ~5px, got {}",
        gap_between
    );
}

// ---------------------------------------------------------------------------
// Repeated kinds — snapshot restore preserves focused panel instance
// ---------------------------------------------------------------------------

#[test]
fn snapshot_restore_with_repeated_kinds_preserves_focused_panel_instance() {
    let mut rt = Layout::master_stack(["editor", "chat", "editor"])
        .master_ratio(0.6)
        .gap(1.0)
        .into_runtime()
        .unwrap();

    // Focus the second "editor" (sequence index 2)
    let second_editor = rt.sequence().get(2).unwrap();
    assert_eq!(rt.tree().panel_kind(second_editor).unwrap(), "editor");
    rt.focus(second_editor);

    // Verify focus is at sequence index 2, not 0
    let focused_before = rt.viewport().focus.unwrap();
    let idx_before = rt.sequence().index_of(focused_before).unwrap();
    assert_eq!(idx_before, 2);

    // Snapshot and restore
    let snap = rt.snapshot().unwrap();
    let rt2 = LayoutRuntime::from_snapshot(snap).unwrap();

    // Restored focus should be on the same sequence position (the second editor)
    let focused_after = rt2.viewport().focus.unwrap();
    let idx_after = rt2.sequence().index_of(focused_after).unwrap();
    assert_eq!(
        idx_after, 2,
        "restored focus should be on the second editor (index 2), not the first"
    );
    assert_eq!(rt2.focused_kind(), Some("editor"));
}

// ---------------------------------------------------------------------------
// Serde round-trip tests (JSON serialize → deserialize → restore)
// ---------------------------------------------------------------------------

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;
    use serde_json::Value;

    fn json_round_trip(snap: &LayoutSnapshot) -> LayoutSnapshot {
        let json = serde_json::to_string(snap).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    fn spanned_non_panel_snapshot(node: Value) -> LayoutSnapshot {
        let layout = panes::layout! {
            grid(columns: 2, gap: 8.0) {
                panel("wide", full_width: true)
            }
        }
        .unwrap();

        let snap = LayoutRuntime::from(layout).snapshot().unwrap();
        let mut value = serde_json::to_value(&snap).unwrap();
        let children = value["source"]["Tree"]["root"]["Grid"]["children"]
            .as_array_mut()
            .unwrap();
        children[0]["node"] = node;
        serde_json::from_value(value).unwrap()
    }

    fn assert_spanned_non_panel_snapshot_errors(node: Value, expected_kind: &'static str) {
        let err = match LayoutRuntime::from_snapshot(spanned_non_panel_snapshot(node)) {
            Ok(_) => panic!("expected invalid spanned non-panel snapshot to fail"),
            Err(err) => err,
        };
        assert!(matches!(
            err,
            PaneError::InvalidTree(TreeError::SnapshotSpanRequiresPanel(kind)) if kind == expected_kind
        ));
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
        assert_eq!(frame.layout().panels().count(), 3);
        assert_eq!(frame.layout().decoration_panels().len(), 3);
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
    fn grid_json_round_trip() {
        let layout = panes::layout! {
            row(gap: 4.0) {
                panel("sidebar", fixed: 100.0)
                grid(columns: 2, gap: 8.0) {
                    panel("a")
                    panel("b")
                    panel("c", full_width: true)
                }
            }
        }
        .unwrap();

        let rt = LayoutRuntime::from(layout);
        let snap = rt.snapshot().unwrap();
        let restored = json_round_trip(&snap);

        let mut rt2 = LayoutRuntime::from_snapshot(restored).unwrap();
        let frame = rt2.resolve(800.0, 600.0).unwrap();
        assert_eq!(frame.layout().panels().count(), 4);

        let sidebar = frame
            .layout()
            .panels()
            .find(|e| e.kind == "sidebar")
            .unwrap();
        assert!((sidebar.rect.w - 100.0).abs() < 1.0);

        // Full-width item should be wider than single-column items
        let c = frame.layout().panels().find(|e| e.kind == "c").unwrap();
        let a = frame.layout().panels().find(|e| e.kind == "a").unwrap();
        assert!(
            c.rect.w > a.rect.w * 1.5,
            "full-width c should be wider than a: c.w={}, a.w={}",
            c.rect.w,
            a.rect.w
        );
    }

    #[test]
    fn snapshot_restore_errors_on_spanned_row_grid_item() {
        assert_spanned_non_panel_snapshot_errors(
            serde_json::json!({
                "Row": {
                    "gap": 0.0,
                    "constraints": null,
                    "children": []
                }
            }),
            "row",
        );
    }

    #[test]
    fn snapshot_restore_errors_on_spanned_col_grid_item() {
        assert_spanned_non_panel_snapshot_errors(
            serde_json::json!({
                "Col": {
                    "gap": 0.0,
                    "constraints": null,
                    "children": []
                }
            }),
            "col",
        );
    }

    #[test]
    fn snapshot_restore_errors_on_spanned_nested_grid_item() {
        assert_spanned_non_panel_snapshot_errors(
            serde_json::json!({
                "Grid": {
                    "columns": { "Fixed": 1 },
                    "gap": 0.0,
                    "auto_rows": false,
                    "children": []
                }
            }),
            "grid",
        );
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
