use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{CardSpan, GridColumnMode, Layout, MutationError, PaneError, StrategyKind};

fn kinds(n: usize) -> Vec<Arc<str>> {
    (0..n).map(|i| Arc::from(format!("p{i}"))).collect()
}

#[cfg(test)]
fn dashboard_runtime(panel_count: usize, columns: usize) -> LayoutRuntime {
    let panel_kinds = kinds(panel_count);
    let spans: Arc<[CardSpan]> = vec![CardSpan::columns(1); panel_count].into();
    LayoutRuntime::from_strategy(
        StrategyKind::Dashboard {
            columns: GridColumnMode::Fixed(columns),
            gap: 0.0,
            spans,
            auto_rows: false,
        },
        &panel_kinds,
    )
    .unwrap()
}

#[test]
fn set_span_columns() {
    let mut rt = dashboard_runtime(2, 4);
    let p0 = rt.sequence().get(0).unwrap();

    rt.set_card_span(p0, CardSpan::columns(2)).unwrap();

    let frame = rt.resolve(400.0, 200.0).unwrap();
    let r0 = frame.layout().get(p0).unwrap();
    let p1 = rt.sequence().get(1).unwrap();
    let r1 = frame.layout().get(p1).unwrap();
    assert!(
        r0.w > r1.w,
        "span-2 card should be wider than span-1: {} vs {}",
        r0.w,
        r1.w
    );
}

#[test]
fn set_span_full_width() {
    let mut rt = dashboard_runtime(2, 4);
    let p0 = rt.sequence().get(0).unwrap();

    rt.set_card_span(p0, CardSpan::full_width()).unwrap();

    let frame = rt.resolve(400.0, 200.0).unwrap();
    let r0 = frame.layout().get(p0).unwrap();
    assert!(
        (r0.w - 400.0).abs() < 1.0,
        "full-width card should span viewport: got {}",
        r0.w
    );
}

#[test]
fn round_trip_span_change() {
    let mut rt = dashboard_runtime(4, 4);
    let p1 = rt.sequence().get(1).unwrap();

    rt.set_card_span(p1, CardSpan::columns(2)).unwrap();

    let frame = rt.resolve(400.0, 200.0).unwrap();
    let r1 = frame.layout().get(p1).unwrap();
    // 4 columns, 0 gap → each column is 100px. Span 2 → ~200px.
    assert!(
        (r1.w - 200.0).abs() < 2.0,
        "expected ~200px for span-2, got {}",
        r1.w
    );
}

#[test]
fn error_on_non_dashboard() {
    let mut rt = Layout::master_stack(["a", "b", "c"])
        .into_runtime()
        .unwrap();
    let p0 = rt.sequence().get(0).unwrap();

    let err = rt.set_card_span(p0, CardSpan::columns(2)).unwrap_err();
    assert!(
        matches!(
            err,
            PaneError::InvalidMutation(MutationError::SpanNotSupported)
        ),
        "expected SpanNotSupported, got {err}"
    );
}

#[cfg(test)]
fn direct_grid_runtime() -> LayoutRuntime {
    let layout = panes::layout! {
        grid(columns: 2) {
            panel("chart", span: 1)
            panel("stats")
            panel("events")
        }
    }
    .unwrap();
    LayoutRuntime::new(layout.into())
}

fn panel_of(rt: &LayoutRuntime, kind: &str) -> panes::PanelId {
    rt.tree().panels_by_kind(kind)[0]
}

#[cfg(test)]
fn rect_of(rt: &mut LayoutRuntime, kind: &str, w: f32, h: f32) -> panes::Rect {
    let pid = panel_of(rt, kind);
    *rt.resolve(w, h).unwrap().layout().get(pid).unwrap()
}

#[test]
fn direct_tree_updates_wrapped_card() {
    let mut rt = direct_grid_runtime();
    let chart = panel_of(&rt, "chart");

    rt.set_card_span(chart, CardSpan::columns(1).with_rows(2))
        .unwrap();

    let chart_rect = rect_of(&mut rt, "chart", 200.0, 200.0);
    let events_rect = rect_of(&mut rt, "events", 200.0, 200.0);
    assert!(
        (chart_rect.h - 200.0).abs() < 0.5,
        "chart should span two rows: {chart_rect:?}"
    );
    assert!(
        (events_rect.x - 100.0).abs() < 0.5 && (events_rect.y - 100.0).abs() < 0.5,
        "events should stack under stats: {events_rect:?}"
    );
}

