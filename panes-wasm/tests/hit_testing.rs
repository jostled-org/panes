#![allow(clippy::unwrap_used, clippy::expect_used)]

use panes_wasm::WasmRuntime;

// --- 4.T1: panel_at_point center ---

#[test]
fn wasm_panel_at_point_center() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b"]).unwrap();
    let layout = rt.resolve(200.0, 100.0).unwrap();

    // Parse panels to find "b"'s rect
    let json = layout.panels().unwrap();
    let panels: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    let b_panel = &panels[1];
    let b_id = b_panel["id"].as_u64().unwrap() as u32;
    let bx = b_panel["rect"]["x"].as_f64().unwrap();
    let by = b_panel["rect"]["y"].as_f64().unwrap();
    let bw = b_panel["rect"]["w"].as_f64().unwrap();
    let bh = b_panel["rect"]["h"].as_f64().unwrap();

    // Hit-test at center of "b"
    let hit = layout.panel_at_point(bx + bw / 2.0, by + bh / 2.0);
    assert_eq!(hit, Some(b_id));
}

// --- 4.T2: panel_at_point outside ---

#[test]
fn wasm_panel_at_point_outside() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b"]).unwrap();
    let layout = rt.resolve(200.0, 100.0).unwrap();

    assert_eq!(layout.panel_at_point(-10.0, -10.0), None);
}

// --- 4.T3: boundary_at_point ---

#[test]
fn wasm_boundary_at_point() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b"]).unwrap();
    let layout = rt.resolve(200.0, 100.0).unwrap();

    // The boundary between "a" and "b" in a master-stack with 2 panels
    // should be a vertical line. Find the shared edge by looking at panel rects.
    let json = layout.panels().unwrap();
    let panels: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    let a_rect = &panels[0]["rect"];
    let a_right = a_rect["x"].as_f64().unwrap() + a_rect["w"].as_f64().unwrap();

    // Hit at the boundary edge, mid-height
    let result = layout.boundary_at_point(a_right, 50.0, 5.0);
    let json = result.expect("expected Some(json) for boundary hit");
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(
        parsed.get("axis").is_some(),
        "expected boundary JSON with axis field"
    );
    assert!(parsed.get("position").is_some());
    assert!(parsed.get("sides").is_some());
}

// --- 4.T4: boundary_at_point none ---

#[test]
fn wasm_boundary_at_point_none() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b"]).unwrap();
    let layout = rt.resolve(200.0, 100.0).unwrap();

    // Center of the first panel — far from any boundary
    let json = layout.panels().unwrap();
    let panels: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    let a_rect = &panels[0]["rect"];
    let cx = a_rect["x"].as_f64().unwrap() + a_rect["w"].as_f64().unwrap() / 2.0;
    let cy = a_rect["y"].as_f64().unwrap() + a_rect["h"].as_f64().unwrap() / 2.0;

    let result = layout.boundary_at_point(cx, cy, 2.0);
    assert_eq!(result, None);
}

// --- 4.T5: overlay_at_point ---

#[test]
fn wasm_overlay_at_point() {
    // master-stack has no overlays, so overlay_at_point should return None
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b"]).unwrap();
    let layout = rt.resolve(200.0, 100.0).unwrap();

    assert_eq!(layout.overlay_at_point(50.0, 50.0), None);
}
