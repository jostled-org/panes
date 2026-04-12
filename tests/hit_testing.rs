#![allow(clippy::unwrap_used, clippy::panic)]
use panes::resolver::BoundaryAxis;
use panes::runtime::LayoutRuntime;
use panes::{Layout, Overlay};

// --- Step 8: panel_at_point ---

#[test]
fn panel_at_point_center_of_panel() {
    let layout = Layout::row(["a", "b", "c"]).unwrap();
    let resolved = layout.resolve(300.0, 100.0).unwrap();

    // Three equal panels in a row: a=[0,100), b=[100,200), c=[200,300)
    let b_rect = resolved.by_kind("b")[0];
    let b_resolved = resolved.get(b_rect).unwrap();
    let (cx, cy) = b_resolved.center();

    let hit = resolved.panel_at_point(cx, cy);
    assert_eq!(hit, Some(b_rect));
}

#[test]
fn panel_at_point_outside_all_panels() {
    let layout = Layout::row(["a", "b", "c"]).unwrap();
    let resolved = layout.resolve(300.0, 100.0).unwrap();

    assert_eq!(resolved.panel_at_point(-10.0, -10.0), None);
}

#[test]
fn panel_at_point_on_edge_returns_panel() {
    let layout = Layout::row(["a", "b"]).unwrap();
    let resolved = layout.resolve(200.0, 100.0).unwrap();

    let a_id = resolved.by_kind("a")[0];
    let a_rect = resolved.get(a_id).unwrap();

    // Query at exact origin (inclusive lower bound)
    let hit = resolved.panel_at_point(a_rect.x, a_rect.y);
    assert_eq!(hit, Some(a_id));
}

#[test]
fn panel_at_point_overlay_wins_over_panel() {
    let mut rt = Layout::master_stack(["editor", "chat", "status"])
        .master_ratio(0.6)
        .gap(1.0)
        .into_runtime()
        .unwrap();

    // Add an overlay that covers the center of the viewport
    rt.add_overlay("palette", Overlay::center().fixed(200.0, 100.0))
        .unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    let resolved = frame.layout();

    // Center of viewport — overlay is there
    let overlay_hit = resolved.overlay_at_point(400.0, 300.0);
    assert!(overlay_hit.is_some(), "overlay should be hit at center");

    // panel_at_point should still find the panel beneath
    let panel_hit = resolved.panel_at_point(400.0, 300.0);
    assert!(
        panel_hit.is_some(),
        "panel should still be found beneath overlay"
    );
}

#[test]
fn panel_at_point_ignores_retired_panel_holes() {
    let layout = Layout::row(["a", "tmp1", "b", "tmp2", "c"]).unwrap();
    let mut rt = LayoutRuntime::new(layout.into());

    let initial = rt.resolve(500.0, 100.0).unwrap();
    let tmp1 = initial.layout().by_kind("tmp1")[0];
    let tmp2 = initial.layout().by_kind("tmp2")[0];

    rt.tree_mut().remove_panel(tmp1).unwrap();
    rt.tree_mut().remove_panel(tmp2).unwrap();

    let frame = rt.resolve(300.0, 100.0).unwrap();
    let resolved = frame.layout();
    let b_id = resolved.by_kind("b")[0];
    let b_rect = resolved.get(b_id).unwrap();
    let (cx, cy) = b_rect.center();

    assert_eq!(resolved.panel_at_point(cx, cy), Some(b_id));
}

// --- Step 2: panel_at_point early exit ---

#[test]
fn panel_at_point_returns_topmost_on_overlapping_rects() {
    // Build a simple row layout and resolve it
    let layout = Layout::row(["a", "b"]).unwrap();
    let mut resolved = layout.resolve(200.0, 100.0).unwrap();

    let a_id = resolved.by_kind("a")[0];
    let b_id = resolved.by_kind("b")[0];

    // Verify basic hit-testing works for each panel
    let a_rect = *resolved.get(a_id).unwrap();
    let b_rect = *resolved.get(b_id).unwrap();
    let (a_cx, a_cy) = a_rect.center();
    let (b_cx, b_cy) = b_rect.center();
    assert_eq!(resolved.panel_at_point(a_cx, a_cy), Some(a_id));
    assert_eq!(resolved.panel_at_point(b_cx, b_cy), Some(b_id));

    // Shift all rects left so panel b overlaps where panel a used to be.
    // After shifting by -b_rect.x, panel b starts at x=0 and overlaps panel a's area.
    resolved.shift_x(-b_rect.x);

    // Both panels now cover x=0..a_rect.w (panel a) and x=0..b_rect.w (panel b).
    // The overlap region is at x=0. Query the center of that overlap.
    let overlap_y = a_rect.h / 2.0;
    let hit = resolved.panel_at_point(1.0, overlap_y);

    // b has the higher PanelId.raw(), so it wins the tiebreak
    assert!(
        b_id.raw() > a_id.raw(),
        "b should have higher raw id than a"
    );
    assert_eq!(hit, Some(b_id));
}

#[test]
fn panel_at_point_returns_none_for_empty_space() {
    let mut rt = Layout::master_stack(["editor", "chat", "status"])
        .master_ratio(0.6)
        .gap(10.0)
        .into_runtime()
        .unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    let resolved = frame.layout();

    // Master panel takes 60% = 480px, gap = 10px, stack starts at 490px.
    // Query in the 10px gap between master and stack.
    let gap_x = 485.0;
    let gap_y = 300.0;

    // Verify no panel covers the gap
    let hit = resolved.panel_at_point(gap_x, gap_y);
    assert_eq!(hit, None, "gap area should not hit any panel");
}

