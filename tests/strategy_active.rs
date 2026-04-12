#![allow(clippy::unwrap_used, clippy::panic)]
use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{ActivePanelVariant, FocusOutcome, FocusRejection, StrategyKind};

fn kinds(n: usize) -> Vec<Arc<str>> {
    (0..n).map(|i| Arc::from(format!("p{i}"))).collect()
}

fn monocle_runtime(n: usize) -> LayoutRuntime {
    let k = kinds(n);
    LayoutRuntime::from_strategy(
        StrategyKind::ActivePanel {
            variant: ActivePanelVariant::Monocle,
            bar_height: 0.0,
        },
        &k,
    )
    .unwrap()
}

#[test]
fn monocle_focus_change_is_constraint_only() {
    let mut rt = monocle_runtime(3);
    let p0 = rt.sequence().get(0).unwrap();
    let p1 = rt.sequence().get(1).unwrap();

    // Initially p0 is visible
    let c0 = rt.tree().panel_constraints(p0).unwrap();
    assert!(c0.grow.is_some());

    // Focus p1
    rt.focus(p1);
    assert_eq!(rt.focused(), Some(p1));

    // p0 should now be hidden, p1 visible
    let c0 = rt.tree().panel_constraints(p0).unwrap();
    assert_eq!(c0.fixed, Some(0.0));
    let c1 = rt.tree().panel_constraints(p1).unwrap();
    assert!(c1.grow.is_some());
}

#[test]
fn monocle_add_hides_previous() {
    let mut rt = monocle_runtime(2);
    let p0 = rt.sequence().get(0).unwrap();
    let new_pid = rt.add_panel(Arc::from("p_new")).unwrap();
    assert_eq!(rt.focused(), Some(new_pid));
    let c0 = rt.tree().panel_constraints(p0).unwrap();
    assert_eq!(c0.fixed, Some(0.0));
}

#[test]
fn monocle_remove_focused_shows_neighbor() {
    let mut rt = monocle_runtime(3);
    let p1 = rt.sequence().get(1).unwrap();
    rt.focus(p1);
    let new_focus = rt.remove_panel(p1).unwrap();
    assert!(new_focus.is_some());
    let focus = new_focus.unwrap();
    let c = rt.tree().panel_constraints(focus).unwrap();
    assert!(c.grow.is_some());
}

#[test]
fn monocle_focus_same_panel_is_noop() {
    let mut rt = monocle_runtime(2);
    let p0 = rt.sequence().get(0).unwrap();
    rt.focus(p0);
    assert_eq!(rt.focused(), Some(p0));
}

// -- Tabbed --

#[test]
fn tabbed_sequence_excludes_tab_panels() {
    let rt = LayoutRuntime::from_strategy(
        StrategyKind::ActivePanel {
            variant: ActivePanelVariant::Tabbed,
            bar_height: 1.0,
        },
        &[
            Arc::from("editor"),
            Arc::from("terminal"),
            Arc::from("logs"),
        ],
    )
    .unwrap();

    // Sequence should only contain content panels, not decoration panels
    assert_eq!(rt.sequence().len(), 3);
    for pid in rt.sequence().iter() {
        assert!(
            !rt.tree().is_decoration(pid),
            "decoration panel should not be in sequence"
        );
    }
}

#[test]
fn tabbed_focus_cycles_content_only() {
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::ActivePanel {
            variant: ActivePanelVariant::Tabbed,
            bar_height: 1.0,
        },
        &[
            Arc::from("editor"),
            Arc::from("terminal"),
            Arc::from("logs"),
        ],
    )
    .unwrap();

    let p0 = rt.sequence().get(0).unwrap();
    assert_eq!(rt.tree().panel_kind(p0).unwrap(), "editor");

    // Tab through all panels — each should be a content panel
    rt.focus_next();
    let f1 = rt.focused().unwrap();
    assert_eq!(rt.tree().panel_kind(f1).unwrap(), "terminal");

    rt.focus_next();
    let f2 = rt.focused().unwrap();
    assert_eq!(rt.tree().panel_kind(f2).unwrap(), "logs");

    // Previous focus should be hidden
    let c1 = rt.tree().panel_constraints(f1).unwrap();
    assert_eq!(c1.fixed, Some(0.0));
    // Current focus should be visible
    let c2 = rt.tree().panel_constraints(f2).unwrap();
    assert!(c2.grow.is_some());
}

// -- Stacked --

