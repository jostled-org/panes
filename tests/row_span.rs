use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{
    CardSpan, ColumnSpan, ConstraintError, Grid, GridColumnMode, Layout, PaneError, Rect,
    ResolvedLayout, StrategyKind,
};

#[cfg(test)]
fn rect_of(resolved: &ResolvedLayout, kind: &str) -> Rect {
    *resolved.get(resolved.by_kind(kind)[0]).unwrap()
}

fn assert_rect(actual: Rect, expected: (f32, f32, f32, f32), kind: &str) {
    let (x, y, w, h) = expected;
    assert!(
        (actual.x - x).abs() < 0.5
            && (actual.y - y).abs() < 0.5
            && (actual.w - w).abs() < 0.5
            && (actual.h - h).abs() < 0.5,
        "panel '{kind}': expected {expected:?}, got {actual:?}"
    );
}

/// Chart fills column 1 across both rows; stats and events stack in column 2.
fn assert_tall_chart_geometry(resolved: &ResolvedLayout) {
    assert_rect(
        rect_of(resolved, "chart"),
        (0.0, 0.0, 100.0, 200.0),
        "chart",
    );
    assert_rect(
        rect_of(resolved, "stats"),
        (100.0, 0.0, 100.0, 100.0),
        "stats",
    );
    assert_rect(
        rect_of(resolved, "events"),
        (100.0, 100.0, 100.0, 100.0),
        "events",
    );
}

// -- CardSpan construction --

#[test]
fn card_span_constructors() {
    let span = CardSpan::columns(2).with_rows(3);
    assert_eq!(span.columns, ColumnSpan::Count(2));
    assert_eq!(span.rows, 3);

    let full = CardSpan::full_width();
    assert_eq!(full.columns, ColumnSpan::FullWidth);
    assert_eq!(full.rows, 1);

    assert_eq!(CardSpan::default(), CardSpan::columns(1));
}

#[test]
fn card_span_from_conversions() {
    assert_eq!(CardSpan::from(2), CardSpan::columns(2));
    assert_eq!(CardSpan::from((1, 2)), CardSpan::columns(1).with_rows(2));
    assert_eq!(
        CardSpan::from(ColumnSpan::FullWidth),
        CardSpan::full_width()
    );
}

// -- Geometry --

#[test]
fn dashboard_row_span_from_tuples() {
    let resolved = Layout::dashboard([("chart", (1, 2)), ("stats", (1, 1)), ("events", (1, 1))])
        .columns(2)
        .resolve(200.0, 200.0)
        .unwrap();

    assert_tall_chart_geometry(&resolved);
}

#[test]
fn grid_builder_row_span() {
    let layout = Layout::build_grid(Grid::columns(2), |g| {
        g.panel_span("chart", CardSpan::columns(1).with_rows(2));
        g.panel("stats");
        g.panel("events");
    })
    .unwrap();

    assert_tall_chart_geometry(&layout.resolve(200.0, 200.0).unwrap());
}

#[test]
fn macro_rows_only() {
    let layout = panes::layout! {
        grid(columns: 2) {
            panel("chart", rows: 2)
            panel("stats")
            panel("events")
        }
    }
    .unwrap();

    assert_tall_chart_geometry(&layout.resolve(200.0, 200.0).unwrap());
}

#[test]
fn macro_span_and_rows() {
    let layout = panes::layout! {
        grid(columns: 3) {
            panel("chart", span: 2, rows: 2)
            panel("stats")
            panel("events")
        }
    }
    .unwrap();
    let resolved = layout.resolve(300.0, 200.0).unwrap();

    assert_rect(
        rect_of(&resolved, "chart"),
        (0.0, 0.0, 200.0, 200.0),
        "chart",
    );
    assert_rect(
        rect_of(&resolved, "stats"),
        (200.0, 0.0, 100.0, 100.0),
        "stats",
    );
    assert_rect(
        rect_of(&resolved, "events"),
        (200.0, 100.0, 100.0, 100.0),
        "events",
    );
}

#[test]
fn macro_full_width_and_rows() {
    let layout = panes::layout! {
        grid(columns: 2) {
            panel("banner", full_width: true, rows: 2)
            panel("footer")
        }
    }
    .unwrap();
    let resolved = layout.resolve(200.0, 300.0).unwrap();

    assert_rect(
        rect_of(&resolved, "banner"),
        (0.0, 0.0, 200.0, 200.0),
        "banner",
    );
    assert_rect(
        rect_of(&resolved, "footer"),
        (0.0, 200.0, 100.0, 100.0),
        "footer",
    );
}

// -- Validation --

#[test]
fn zero_rows_rejected() {
    let err = Layout::dashboard([("a", CardSpan::columns(1).with_rows(0))])
        .build()
        .unwrap_err();
    assert!(
        matches!(
            err,
            PaneError::InvalidConstraint(ConstraintError::GridSpanZero)
        ),
        "expected GridSpanZero, got {err:?}"
    );
}

#[test]
fn zero_columns_rejected() {
    let err = Layout::dashboard([("a", 0)]).build().unwrap_err();
    assert!(
        matches!(
            err,
            PaneError::InvalidConstraint(ConstraintError::GridSpanZero)
        ),
        "expected GridSpanZero, got {err:?}"
    );
}

#[test]
fn row_overflow_rejected() {
    let rows = usize::from(u16::MAX) + 1;
    let err = Layout::dashboard([("a", CardSpan::columns(1).with_rows(rows))])
        .build()
        .unwrap_err();
    assert!(
        matches!(
            err,
            PaneError::InvalidConstraint(ConstraintError::GridSpanOverflow(n)) if n == rows
        ),
        "expected GridSpanOverflow, got {err:?}"
    );
}

// -- Runtime --

#[cfg(test)]
fn dashboard_runtime(n: usize, columns: usize) -> LayoutRuntime {
    let kinds: Vec<Arc<str>> = ["chart", "stats", "events"]
        .iter()
        .take(n)
        .map(|k| Arc::from(*k))
        .collect();
    let spans: Arc<[CardSpan]> = vec![CardSpan::columns(1); n].into();
    LayoutRuntime::from_strategy(
        StrategyKind::Dashboard {
            columns: GridColumnMode::Fixed(columns),
            gap: 0.0,
            spans,
            auto_rows: false,
        },
        &kinds,
    )
    .unwrap()
}

#[test]
fn set_card_span_rows() {
    let mut rt = dashboard_runtime(3, 2);
    let chart = rt.sequence().get(0).unwrap();

    rt.set_card_span(chart, CardSpan::columns(1).with_rows(2))
        .unwrap();

    let frame = rt.resolve(200.0, 200.0).unwrap();
    assert_tall_chart_geometry(frame.layout());
}

#[test]
fn tree_snapshot_preserves_row_span() {
    let layout = panes::layout! {
        grid(columns: 2) {
            panel("chart", rows: 2)
            panel("stats")
            panel("events")
        }
    }
    .unwrap();
    let snap = LayoutRuntime::from(layout).snapshot().unwrap();

    let mut restored = LayoutRuntime::from_snapshot(snap).unwrap();
    let frame = restored.resolve(200.0, 200.0).unwrap();
    assert_tall_chart_geometry(frame.layout());
}
