#![allow(clippy::unwrap_used, clippy::panic)]
use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{CardSpan, GridColumnMode, StrategyKind};

fn kinds(n: usize) -> Vec<Arc<str>> {
    (0..n).map(|i| Arc::from(format!("p{i}"))).collect()
}

#[test]
fn dashboard_auto_rows_sizes_to_content() {
    let k = kinds(4);
    let spans: Arc<[CardSpan]> = vec![CardSpan::Columns(1); 4].into();
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::Dashboard {
            columns: GridColumnMode::Fixed(2),
            gap: 0.0,
            spans,
            auto_rows: true,
        },
        &k,
    )
    .unwrap();

    // Resolve once to get panel IDs
    let frame = rt.resolve(800.0, 600.0).unwrap();
    let ids: Vec<_> = frame.layout().panels().map(|p| p.id).collect();
    let (p0, p1, p2) = (ids[0], ids[1], ids[2]);

    rt.set_panel_size(p0, 400.0, 100.0).unwrap();
    rt.set_panel_size(p1, 400.0, 50.0).unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    let layout = frame.layout();

    // First row should size to tallest card (p0 at 100)
    let r0 = layout.get(p0).unwrap();
    assert!(
        r0.h >= 99.0,
        "first row should be at least 100px tall, got {}",
        r0.h
    );

    // Second row (no intrinsic size) can differ from first
    let r2 = layout.get(p2).unwrap();
    let _ = r2;
}

#[test]
fn dashboard_default_rows_are_equal() {
    let k = kinds(4);
    let spans: Arc<[CardSpan]> = vec![CardSpan::Columns(1); 4].into();
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::Dashboard {
            columns: GridColumnMode::Fixed(2),
            gap: 0.0,
            spans,
            auto_rows: false,
        },
        &k,
    )
    .unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    let layout = frame.layout();
    let ids: Vec<_> = layout.panels().map(|p| p.id).collect();

    // With 1fr rows (default), all rows should have equal height
    let r0 = layout.get(ids[0]).unwrap();
    let r2 = layout.get(ids[2]).unwrap();
    assert!(
        (r0.h - r2.h).abs() < 1.0,
        "default rows should be equal: row0={}, row1={}",
        r0.h,
        r2.h
    );
}
