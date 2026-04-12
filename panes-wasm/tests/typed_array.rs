#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use panes::{Layout, Overlay};
use panes_wasm::WasmRuntime;

#[test]
fn panels_buf_matches_panels_json() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["editor", "chat", "status"]).unwrap();
    let mut layout = rt.resolve(800.0, 600.0).unwrap();

    let json_str = layout.panels().unwrap();
    let panels: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap();

    let buf = layout.panels_buf();
    assert_eq!(buf.len(), panels.len() * 6);

    for (i, panel) in panels.iter().enumerate() {
        let off = i * 6;
        let id = panel["id"].as_u64().unwrap() as f64;
        let x = panel["rect"]["x"].as_f64().unwrap();
        let y = panel["rect"]["y"].as_f64().unwrap();
        let w = panel["rect"]["w"].as_f64().unwrap();
        let h = panel["rect"]["h"].as_f64().unwrap();
        let kind_index = panel["kindIndex"].as_u64().unwrap() as f64;

        assert_eq!(buf[off], id, "panel {i} id mismatch");
        assert_eq!(buf[off + 1], x, "panel {i} x mismatch");
        assert_eq!(buf[off + 2], y, "panel {i} y mismatch");
        assert_eq!(buf[off + 3], w, "panel {i} w mismatch");
        assert_eq!(buf[off + 4], h, "panel {i} h mismatch");
        assert_eq!(buf[off + 5], kind_index, "panel {i} kindIndex mismatch");
    }
}

#[test]
fn kind_table_matches_kind_indices() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["editor", "chat", "status"]).unwrap();
    let mut layout = rt.resolve(800.0, 600.0).unwrap();

    let kind_table_str = layout.kind_table().unwrap();
    let kind_table: Vec<String> = serde_json::from_str(&kind_table_str).unwrap();

    let json_str = layout.panels().unwrap();
    let panels: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap();

    let buf = layout.panels_buf();
    for (i, panel) in panels.iter().enumerate() {
        let off = i * 6;
        let kind_index = buf[off + 5] as usize;
        let expected_kind = panel["kind"].as_str().unwrap();
        assert_eq!(
            kind_table[kind_index], expected_kind,
            "panel {i}: kind_table[{kind_index}] != {expected_kind}"
        );
    }
}

#[test]
fn panel_count_matches_buffer_length() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c", "d", "e"]).unwrap();
    let mut layout = rt.resolve(800.0, 600.0).unwrap();

    assert_eq!(layout.panel_count(), 5);
    assert_eq!(layout.panels_buf().len(), 30);
}

#[test]
fn panels_buf_single_panel() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["only"]).unwrap();
    let mut layout = rt.resolve(800.0, 600.0).unwrap();

    assert_eq!(layout.panel_count(), 1);
    assert_eq!(layout.panels_buf().len(), 6);
}

#[test]
fn panel_count_matches_buffer_length_before_and_after_buffer_fill() {
    let mut rt = WasmRuntime::from_preset("split", &["left", "right"]).unwrap();
    let mut layout = rt.resolve(800.0, 600.0).unwrap();

    assert_eq!(layout.panel_count(), 2);
    assert_eq!(layout.panels_buf().len() / 6, 2);
    assert_eq!(layout.panel_count(), 2);
}

#[test]
fn panels_buf_reuses_buffer() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c"]).unwrap();
    let mut layout = rt.resolve(800.0, 600.0).unwrap();

    let first = layout.panels_buf().to_vec();
    let second = layout.panels_buf().to_vec();
    assert_eq!(first, second);
}

