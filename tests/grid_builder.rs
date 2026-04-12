#![allow(clippy::unwrap_used, clippy::panic)]
use panes::{CardSpan, Grid, Layout, LayoutBuilder, LayoutTree, PaneError, TreeError, fixed};
use std::sync::Arc;

#[test]
fn build_grid_fixed_columns_resolves_cards() {
    let layout = Layout::build_grid(Grid::columns(2).gap(8.0), |g| {
        g.panel("a");
        g.panel("b");
        g.panel("c");
        g.panel("d");
    })
    .unwrap();

    let resolved = layout.resolve(400.0, 300.0).unwrap();

    // 2 columns, 8px gap in 400px viewport: each column ~196px
    // 2 rows, 8px gap in 300px viewport: each row ~146px
    let rect_a = resolved.get(resolved.by_kind("a")[0]).unwrap();
    let rect_b = resolved.get(resolved.by_kind("b")[0]).unwrap();
    let rect_c = resolved.get(resolved.by_kind("c")[0]).unwrap();
    let rect_d = resolved.get(resolved.by_kind("d")[0]).unwrap();

    // Row 1: a and b side by side
    assert!(
        (rect_a.y - rect_b.y).abs() < 1.0,
        "a and b should be on the same row: a.y={}, b.y={}",
        rect_a.y,
        rect_b.y
    );
    assert!(
        rect_b.x > rect_a.x,
        "b should be right of a: a.x={}, b.x={}",
        rect_a.x,
        rect_b.x
    );

    // Row 2: c and d side by side
    assert!(
        (rect_c.y - rect_d.y).abs() < 1.0,
        "c and d should be on the same row: c.y={}, d.y={}",
        rect_c.y,
        rect_d.y
    );
    assert!(
        rect_d.x > rect_c.x,
        "d should be right of c: c.x={}, d.x={}",
        rect_c.x,
        rect_d.x
    );

    // Second row below first
    assert!(
        rect_c.y > rect_a.y,
        "c should be below a: a.y={}, c.y={}",
        rect_a.y,
        rect_c.y
    );

    // Column widths approximately equal
    assert!(
        (rect_a.w - rect_b.w).abs() < 1.0,
        "columns should be equal width: a.w={}, b.w={}",
        rect_a.w,
        rect_b.w
    );
}

#[test]
fn build_grid_auto_fit_and_auto_rows_resolve_without_taffy_escape_hatch() {
    let layout = Layout::build_grid(Grid::auto_fit(150.0).gap(10.0).auto_rows(), |g| {
        g.panel("p0");
        g.panel("p1");
        g.panel("p2");
        g.panel("p3");
        g.panel("p4");
        g.panel("p5");
    })
    .unwrap();

    // At 500px wide with min 150px columns and 10px gap:
    // auto-fit should produce 3 columns: (500 - 2*10) / 3 = 160px each
    let resolved = layout.resolve(500.0, 400.0).unwrap();

    let panels: Vec<_> = resolved.panels().collect();
    assert_eq!(panels.len(), 6);

    // All panels should have rects with positive dimensions
    for entry in &panels {
        assert!(entry.rect.w > 0.0, "panel {} has zero width", entry.kind);
        assert!(entry.rect.h > 0.0, "panel {} has zero height", entry.kind);
    }

    // At 300px wide, fewer columns should fit
    let narrow = layout.resolve(300.0, 400.0).unwrap();
    let narrow_panels: Vec<_> = narrow.panels().collect();
    assert_eq!(narrow_panels.len(), 6);

    // Narrower viewport should produce panels at more y-positions (more rows)
    let wide_ys: std::collections::BTreeSet<i32> = panels.iter().map(|e| e.rect.y as i32).collect();
    let narrow_ys: std::collections::BTreeSet<i32> =
        narrow_panels.iter().map(|e| e.rect.y as i32).collect();
    assert!(
        narrow_ys.len() >= wide_ys.len(),
        "narrower viewport should produce at least as many rows: wide={}, narrow={}",
        wide_ys.len(),
        narrow_ys.len()
    );
}

