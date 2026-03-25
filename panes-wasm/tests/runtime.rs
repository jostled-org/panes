#![allow(clippy::unwrap_used, clippy::expect_used)]

use panes_wasm::WasmRuntime;

#[test]
fn wasm_runtime_master_stack_resolve() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c"]).unwrap();
    let layout = rt.resolve(800.0, 600.0).unwrap();
    let json = layout.panels().unwrap();
    let panels: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    assert_eq!(panels.len(), 3);
    for panel in &panels {
        let rect = panel.get("rect").unwrap();
        assert!(rect["x"].as_f64().is_some());
        assert!(rect["y"].as_f64().is_some());
        assert!(rect["w"].as_f64().unwrap() > 0.0);
        assert!(rect["h"].as_f64().unwrap() > 0.0);
    }
}

#[test]
fn wasm_runtime_add_remove_panel() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c"]).unwrap();

    let pid = rt.add_panel("d").unwrap();
    let layout = rt.resolve(800.0, 600.0).unwrap();
    let panels: Vec<serde_json::Value> = serde_json::from_str(&layout.panels().unwrap()).unwrap();
    assert_eq!(panels.len(), 4);

    rt.remove_panel(pid).unwrap();
    let layout = rt.resolve(800.0, 600.0).unwrap();
    let panels: Vec<serde_json::Value> = serde_json::from_str(&layout.panels().unwrap()).unwrap();
    assert_eq!(panels.len(), 3);
}

#[test]
fn wasm_runtime_focus_navigation() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c"]).unwrap();
    let _ = rt.resolve(800.0, 600.0).unwrap();

    let first = rt.focused();
    assert!(first.is_some());

    rt.focus_next();
    let second = rt.focused();
    assert!(second.is_some());
    assert_ne!(first, second);
}

#[test]
fn wasm_runtime_panel_sizing() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c"]).unwrap();
    let layout = rt.resolve(800.0, 600.0).unwrap();

    let panels: Vec<serde_json::Value> = serde_json::from_str(&layout.panels().unwrap()).unwrap();
    let pid = panels[0]["id"].as_u64().unwrap() as u32;
    let original_w = panels[0]["rect"]["w"].as_f64().unwrap();

    // Set a specific size
    rt.set_panel_size(pid, 200.0, 300.0).unwrap();
    let layout2 = rt.resolve(800.0, 600.0).unwrap();
    let panels2: Vec<serde_json::Value> = serde_json::from_str(&layout2.panels().unwrap()).unwrap();
    let sized_panel = panels2
        .iter()
        .find(|p| p["id"].as_u64().unwrap() as u32 == pid)
        .unwrap();
    let sized_w = sized_panel["rect"]["w"].as_f64().unwrap();
    assert!((sized_w - original_w).abs() > 1.0);

    // Clear the size, should revert
    rt.clear_panel_size(pid).unwrap();
    let layout3 = rt.resolve(800.0, 600.0).unwrap();
    let panels3: Vec<serde_json::Value> = serde_json::from_str(&layout3.panels().unwrap()).unwrap();
    let reverted_panel = panels3
        .iter()
        .find(|p| p["id"].as_u64().unwrap() as u32 == pid)
        .unwrap();
    let reverted_w = reverted_panel["rect"]["w"].as_f64().unwrap();
    assert!((reverted_w - original_w).abs() < 1.0);
}

#[test]
fn wasm_layout_diff_first_frame_all_added() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c"]).unwrap();
    let _ = rt.resolve(800.0, 600.0).unwrap();

    let diff_json = rt.layout_diff().unwrap();
    let diff: serde_json::Value = serde_json::from_str(&diff_json).unwrap();

    // First frame: all panels are "added"
    let added = diff["added"].as_array().unwrap();
    assert_eq!(added.len(), 3);

    // No other categories populated
    assert!(diff["removed"].as_array().unwrap().is_empty());
    assert!(diff["moved"].as_array().unwrap().is_empty());
    assert!(diff["resized"].as_array().unwrap().is_empty());
}

#[test]
fn wasm_layout_diff_after_add_panel() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c"]).unwrap();
    let _ = rt.resolve(800.0, 600.0).unwrap();

    rt.add_panel("d").unwrap();
    let _ = rt.resolve(800.0, 600.0).unwrap();

    let diff_json = rt.layout_diff().unwrap();
    let diff: serde_json::Value = serde_json::from_str(&diff_json).unwrap();

    // New panel is in "added"
    let added = diff["added"].as_array().unwrap();
    assert_eq!(added.len(), 1);

    // Original panels should be in unchanged or resized (not added)
    assert!(diff["removed"].as_array().unwrap().is_empty());

    let resized = diff["resized"].as_array().unwrap();
    let unchanged = diff["unchanged"].as_array().unwrap();
    assert_eq!(resized.len() + unchanged.len(), 3);

    // Verify resized entries have from/to rects with f64 fields
    for change in resized {
        assert!(change["id"].as_u64().is_some());
        assert!(change["from"]["x"].as_f64().is_some());
        assert!(change["to"]["w"].as_f64().is_some());
    }
}

#[test]
fn wasm_overlay_diff_returns_json() {
    // master-stack has no overlays, so overlay diff should be valid but empty
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c"]).unwrap();
    let _ = rt.resolve(800.0, 600.0).unwrap();

    let diff_json = rt.overlay_diff().unwrap();
    let diff: serde_json::Value = serde_json::from_str(&diff_json).unwrap();

    // Valid JSON with expected structure
    assert!(diff["added"].as_array().is_some());
    assert!(diff["removed"].as_array().is_some());
    assert!(diff["moved"].as_array().is_some());
    assert!(diff["resized"].as_array().is_some());
    assert!(diff["unchanged"].as_array().is_some());
}

#[test]
fn wasm_scroll_offset_default_zero() {
    let rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c"]).unwrap();
    assert_eq!(rt.scroll_offset(), 0.0);
}

#[test]
fn wasm_set_scroll_offset() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c"]).unwrap();
    rt.set_scroll_offset(100.0).unwrap();
    assert_eq!(rt.scroll_offset(), 100.0);
}

#[test]
fn wasm_scroll_by() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c"]).unwrap();
    rt.set_scroll_offset(50.0).unwrap();
    rt.scroll_by(25.0).unwrap();
    assert_eq!(rt.scroll_offset(), 75.0);
}