#[test]
fn wasm_layout_fast_path_returns_panel_and_boundary_data_without_json_primary_path() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c"]).unwrap();
    let mut layout = rt.resolve(800.0, 600.0).unwrap();

    // Panel fast path: panels_buf returns structured f64 data
    let buf = layout.panels_buf().to_vec();
    assert_eq!(buf.len(), 3 * 6, "3 panels × 6 values each");

    // Each panel has positive width and height
    for i in 0..3 {
        let off = i * 6;
        let w = buf[off + 3];
        let h = buf[off + 4];
        assert!(w > 0.0, "panel {i} width must be positive");
        assert!(h > 0.0, "panel {i} height must be positive");
    }

    // Boundary fast path: boundary_at_point_buf returns structured data
    // master-stack with 3 panels has a vertical boundary between left master and right stack
    let first_w = buf[3]; // width of first panel
    let boundary_x = first_w; // boundary is at the right edge of the first panel
    let mid_y = 300.0; // middle of the viewport

    let hit = layout.boundary_at_point_buf(boundary_x, mid_y, 5.0);
    assert_eq!(
        hit.len(),
        4,
        "boundary hit should be 4 f64 values [axis, side1, side2, position]"
    );

    // axis: 0.0 = vertical (boundary between left and right)
    assert_eq!(hit[0], 0.0, "boundary axis should be vertical (0.0)");
    // side IDs should be valid u32 values
    assert!(hit[1] >= 0.0, "side1 must be a valid ID");
    assert!(hit[2] >= 0.0, "side2 must be a valid ID");
    // position should be near the boundary_x
    assert!(
        (hit[3] - boundary_x).abs() < 2.0,
        "position should be near boundary_x"
    );

    // No boundary at a point far from any edge — use the center of the first panel
    let first_cx = buf[1] + buf[3] / 2.0; // x + w/2
    let first_cy = buf[2] + buf[4] / 2.0; // y + h/2
    let no_hit = layout.boundary_at_point_buf(first_cx, first_cy, 1.0);
    assert!(
        no_hit.is_empty(),
        "no boundary should be found at center of a panel"
    );
}

#[test]
fn wasm_fast_paths_remain_authoritative_for_panel_and_boundary_queries() {
    // Verify fast paths are the canonical source and JSON convenience helpers
    // produce identical logical results across multiple layout configurations.
    for preset in &["master-stack", "dwindle", "split"] {
        let panels: &[&str] = match *preset {
            "split" => &["left", "right"],
            _ => &["a", "b", "c"],
        };
        let mut rt = WasmRuntime::from_preset(preset, panels).unwrap();
        let mut layout = rt.resolve(800.0, 600.0).unwrap();

        // Panel fast path vs JSON
        let buf = layout.panels_buf().to_vec();
        let json_str = layout.panels().unwrap();
        let json_panels: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap();

        assert_eq!(
            buf.len(),
            json_panels.len() * 6,
            "preset {preset}: buffer/JSON panel count mismatch"
        );
        for (i, panel) in json_panels.iter().enumerate() {
            let off = i * 6;
            assert_eq!(
                buf[off],
                panel["id"].as_u64().unwrap() as f64,
                "preset {preset} panel {i} id"
            );
            assert_eq!(
                buf[off + 1],
                panel["rect"]["x"].as_f64().unwrap(),
                "preset {preset} panel {i} x"
            );
        }

        // Boundary fast path vs JSON at the edge of the first panel
        let first_right = buf[1] + buf[3]; // x + w of first panel
        let mid_y = 300.0;
        let tolerance = 5.0;

        let json_hit = layout.boundary_at_point(first_right, mid_y, tolerance);
        let buf_hit = layout.boundary_at_point_buf(first_right, mid_y, tolerance);

        match (json_hit, buf_hit.is_empty()) {
            (Some(json_str), false) => {
                let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
                let expected_axis = match buf_hit[0] as u32 {
                    0 => "vertical",
                    1 => "horizontal",
                    _ => panic!("unexpected axis code"),
                };
                assert_eq!(
                    json["axis"].as_str().unwrap(),
                    expected_axis,
                    "preset {preset}: axis mismatch"
                );
                assert_eq!(
                    json["sides"][0].as_u64().unwrap() as f64,
                    buf_hit[1],
                    "preset {preset}: side1 mismatch"
                );
                assert_eq!(
                    json["sides"][1].as_u64().unwrap() as f64,
                    buf_hit[2],
                    "preset {preset}: side2 mismatch"
                );
                assert_eq!(
                    json["position"].as_f64().unwrap(),
                    buf_hit[3],
                    "preset {preset}: position mismatch"
                );
            }
            (None, true) => {} // both agree: no hit
            _ => panic!("preset {preset}: JSON and fast-path disagree on boundary hit"),
        }
    }
}