#[test]
fn stacked_sequence_excludes_title_panels() {
    let rt = LayoutRuntime::from_strategy(
        StrategyKind::ActivePanel {
            variant: ActivePanelVariant::Stacked,
            bar_height: 1.0,
        },
        &[
            Arc::from("editor"),
            Arc::from("terminal"),
            Arc::from("logs"),
        ],
    )
    .unwrap();

    assert_eq!(rt.sequence().len(), 3);
    for pid in rt.sequence().iter() {
        assert!(
            !rt.tree().is_decoration(pid),
            "decoration panel should not be in sequence"
        );
    }
}

#[test]
fn stacked_focus_cycles_content_only() {
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::ActivePanel {
            variant: ActivePanelVariant::Stacked,
            bar_height: 1.0,
        },
        &[
            Arc::from("editor"),
            Arc::from("terminal"),
            Arc::from("logs"),
        ],
    )
    .unwrap();

    let p0 = rt.sequence().get(0).unwrap();
    assert_eq!(rt.tree().panel_kind(p0).unwrap(), "editor");

    rt.focus_next();
    let f1 = rt.focused().unwrap();
    assert_eq!(rt.tree().panel_kind(f1).unwrap(), "terminal");

    // editor should be hidden, terminal visible
    let c0 = rt.tree().panel_constraints(p0).unwrap();
    assert_eq!(c0.fixed, Some(0.0));
    let c1 = rt.tree().panel_constraints(f1).unwrap();
    assert!(c1.grow.is_some());
}

// -- Decoration node identity (Step 1) --

#[test]
fn tabbed_and_stacked_sequences_still_exclude_decoration_nodes() {
    // Tabbed
    let tabbed_rt = LayoutRuntime::from_strategy(
        StrategyKind::ActivePanel {
            variant: ActivePanelVariant::Tabbed,
            bar_height: 1.0,
        },
        &[Arc::from("a"), Arc::from("b"), Arc::from("c")],
    )
    .unwrap();
    assert_eq!(tabbed_rt.sequence().len(), 3);
    for pid in tabbed_rt.sequence().iter() {
        assert!(
            !tabbed_rt.tree().is_decoration(pid),
            "tabbed sequence must contain only content panels"
        );
    }

    // Stacked
    let stacked_rt = LayoutRuntime::from_strategy(
        StrategyKind::ActivePanel {
            variant: ActivePanelVariant::Stacked,
            bar_height: 1.0,
        },
        &[Arc::from("a"), Arc::from("b"), Arc::from("c")],
    )
    .unwrap();
    assert_eq!(stacked_rt.sequence().len(), 3);
    for pid in stacked_rt.sequence().iter() {
        assert!(
            !stacked_rt.tree().is_decoration(pid),
            "stacked sequence must contain only content panels"
        );
    }
}

// -- Dirty-state split: focus changes are constraint-only --

#[test]
fn monocle_focus_change_is_constraint_only_under_split_dirty_state() {
    let mut rt = monocle_runtime(3);

    // First resolve: establish baseline
    let _ = rt.resolve(200.0, 200.0).unwrap();
    let p0 = rt.sequence().get(0).unwrap();
    let p1 = rt.sequence().get(1).unwrap();
    assert_eq!(rt.last_diff().added.len(), 3);

    // Second resolve: settle into unchanged state
    let _ = rt.resolve(200.0, 200.0).unwrap();
    assert_eq!(rt.last_diff().unchanged.len(), 3);

    // Focus p1 — constraint-only change (hide p0, show p1)
    rt.focus(p1);
    assert_eq!(rt.focused(), Some(p1));

    // Third resolve: visibility changed but no topology mutation
    let frame3 = rt.resolve(200.0, 200.0).unwrap();

    // Visibility updated: p1 visible, p0 hidden (zero-height in col layout)
    let r1 = frame3.layout().get(p1).unwrap();
    assert!(r1.h > 0.0, "focused panel should be visible");
    let r0 = frame3.layout().get(p0).unwrap();
    assert!(
        r0.h < 1.0,
        "unfocused panel should be hidden, got h={}",
        r0.h
    );

    // Kind lookup still works (kind caches preserved)
    assert_eq!(frame3.layout().by_kind("p0"), &[p0]);
    assert_eq!(frame3.layout().by_kind("p1"), &[p1]);

    // Diff should show resized/moved, NOT added/removed (same-panel path)
    let diff = rt.last_diff();
    assert!(
        diff.added.is_empty(),
        "focus change must not report added panels"
    );
    assert!(
        diff.removed.is_empty(),
        "focus change must not report removed panels"
    );
}

