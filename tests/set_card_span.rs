use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{CardSpan, Layout, MutationError, PaneError, StrategyKind};

fn kinds(n: usize) -> Vec<Arc<str>> {
    (0..n).map(|i| Arc::from(format!("p{i}"))).collect()
}

fn dashboard_runtime(n: usize, columns: usize) -> LayoutRuntime {
    let k = kinds(n);
    let spans: Arc<[CardSpan]> = vec![CardSpan::Columns(1); n].into();
    LayoutRuntime::from_strategy(
        StrategyKind::Dashboard {
            columns,
            gap: 0.0,
            spans,
        },
        &k,
    )
    .unwrap()
}

#[test]
fn set_span_columns() {
    let mut rt = dashboard_runtime(2, 4);
    let p0 = rt.sequence().get(0).unwrap();

    rt.set_card_span(p0, CardSpan::Columns(2)).unwrap();

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

    rt.set_card_span(p0, CardSpan::FullWidth).unwrap();

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

    rt.set_card_span(p1, CardSpan::Columns(2)).unwrap();

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

    let err = rt.set_card_span(p0, CardSpan::Columns(2)).unwrap_err();
    assert!(
        matches!(
            err,
            PaneError::InvalidMutation(MutationError::SpanNotSupported)
        ),
        "expected SpanNotSupported, got {err}"
    );
}

#[test]
fn error_on_no_strategy() {
    let mut tree = panes::LayoutTree::new();
    let (_, n0) = tree.add_panel("a", panes::grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n0]).unwrap();
    tree.set_root(root);
    let mut rt = LayoutRuntime::new(tree);

    let pid = rt.tree().panels_by_kind("a")[0];
    let err = rt.set_card_span(pid, CardSpan::Columns(2)).unwrap_err();
    assert!(
        matches!(err, PaneError::InvalidMutation(MutationError::NoStrategy)),
        "expected NoStrategy, got {err}"
    );
}

#[test]
fn auto_fill_set_span() {
    let k = kinds(3);
    let spans: Arc<[CardSpan]> = vec![CardSpan::Columns(1); 3].into();
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::DashboardAutoFill {
            min_width: 100.0,
            gap: 0.0,
            spans,
        },
        &k,
    )
    .unwrap();
    let p0 = rt.sequence().get(0).unwrap();

    rt.set_card_span(p0, CardSpan::Columns(2)).unwrap();

    // Verify strategy was updated
    match rt.strategy().unwrap() {
        StrategyKind::DashboardAutoFill { spans, .. } => {
            assert_eq!(spans[0], CardSpan::Columns(2));
        }
        other => panic!("expected DashboardAutoFill, got {other:?}"),
    }
}

#[test]
fn preserves_other_spans() {
    let k = kinds(3);
    let initial_spans: Arc<[CardSpan]> = vec![
        CardSpan::Columns(1),
        CardSpan::Columns(3),
        CardSpan::FullWidth,
    ]
    .into();
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::Dashboard {
            columns: 4,
            gap: 0.0,
            spans: initial_spans,
        },
        &k,
    )
    .unwrap();
    let p0 = rt.sequence().get(0).unwrap();

    rt.set_card_span(p0, CardSpan::Columns(2)).unwrap();

    match rt.strategy().unwrap() {
        StrategyKind::Dashboard { spans, .. } => {
            assert_eq!(spans[0], CardSpan::Columns(2));
            assert_eq!(spans[1], CardSpan::Columns(3));
            assert_eq!(spans[2], CardSpan::FullWidth);
        }
        other => panic!("expected Dashboard, got {other:?}"),
    }
}
