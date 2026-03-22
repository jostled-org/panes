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