#[test]
fn build_grid_rejects_invalid_columns_gap_and_min_width() {
    // Zero columns
    let err = Layout::build_grid(Grid::columns(0), |g| {
        g.panel("a");
    })
    .unwrap_err();
    assert!(
        matches!(err, PaneError::InvalidTree(TreeError::DashboardNoColumns)),
        "expected DashboardNoColumns, got: {err:?}"
    );

    // Non-finite gap (NaN)
    let err = Layout::build_grid(Grid::columns(2).gap(f32::NAN), |g| {
        g.panel("a");
    })
    .unwrap_err();
    assert!(
        matches!(err, PaneError::InvalidConstraint(_)),
        "expected InvalidConstraint for NaN gap, got: {err:?}"
    );

    // Non-finite gap (infinity)
    let err = Layout::build_grid(Grid::columns(2).gap(f32::INFINITY), |g| {
        g.panel("a");
    })
    .unwrap_err();
    assert!(
        matches!(err, PaneError::InvalidConstraint(_)),
        "expected InvalidConstraint for infinite gap, got: {err:?}"
    );

    // Zero auto-fit min width
    let err = Layout::build_grid(Grid::auto_fit(0.0), |g| {
        g.panel("a");
    })
    .unwrap_err();
    assert!(
        matches!(
            err,
            PaneError::InvalidTree(TreeError::DashboardMinWidthInvalid)
        ),
        "expected DashboardMinWidthInvalid, got: {err:?}"
    );

    // Negative auto-fill min width
    let err = Layout::build_grid(Grid::auto_fill(-10.0), |g| {
        g.panel("a");
    })
    .unwrap_err();
    assert!(
        matches!(
            err,
            PaneError::InvalidTree(TreeError::DashboardMinWidthInvalid)
        ),
        "expected DashboardMinWidthInvalid, got: {err:?}"
    );

    // NaN auto-fit min width
    let err = Layout::build_grid(Grid::auto_fit(f32::NAN), |g| {
        g.panel("a");
    })
    .unwrap_err();
    assert!(
        matches!(
            err,
            PaneError::InvalidTree(TreeError::DashboardMinWidthInvalid)
        ),
        "expected DashboardMinWidthInvalid, got: {err:?}"
    );
}

#[test]
fn build_grid_with_span_resolves_wider_items() {
    let layout = Layout::build_grid(Grid::columns(3).gap(0.0), |g| {
        g.panel("a");
        g.panel("b");
        g.panel("c");
        g.panel_span("wide", CardSpan::FullWidth);
    })
    .unwrap();

    let resolved = layout.resolve(300.0, 200.0).unwrap();

    let rect_a = resolved.get(resolved.by_kind("a")[0]).unwrap();
    let rect_wide = resolved.get(resolved.by_kind("wide")[0]).unwrap();

    // Full-width item should span the entire grid width
    assert!(
        rect_wide.w > rect_a.w * 2.0,
        "full-width item should be wider than a single column: wide.w={}, a.w={}",
        rect_wide.w,
        rect_a.w
    );
    assert!(
        (rect_wide.w - 300.0).abs() < 1.0,
        "full-width item should span full viewport: wide.w={}",
        rect_wide.w
    );
}