#[test]
fn window_focus_slide_updates_visibility_without_topology_rebuild() {
    let mut rt = window_runtime(4);

    // First resolve: establish baseline
    let _ = rt.resolve(400.0, 200.0).unwrap();
    let p0 = rt.sequence().get(0).unwrap();
    let p3 = rt.sequence().get(3).unwrap();
    assert_eq!(rt.last_diff().added.len(), 4);

    // Second resolve: settle into unchanged state
    let _ = rt.resolve(400.0, 200.0).unwrap();
    assert_eq!(rt.last_diff().unchanged.len(), 4);

    // Focus p3 — slides window from [0,1] to [2,3]
    rt.focus(p3);
    assert_eq!(rt.focused(), Some(p3));
    assert!(rt.viewport().window_start >= 2);

    // Third resolve: window shifted, no topology change
    let frame3 = rt.resolve(400.0, 200.0).unwrap();

    // p3 should be visible, p0 should be hidden
    let r3 = frame3.layout().get(p3).unwrap();
    assert!(r3.w > 0.0, "focused panel should be visible");
    let r0 = frame3.layout().get(p0).unwrap();
    assert!(
        r0.w < 1.0,
        "panel outside window should be hidden, got w={}",
        r0.w
    );

    // Kind lookup stable (kind caches preserved)
    assert_eq!(frame3.layout().by_kind("p0"), &[p0]);
    assert_eq!(frame3.layout().by_kind("p3"), &[p3]);

    // Diff: same-panel path (no add/remove churn)
    let diff = rt.last_diff();
    assert!(
        diff.added.is_empty(),
        "window slide must not report added panels"
    );
    assert!(
        diff.removed.is_empty(),
        "window slide must not report removed panels"
    );
}

fn window_runtime(n: usize) -> LayoutRuntime {
    let k = kinds(n);
    LayoutRuntime::from_strategy(
        StrategyKind::Window {
            panel_count: 2,
            gap: 0.0,
        },
        &k,
    )
    .unwrap()
}

#[test]
fn window_focus_within_pair_no_constraint_change() {
    let mut rt = window_runtime(4);
    let p0 = rt.sequence().get(0).unwrap();
    let p1 = rt.sequence().get(1).unwrap();

    // Both p0 and p1 should be visible initially
    let c0 = rt.tree().panel_constraints(p0).unwrap();
    let c1 = rt.tree().panel_constraints(p1).unwrap();
    assert!(c0.grow.is_some());
    assert!(c1.grow.is_some());

    // Focus p1 — still in window, no constraint changes
    rt.focus(p1);
    assert_eq!(rt.focused(), Some(p1));
    assert_eq!(rt.viewport().window_start, 0);
}

#[test]
fn window_focus_past_edge_slides() {
    let mut rt = window_runtime(4);
    let p3 = rt.sequence().get(3).unwrap();

    rt.focus(p3);
    assert_eq!(rt.focused(), Some(p3));

    // Window should have shifted
    let ws = rt.viewport().window_start;
    assert!(ws >= 2);

    // p3 should be visible
    let c3 = rt.tree().panel_constraints(p3).unwrap();
    assert!(c3.grow.is_some());
}

#[test]
fn window_focus_rejects_when_sequence_and_tree_drift() {
    let mut rt = window_runtime(4);
    let initial_focus = rt.focused();
    let p3 = rt.sequence().get(3).unwrap();

    rt.tree_mut().remove_panel(p3).unwrap();

    let outcome = rt.focus(p3);

    assert_eq!(
        outcome,
        FocusOutcome::Rejected(FocusRejection::StrategyRejected)
    );
    assert_eq!(rt.focused(), initial_focus);
    assert_eq!(rt.viewport().window_start, 0);
}

#[test]
fn window_add_shifts_window() {
    let mut rt = window_runtime(3);
    let new_pid = rt.add_panel(Arc::from("p_new")).unwrap();
    assert_eq!(rt.focused(), Some(new_pid));

    // New panel should be visible
    let c = rt.tree().panel_constraints(new_pid).unwrap();
    assert!(c.grow.is_some());
}

#[test]
fn window_remove_adjusts_window() {
    let mut rt = window_runtime(4);
    let p0 = rt.sequence().get(0).unwrap();
    let new_focus = rt.remove_panel(p0).unwrap();
    assert!(new_focus.is_some());
    assert_eq!(rt.sequence().len(), 3);
}