#[test]
fn direct_tree_wraps_plain_grid_panel() {
    let mut rt = direct_grid_runtime();
    let stats = panel_of(&rt, "stats");

    rt.set_card_span(stats, CardSpan::full_width()).unwrap();

    let stats_rect = rect_of(&mut rt, "stats", 200.0, 200.0);
    assert!(
        (stats_rect.w - 200.0).abs() < 0.5,
        "full-width stats should span the grid: {stats_rect:?}"
    );
    rt.tree().validate().unwrap();
    let wrapper = rt
        .tree()
        .parent(rt.tree().node_for_panel(stats).unwrap())
        .unwrap()
        .unwrap();
    assert!(matches!(
        rt.tree().node(wrapper),
        Some(panes::Node::GridItemWrapper { span, .. }) if *span == CardSpan::full_width()
    ));
}

#[test]
fn direct_tree_span_change_keeps_focus() {
    let mut rt = direct_grid_runtime();
    let events = panel_of(&rt, "events");
    rt.focus(events);

    rt.set_card_span(events, CardSpan::columns(2)).unwrap();

    assert_eq!(rt.focused(), Some(events));
}

#[test]
fn direct_tree_rejects_invalid_span() {
    let mut rt = direct_grid_runtime();
    let chart = panel_of(&rt, "chart");

    let err = rt.set_card_span(chart, CardSpan::columns(0)).unwrap_err();
    assert!(
        matches!(
            err,
            PaneError::InvalidConstraint(panes::ConstraintError::GridSpanZero)
        ),
        "expected GridSpanZero, got {err}"
    );
}

#[test]
fn direct_tree_rejects_panel_outside_grid() {
    let mut tree = panes::LayoutTree::new();
    let (_, n0) = tree.add_panel("a", panes::grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    let pid = rt.tree().panels_by_kind("a")[0];
    let err = rt.set_card_span(pid, CardSpan::columns(2)).unwrap_err();
    assert!(
        matches!(
            err,
            PaneError::InvalidMutation(MutationError::NotGridItem(p)) if p == pid
        ),
        "expected NotGridItem, got {err}"
    );
}

#[test]
fn direct_tree_rejects_unknown_panel() {
    let mut rt = direct_grid_runtime();
    let missing = panel_of(&rt, "events");
    rt.tree_mut().remove_panel(missing).unwrap();

    let err = rt.set_card_span(missing, CardSpan::columns(2)).unwrap_err();
    assert!(
        matches!(err, PaneError::PanelNotFound(p) if p == missing),
        "expected PanelNotFound, got {err}"
    );
}

#[test]
fn auto_fill_set_span() {
    let k = kinds(3);
    let spans: Arc<[CardSpan]> = vec![CardSpan::columns(1); 3].into();
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::Dashboard {
            columns: GridColumnMode::AutoFill { min_width: 100.0 },
            gap: 0.0,
            spans,
            auto_rows: false,
        },
        &k,
    )
    .unwrap();
    let p0 = rt.sequence().get(0).unwrap();

    rt.set_card_span(p0, CardSpan::columns(2)).unwrap();

    // Verify strategy was updated
    match rt.strategy().unwrap() {
        StrategyKind::Dashboard {
            spans,
            columns: GridColumnMode::AutoFill { .. },
            ..
        } => {
            assert_eq!(spans[0], CardSpan::columns(2));
        }
        other => panic!("expected Dashboard with AutoFill, got {other:?}"),
    }
}

#[test]
fn preserves_other_spans() {
    let k = kinds(3);
    let initial_spans: Arc<[CardSpan]> = vec![
        CardSpan::columns(1),
        CardSpan::columns(3),
        CardSpan::full_width(),
    ]
    .into();
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::Dashboard {
            columns: GridColumnMode::Fixed(4),
            gap: 0.0,
            spans: initial_spans,
            auto_rows: false,
        },
        &k,
    )
    .unwrap();
    let p0 = rt.sequence().get(0).unwrap();

    rt.set_card_span(p0, CardSpan::columns(2)).unwrap();

    match rt.strategy().unwrap() {
        StrategyKind::Dashboard { spans, .. } => {
            assert_eq!(spans[0], CardSpan::columns(2));
            assert_eq!(spans[1], CardSpan::columns(3));
            assert_eq!(spans[2], CardSpan::full_width());
        }
        other => panic!("expected Dashboard, got {other:?}"),
    }
}
