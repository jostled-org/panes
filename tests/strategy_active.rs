use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{ActivePanelVariant, StrategyKind};

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
    rt.focus(p1).unwrap();
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
    rt.focus(p1).unwrap();
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
    rt.focus(p0).unwrap();
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

    // Sequence should only contain content panels, not _tab panels
    assert_eq!(rt.sequence().len(), 3);
    for pid in rt.sequence().iter() {
        let kind = rt.tree().panel_kind(pid).unwrap();
        assert!(
            !kind.ends_with("_tab"),
            "tab panel {kind} should not be in sequence"
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
    rt.focus_next().unwrap();
    let f1 = rt.focused().unwrap();
    assert_eq!(rt.tree().panel_kind(f1).unwrap(), "terminal");

    rt.focus_next().unwrap();
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
        let kind = rt.tree().panel_kind(pid).unwrap();
        assert!(
            !kind.ends_with("_title"),
            "title panel {kind} should not be in sequence"
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

    rt.focus_next().unwrap();
    let f1 = rt.focused().unwrap();
    assert_eq!(rt.tree().panel_kind(f1).unwrap(), "terminal");

    // editor should be hidden, terminal visible
    let c0 = rt.tree().panel_constraints(p0).unwrap();
    assert_eq!(c0.fixed, Some(0.0));
    let c1 = rt.tree().panel_constraints(f1).unwrap();
    assert!(c1.grow.is_some());
}

// -- Window (Scrollable) --

fn window_runtime(n: usize) -> LayoutRuntime {
    let k = kinds(n);
    LayoutRuntime::from_strategy(StrategyKind::Window { size: 2, gap: 0.0 }, &k).unwrap()
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
    rt.focus(p1).unwrap();
    assert_eq!(rt.focused(), Some(p1));
    assert_eq!(rt.viewport().window_start, 0);
}

#[test]
fn window_focus_past_edge_slides() {
    let mut rt = window_runtime(4);
    let p3 = rt.sequence().get(3).unwrap();

    rt.focus(p3).unwrap();
    assert_eq!(rt.focused(), Some(p3));

    // Window should have shifted
    let ws = rt.viewport().window_start;
    assert!(ws >= 2);

    // p3 should be visible
    let c3 = rt.tree().panel_constraints(p3).unwrap();
    assert!(c3.grow.is_some());
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
