#![allow(clippy::unwrap_used, clippy::expect_used)]

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
fn panels_buf_reuses_buffer() {
    let mut rt = WasmRuntime::from_preset("master-stack", &["a", "b", "c"]).unwrap();
    let mut layout = rt.resolve(800.0, 600.0).unwrap();

    let first = layout.panels_buf().to_vec();
    let second = layout.panels_buf().to_vec();
    assert_eq!(first, second);
}
