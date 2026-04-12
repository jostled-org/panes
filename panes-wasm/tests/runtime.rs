#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use panes::PanelInputKind;
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
    assert!(diff["anchorFailed"].as_array().is_some());
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

#[test]
fn wasm_rejects_invalid_resolve_inputs() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c"]).unwrap();

    let width_err = rt.resolve(f64::NAN, 600.0).err().unwrap();
    assert_eq!(width_err, "width must be finite");

    let height_err = rt.resolve(800.0, f64::INFINITY).err().unwrap();
    assert_eq!(height_err, "height must be finite");

    let range_err = rt.resolve(f64::from(f32::MAX) * 2.0, 600.0).err().unwrap();
    assert_eq!(range_err, "width is out of range for f32");
}

#[test]
fn wasm_rejects_invalid_scroll_inputs() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c"]).unwrap();

    let offset_err = rt.set_scroll_offset(f64::NEG_INFINITY).unwrap_err();
    assert_eq!(offset_err, "offset must be finite");

    let delta_err = rt.scroll_by(f64::from(f32::MAX) * 2.0).unwrap_err();
    assert_eq!(delta_err, "delta is out of range for f32");
}

#[test]
fn wasm_from_preset_uses_shared_core_catalog_names() {
    let catalog = WasmRuntime::available_presets();
    assert!(!catalog.is_empty(), "catalog must not be empty");

    // All DynamicList presets should work with a panel list
    let dynamic_kinds = &["a", "b", "c"];
    for info in catalog
        .iter()
        .filter(|p| p.input == PanelInputKind::DynamicList)
    {
        let result = WasmRuntime::from_preset(info.name, dynamic_kinds);
        assert!(
            result.is_ok(),
            "DynamicList preset '{}' should construct successfully, got: {:?}",
            info.name,
            result.err()
        );
    }

    // FixedSlots presets should work with the correct panel count
    for info in catalog
        .iter()
        .filter(|p| p.input == PanelInputKind::FixedSlots)
    {
        let slots: &[&str] = match info.name {
            "sidebar" | "split" => &["left", "right"],
            "holy-grail" => &["header", "footer", "left", "main", "right"],
            other => panic!("unexpected FixedSlots preset: {other}"),
        };
        let result = WasmRuntime::from_preset(info.name, slots);
        assert!(
            result.is_ok(),
            "FixedSlots preset '{}' should construct successfully, got: {:?}",
            info.name,
            result.err()
        );
    }

    // Unknown names must be rejected
    let result = WasmRuntime::from_preset("nonexistent-preset", &["a"]);
    assert!(result.is_err(), "unknown preset must be rejected");
    let err = result.err().unwrap();
    assert!(
        err.contains("unknown preset"),
        "expected unknown preset error, got: {err}"
    );
}

#[test]
fn wasm_runtime_uses_shared_preset_catalog_without_convenience_path_drift() {
    let catalog = WasmRuntime::available_presets();

    // Every preset in the catalog must resolve and produce consistent
    // fast-path panel data.
    for info in catalog {
        let slots: &[&str] = match info.input {
            PanelInputKind::DynamicList => &["x", "y", "z"],
            PanelInputKind::FixedSlots => match info.name {
                "sidebar" | "split" => &["left", "right"],
                "holy-grail" => &["header", "footer", "left", "main", "right"],
                other => panic!("unhandled FixedSlots preset: {other}"),
            },
        };

        let mut rt = WasmRuntime::from_preset(info.name, slots)
            .unwrap_or_else(|e| panic!("preset '{}' failed to construct: {e}", info.name));

        let mut layout = rt
            .resolve(800.0, 600.0)
            .unwrap_or_else(|e| panic!("preset '{}' failed to resolve: {e}", info.name));

        // Fast-path buffer must contain exactly slots.len() panels
        let buf = layout.panels_buf().to_vec();
        assert_eq!(
            buf.len(),
            slots.len() * 6,
            "preset '{}': expected {} panels in fast path, got {}",
            info.name,
            slots.len(),
            buf.len() / 6,
        );

        // JSON convenience path must agree with fast path
        let json_str = layout
            .panels()
            .unwrap_or_else(|e| panic!("preset '{}' JSON panels failed: {e}", info.name));
        let json_panels: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            json_panels.len(),
            slots.len(),
            "preset '{}': JSON panel count drift from fast path",
            info.name
        );

        // Kind table must cover all kind indices in the buffer
        let kind_table_str = layout
            .kind_table()
            .unwrap_or_else(|e| panic!("preset '{}' kind_table failed: {e}", info.name));
        let kind_table: Vec<String> = serde_json::from_str(&kind_table_str).unwrap();
        for i in 0..slots.len() {
            let kind_idx = buf[i * 6 + 5] as usize;
            assert!(
                kind_idx < kind_table.len(),
                "preset '{}': kindIndex {} out of range for kind_table (len {})",
                info.name,
                kind_idx,
                kind_table.len()
            );
        }
    }
}

#[test]
fn wasm_from_preset_preserves_fixed_slot_arity_errors() {
    let error = WasmRuntime::from_preset("split", &["left", "right", "extra"])
        .err()
        .unwrap();
    assert_eq!(error, "preset 'split' requires exactly 2 panels, got 3");
}

#[test]
fn wasm_rejects_invalid_panel_size_inputs() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c"]).unwrap();
    let layout = rt.resolve(800.0, 600.0).unwrap();
    let panels: Vec<serde_json::Value> = serde_json::from_str(&layout.panels().unwrap()).unwrap();
    let pid = panels[0]["id"].as_u64().unwrap() as u32;

    let width_err = rt.set_panel_size(pid, f64::NAN, 300.0).unwrap_err();
    assert_eq!(width_err, "width must be finite");

    let height_err = rt
        .set_panel_size(pid, 300.0, f64::from(f32::MAX) * 2.0)
        .unwrap_err();
    assert_eq!(height_err, "height is out of range for f32");
}