#[test]
fn nested_grid_inside_row_resolves_sidebar_and_cards() {
    let mut builder = LayoutBuilder::new();
    builder
        .row(|r| {
            r.panel_with("sidebar", fixed(100.0));
            r.grid(Grid::columns(2).gap(4.0), |g| {
                g.panel("card-a");
                g.panel("card-b");
                g.panel("card-c");
                g.panel("card-d");
            });
        })
        .unwrap();
    let layout = builder.build().unwrap();
    let resolved = layout.resolve(500.0, 300.0).unwrap();

    // Sidebar gets its fixed 100px
    let sidebar = resolved.get(resolved.by_kind("sidebar")[0]).unwrap();
    assert!(
        (sidebar.w - 100.0).abs() < 1.0,
        "sidebar should be 100px wide, got {}",
        sidebar.w
    );
    assert!(
        (sidebar.x).abs() < 1.0,
        "sidebar should start at x=0, got {}",
        sidebar.x
    );

    // Grid cards occupy the remaining 400px
    let card_a = resolved.get(resolved.by_kind("card-a")[0]).unwrap();
    let card_b = resolved.get(resolved.by_kind("card-b")[0]).unwrap();
    assert!(
        card_a.x >= 99.0,
        "card-a should start after the sidebar, got x={}",
        card_a.x
    );
    // Two columns in 400px with 4px gap: each ~198px
    assert!(
        card_a.w > 100.0,
        "card should be wider than the sidebar, got {}",
        card_a.w
    );
    assert!(
        (card_a.w - card_b.w).abs() < 1.0,
        "grid columns should be equal width: a={}, b={}",
        card_a.w,
        card_b.w
    );

    // Second row of cards below first row
    let card_c = resolved.get(resolved.by_kind("card-c")[0]).unwrap();
    assert!(
        card_c.y > card_a.y,
        "card-c should be below card-a: a.y={}, c.y={}",
        card_a.y,
        card_c.y
    );
}

#[test]
fn grid_items_can_contain_nested_row_and_col_structures() {
    let mut builder = LayoutBuilder::new();
    builder
        .grid(Grid::columns(2).gap(0.0), |g| {
            g.panel("simple");
            g.row(|r| {
                r.panel("left");
                r.panel("right");
            });
        })
        .unwrap();
    let layout = builder.build().unwrap();
    let resolved = layout.resolve(400.0, 200.0).unwrap();

    // Simple panel occupies one grid cell
    let simple = resolved.get(resolved.by_kind("simple")[0]).unwrap();
    assert!(simple.w > 0.0);
    assert!(simple.h > 0.0);

    // Nested row children share the second grid cell
    let left = resolved.get(resolved.by_kind("left")[0]).unwrap();
    let right = resolved.get(resolved.by_kind("right")[0]).unwrap();
    assert!(
        left.w > 0.0 && right.w > 0.0,
        "nested children should have positive width"
    );
    assert!(
        (left.y - right.y).abs() < 1.0,
        "left and right should be on the same row: left.y={}, right.y={}",
        left.y,
        right.y
    );
    assert!(
        right.x > left.x,
        "right should be to the right of left: left.x={}, right.x={}",
        left.x,
        right.x
    );
    // Both nested children should be within the right half of the viewport
    assert!(
        left.x >= 199.0,
        "nested row should be in the second grid column, got x={}",
        left.x
    );
}