#[test]
fn wasm_json_helpers_remain_consistent_with_fast_path_outputs() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["editor", "chat", "status"]).unwrap();
    let mut layout = rt.resolve(800.0, 600.0).unwrap();

    // Compare panels() JSON with panels_buf() fast path
    let json_str = layout.panels().unwrap();
    let panels: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap();
    let buf = layout.panels_buf();

    assert_eq!(buf.len(), panels.len() * 6);
    for (i, panel) in panels.iter().enumerate() {
        let off = i * 6;
        assert_eq!(
            buf[off],
            panel["id"].as_u64().unwrap() as f64,
            "panel {i} id"
        );
        assert_eq!(
            buf[off + 1],
            panel["rect"]["x"].as_f64().unwrap(),
            "panel {i} x"
        );
        assert_eq!(
            buf[off + 2],
            panel["rect"]["y"].as_f64().unwrap(),
            "panel {i} y"
        );
        assert_eq!(
            buf[off + 3],
            panel["rect"]["w"].as_f64().unwrap(),
            "panel {i} w"
        );
        assert_eq!(
            buf[off + 4],
            panel["rect"]["h"].as_f64().unwrap(),
            "panel {i} h"
        );
        assert_eq!(
            buf[off + 5],
            panel["kindIndex"].as_u64().unwrap() as f64,
            "panel {i} kindIndex"
        );
    }

    // Compare boundary_at_point() JSON with boundary_at_point_buf() fast path
    let first_w = buf[3];
    let boundary_x = first_w;
    let mid_y = 300.0;
    let tolerance = 5.0;

    let json_hit = layout.boundary_at_point(boundary_x, mid_y, tolerance);
    let buf_hit = layout.boundary_at_point_buf(boundary_x, mid_y, tolerance);

    match (json_hit, buf_hit.is_empty()) {
        (Some(json_str), false) => {
            let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
            let expected_axis = match buf_hit[0] as u32 {
                0 => "vertical",
                1 => "horizontal",
                _ => panic!("unexpected axis code"),
            };
            assert_eq!(json["axis"].as_str().unwrap(), expected_axis);
            let sides = json["sides"].as_array().unwrap();
            assert_eq!(sides[0].as_u64().unwrap() as f64, buf_hit[1]);
            assert_eq!(sides[1].as_u64().unwrap() as f64, buf_hit[2]);
            assert_eq!(json["position"].as_f64().unwrap(), buf_hit[3]);
        }
        (None, true) => {} // both report no hit — consistent
        _ => panic!("JSON and fast-path boundary results disagree on hit presence"),
    }
}

#[test]
fn overlay_failures_json_escapes_kind_strings() {
    let mut runtime = Layout::master_stack(["left", "right"])
        .into_runtime()
        .unwrap();

    let overlay_kind = "overlay\"kind\\line\nfeed";
    runtime
        .add_overlay(
            overlay_kind,
            Overlay::below("missing-panel").fixed(20.0, 10.0),
        )
        .unwrap();

    let frame = runtime.resolve(200.0, 100.0).unwrap();
    let layout = panes_wasm::WasmLayout::from(frame.arc());
    let json = layout.overlay_failures().unwrap();
    let failures: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();

    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0]["kind"].as_str(), Some(overlay_kind));
    assert_eq!(failures[0]["reason"].as_str(), Some("KindNotFound"));
}
