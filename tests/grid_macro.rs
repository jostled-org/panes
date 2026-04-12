#![allow(clippy::unwrap_used, clippy::panic)]
use panes::{CardSpan, Grid, Layout};

#[test]
fn layout_macro_grid_auto_fit_and_gap_match_builder_geometry() {
    // Build via macro
    let macro_layout = panes::layout! {
        grid(auto_fit: 200.0, gap: 10.0) {
            panel("a")
            panel("b")
            panel("c")
            panel("d")
        }
    }
    .unwrap();

    // Build via builder API
    let builder_layout = Layout::build_grid(Grid::auto_fit(200.0).gap(10.0), |g| {
        g.panel("a");
        g.panel("b");
        g.panel("c");
        g.panel("d");
    })
    .unwrap();

    let macro_resolved = macro_layout.resolve(800.0, 400.0).unwrap();
    let builder_resolved = builder_layout.resolve(800.0, 400.0).unwrap();

    // Both should produce the same panel geometry
    for kind in &["a", "b", "c", "d"] {
        let m = macro_resolved.get(macro_resolved.by_kind(kind)[0]).unwrap();
        let b = builder_resolved
            .get(builder_resolved.by_kind(kind)[0])
            .unwrap();
        assert!(
            (m.x - b.x).abs() < 0.1
                && (m.y - b.y).abs() < 0.1
                && (m.w - b.w).abs() < 0.1
                && (m.h - b.h).abs() < 0.1,
            "panel '{kind}' geometry differs: macro={m:?}, builder={b:?}"
        );
    }
}

#[test]
fn grid_macro_supports_span_and_full_width_items() {
    let layout = panes::layout! {
        grid(columns: 3, gap: 0.0) {
            panel("a")
            panel("b")
            panel("c")
            panel("spanned", span: 2)
            panel("full", full_width: true)
        }
    }
    .unwrap();

    let resolved = layout.resolve(300.0, 300.0).unwrap();

    // Regular panels: each column is 100px wide in a 3-col, 300px grid
    let rect_a = resolved.get(resolved.by_kind("a")[0]).unwrap();
    assert!(
        (rect_a.w - 100.0).abs() < 1.0,
        "single-col panel should be ~100px: got {}",
        rect_a.w
    );

    // Spanned panel: 2 columns = ~200px
    let rect_spanned = resolved.get(resolved.by_kind("spanned")[0]).unwrap();
    assert!(
        (rect_spanned.w - 200.0).abs() < 1.0,
        "span-2 panel should be ~200px: got {}",
        rect_spanned.w
    );

    // Full-width panel: spans all 3 columns = 300px
    let rect_full = resolved.get(resolved.by_kind("full")[0]).unwrap();
    assert!(
        (rect_full.w - 300.0).abs() < 1.0,
        "full-width panel should be 300px: got {}",
        rect_full.w
    );
}

#[test]
fn grid_macro_inside_row_matches_builder() {
    // Macro version
    let macro_layout = panes::layout! {
        row(gap: 12.0) {
            panel("sidebar", fixed: 240.0)
            grid(auto_fit: 200.0, gap: 12.0, auto_rows: true) {
                panel("metric-a")
                panel("metric-b")
                panel("detail", span: 2)
                panel("actions", full_width: true)
            }
        }
    }
    .unwrap();

    // Builder version
    let mut builder = panes::LayoutBuilder::new();
    builder
        .row_gap(12.0, |r| {
            r.panel_with("sidebar", panes::fixed(240.0));
            r.grid(Grid::auto_fit(200.0).gap(12.0).auto_rows(), |g| {
                g.panel("metric-a");
                g.panel("metric-b");
                g.panel_span("detail", CardSpan::Columns(2));
                g.panel_span("actions", CardSpan::FullWidth);
            });
        })
        .unwrap();
    let builder_layout = builder.build().unwrap();

    let mr = macro_layout.resolve(1000.0, 600.0).unwrap();
    let br = builder_layout.resolve(1000.0, 600.0).unwrap();

    for kind in &["sidebar", "metric-a", "metric-b", "detail", "actions"] {
        let m = mr.get(mr.by_kind(kind)[0]).unwrap();
        let b = br.get(br.by_kind(kind)[0]).unwrap();
        assert!(
            (m.x - b.x).abs() < 0.1
                && (m.y - b.y).abs() < 0.1
                && (m.w - b.w).abs() < 0.1
                && (m.h - b.h).abs() < 0.1,
            "panel '{kind}' geometry differs: macro={m:?}, builder={b:?}"
        );
    }
}

#[test]
fn grid_macro_with_auto_rows_flag() {
    let layout = panes::layout! {
        grid(columns: 2, gap: 4.0, auto_rows: true) {
            panel("a")
            panel("b")
        }
    }
    .unwrap();

    let resolved = layout.resolve(400.0, 300.0).unwrap();
    let panels: Vec<_> = resolved.panels().collect();
    assert_eq!(panels.len(), 2);
    for entry in &panels {
        assert!(entry.rect.w > 0.0);
        assert!(entry.rect.h > 0.0);
    }
}