#[test]
fn grid_layout_resolves_identical_geometry_after_refactor() {
    // Build a 3-column grid dashboard with 5 panels (one full-width span)
    let layout = Layout::build_grid(Grid::columns(3).gap(4.0), |g| {
        g.panel("a");
        g.panel("b");
        g.panel("c");
        g.panel_span("wide", CardSpan::FullWidth);
        g.panel("d");
    })
    .unwrap();

    let resolved = layout.resolve(600.0, 400.0).unwrap();

    // Verify geometry: 3 columns in 600px with 4px gap
    let rect_a = resolved.get(resolved.by_kind("a")[0]).unwrap();
    let rect_b = resolved.get(resolved.by_kind("b")[0]).unwrap();
    let rect_c = resolved.get(resolved.by_kind("c")[0]).unwrap();
    let rect_wide = resolved.get(resolved.by_kind("wide")[0]).unwrap();
    let rect_d = resolved.get(resolved.by_kind("d")[0]).unwrap();

    // Row 1: a, b, c side by side
    assert!((rect_a.y - rect_b.y).abs() < 1.0, "a and b same row");
    assert!((rect_b.y - rect_c.y).abs() < 1.0, "b and c same row");
    assert!(rect_b.x > rect_a.x, "b right of a");
    assert!(rect_c.x > rect_b.x, "c right of b");

    // Columns approximately equal width (taffy grid may produce slight rounding)
    assert!(
        (rect_a.w - rect_b.w).abs() < 2.0,
        "equal column widths: a.w={}, b.w={}",
        rect_a.w,
        rect_b.w
    );
    assert!(
        (rect_b.w - rect_c.w).abs() < 2.0,
        "equal column widths: b.w={}, c.w={}",
        rect_b.w,
        rect_c.w
    );

    // Full-width item spans the viewport
    assert!(
        (rect_wide.w - 600.0).abs() < 1.0,
        "full-width spans entire grid: got {}",
        rect_wide.w
    );
    assert!(rect_wide.y > rect_a.y, "wide row below first row");

    // d on a third row
    assert!(rect_d.y > rect_wide.y, "d below wide row");

    // All panels have positive dimensions
    for (name, r) in [
        ("a", rect_a),
        ("b", rect_b),
        ("c", rect_c),
        ("wide", rect_wide),
        ("d", rect_d),
    ] {
        assert!(r.w > 0.0, "{name} width > 0");
        assert!(r.h > 0.0, "{name} height > 0");
    }

    // Snapshot round-trip: restore and re-resolve, geometry must match
    let mut rt = panes::runtime::LayoutRuntime::new(panes::LayoutTree::from(layout));
    let frame1 = rt.resolve(600.0, 400.0).unwrap();
    let snap = rt.snapshot().unwrap();
    let mut rt2 = panes::runtime::LayoutRuntime::from_snapshot(snap).unwrap();
    let frame2 = rt2.resolve(600.0, 400.0).unwrap();

    let l1 = frame1.layout();
    let l2 = frame2.layout();
    for kind in ["a", "b", "c", "wide", "d"] {
        let r1 = l1.get(l1.by_kind(kind)[0]).unwrap();
        let r2 = l2.get(l2.by_kind(kind)[0]).unwrap();
        assert!(
            (r1.x - r2.x).abs() < 1.0
                && (r1.y - r2.y).abs() < 1.0
                && (r1.w - r2.w).abs() < 1.0
                && (r1.h - r2.h).abs() < 1.0,
            "round-trip mismatch for {kind}: {r1:?} vs {r2:?}"
        );
    }
}

#[test]
fn grid_item_wrapper_allocation_failure_does_not_leave_orphaned_panel_state() {
    let mut builder = LayoutBuilder::new();
    let err = builder
        .grid(Grid::columns(1), |g| {
            let _alloc_failure = LayoutTree::debug_fail_nth_node_alloc(1);
            g.panel_span("orphan", CardSpan::FullWidth);
        })
        .unwrap_err();

    assert!(matches!(
        err,
        PaneError::InvalidTree(TreeError::ArenaOverflow)
    ));

    builder
        .row(|r| {
            r.panel("ok");
        })
        .unwrap();

    let layout = builder.build().unwrap();

    assert_eq!(layout.tree().panel_count(), 1);
    assert!(layout.tree().panels_by_kind("orphan").is_empty());
    assert_eq!(layout.tree().panels_by_kind("ok").len(), 1);
}

#[test]
fn grid_runtime_accepts_panel_insertion_after_construction() {
    // Build a grid layout through the runtime (dashboard takes (kind, span) tuples)
    let mut rt = Layout::dashboard([
        ("a", CardSpan::Columns(1)),
        ("b", CardSpan::Columns(1)),
        ("c", CardSpan::Columns(1)),
    ])
    .columns(2)
    .gap(4.0)
    .into_runtime()
    .unwrap();

    let frame1 = rt.resolve(400.0, 300.0).unwrap();
    assert_eq!(frame1.layout().panel_count(), 3);

    // Add a panel through add_panel
    let new_pid = rt.add_panel(Arc::from("d")).unwrap();

    let frame2 = rt.resolve(400.0, 300.0).unwrap();
    assert_eq!(frame2.layout().panel_count(), 4, "new panel should appear");

    // New panel has a non-zero rect
    let new_rect = frame2.layout().get(new_pid).unwrap();
    assert!(new_rect.w > 0.0, "new panel width > 0");
    assert!(new_rect.h > 0.0, "new panel height > 0");
}