// --- Step 9: boundary_at_point ---

#[test]
fn boundary_at_point_between_siblings() {
    let layout = Layout::row(["a", "b"]).unwrap();
    let resolved = layout.resolve(200.0, 100.0).unwrap();

    // Two equal panels: a=[0,100), b=[100,200)
    // Boundary is a vertical line at x=100
    let hit = resolved.boundary_at_point(100.0, 50.0, 5.0);
    assert!(hit.is_some(), "should find boundary between a and b");
    let hit = hit.unwrap();
    assert_eq!(hit.axis, BoundaryAxis::Vertical);
}

#[test]
fn boundary_at_point_no_boundary_in_tolerance() {
    let layout = Layout::row(["a", "b"]).unwrap();
    let resolved = layout.resolve(200.0, 100.0).unwrap();

    // Center of panel "a" — far from any boundary
    let hit = resolved.boundary_at_point(50.0, 50.0, 5.0);
    assert!(hit.is_none(), "no boundary near center of panel");
}

#[test]
fn boundary_at_point_col_returns_horizontal() {
    let layout = Layout::col(["a", "b"]).unwrap();
    let resolved = layout.resolve(100.0, 200.0).unwrap();

    // Two equal panels stacked: a=[0,100), b=[100,200)
    // Boundary is a horizontal line at y=100
    let hit = resolved.boundary_at_point(50.0, 100.0, 5.0);
    assert!(hit.is_some(), "should find boundary between a and b");
    let hit = hit.unwrap();
    assert_eq!(hit.axis, BoundaryAxis::Horizontal);
}

#[test]
fn boundary_at_point_nested_containers() {
    let mut rt = Layout::master_stack(["editor", "chat", "status"])
        .master_ratio(0.6)
        .gap(0.0)
        .into_runtime()
        .unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    let resolved = frame.layout();

    // Master panel takes 60% = 480px, stack container takes 40% = 320px
    // Boundary is a vertical line at x=480
    let hit = resolved.boundary_at_point(480.0, 300.0, 5.0);
    assert!(
        hit.is_some(),
        "should find boundary between master and stack"
    );
    let hit = hit.unwrap();
    assert_eq!(hit.axis, BoundaryAxis::Vertical);
}

// --- Step 4: indexed query model ---

#[test]
fn boundary_hit_testing_uses_same_observable_results_under_indexed_query_model() {
    // Complex layout: row of [col(a, b), col(c, d), col(e, f)]
    // Creates both vertical boundaries (between columns) and horizontal boundaries (within columns)
    let layout = Layout::build_row(|row| {
        row.col(|col| {
            col.panel("a");
            col.panel("b");
        });
        row.col(|col| {
            col.panel("c");
            col.panel("d");
        });
        row.col(|col| {
            col.panel("e");
            col.panel("f");
        });
    })
    .unwrap();
    let resolved = layout.resolve(600.0, 200.0).unwrap();

    // Three equal columns of 200px each: [0,200), [200,400), [400,600)
    // Vertical boundaries at x≈200 and x≈400
    // Each column has two rows of 100px: horizontal boundaries at y≈100

    // Vertical boundary between first and second column
    let hit = resolved.boundary_at_point(200.0, 50.0, 5.0);
    assert!(hit.is_some(), "vertical boundary at x≈200");
    assert_eq!(hit.unwrap().axis, BoundaryAxis::Vertical);

    // Vertical boundary between second and third column
    let hit = resolved.boundary_at_point(400.0, 150.0, 5.0);
    assert!(hit.is_some(), "vertical boundary at x≈400");
    assert_eq!(hit.unwrap().axis, BoundaryAxis::Vertical);

    // Horizontal boundary inside first column (y≈100, x in [0,200))
    let hit = resolved.boundary_at_point(100.0, 100.0, 5.0);
    assert!(hit.is_some(), "horizontal boundary at y≈100 in first col");
    assert_eq!(hit.unwrap().axis, BoundaryAxis::Horizontal);

    // Horizontal boundary inside third column (y≈100, x in [400,600))
    let hit = resolved.boundary_at_point(500.0, 100.0, 5.0);
    assert!(hit.is_some(), "horizontal boundary at y≈100 in third col");
    assert_eq!(hit.unwrap().axis, BoundaryAxis::Horizontal);

    // No boundary at center of a panel
    let hit = resolved.boundary_at_point(100.0, 50.0, 2.0);
    assert!(hit.is_none(), "no boundary at center of panel a");

    // No boundary outside layout
    let hit = resolved.boundary_at_point(-10.0, 50.0, 5.0);
    assert!(hit.is_none(), "no boundary outside layout");

    // Nearest-wins: query equidistant from vertical and horizontal boundaries
    // At (200, 100) with tight tolerance, should find the closest
    let hit = resolved.boundary_at_point(201.0, 50.0, 3.0);
    assert!(hit.is_some(), "should find vertical boundary near x=200");
    assert_eq!(hit.unwrap().axis, BoundaryAxis::Vertical);

    // Repeated queries return consistent results
    let hit1 = resolved.boundary_at_point(200.0, 80.0, 5.0);
    let hit2 = resolved.boundary_at_point(200.0, 80.0, 5.0);
    assert_eq!(hit1.unwrap().axis, hit2.unwrap().axis);
    assert_eq!(hit1.unwrap().position, hit2.unwrap().position);
}
