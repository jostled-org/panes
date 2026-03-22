#![allow(clippy::unwrap_used, clippy::expect_used)]

use panes::{Layout, LayoutBuilder};
use rustc_hash::FxHashMap;

fn to_wasm(layout: &Layout, w: f32, h: f32) -> FxHashMap<panes::PanelId, panes_wasm::WasmRect> {
    let resolved = layout.resolve(w, h).unwrap();
    panes_wasm::convert(&resolved)
}

#[test]
fn wasm_two_panels_f64_values() {
    let mut b = LayoutBuilder::new();
    let left = b.panel("left").unwrap();
    let right = b.panel("right").unwrap();
    b.row(|r| {
        r.add(left);
        r.add(right);
    })
    .unwrap();
    let layout = b.build().unwrap();
    let rects = to_wasm(&layout, 80.0, 24.0);

    let l = &rects[&left];
    let r = &rects[&right];

    // f64 values match the f32 source values
    assert!((l.x - 0.0_f64).abs() < f64::EPSILON);
    assert!((l.y - 0.0_f64).abs() < f64::EPSILON);
    assert!((l.w - 40.0_f64).abs() < f64::EPSILON);
    assert!((l.h - 24.0_f64).abs() < f64::EPSILON);

    assert!((r.x - 40.0_f64).abs() < f64::EPSILON);
    assert!((r.y - 0.0_f64).abs() < f64::EPSILON);
    assert!((r.w - 40.0_f64).abs() < f64::EPSILON);
    assert!((r.h - 24.0_f64).abs() < f64::EPSILON);
}

#[test]
fn wasm_preserves_fractional_precision() {
    let mut b = LayoutBuilder::new();
    let p0 = b.panel("a").unwrap();
    let p1 = b.panel("b").unwrap();
    let p2 = b.panel("c").unwrap();
    b.row(|r| {
        r.add(p0);
        r.add(p1);
        r.add(p2);
    })
    .unwrap();
    let layout = b.build().unwrap();
    let rects = to_wasm(&layout, 100.0, 30.0);

    let r0 = &rects[&p0];
    let r1 = &rects[&p1];
    let r2 = &rects[&p2];

    // Fractional values preserved as f64
    let total_w = r0.w + r1.w + r2.w;
    assert!((total_w - 100.0_f64).abs() < 0.01);

    // Adjacent panels share edges
    assert!(((r0.x + r0.w) - r1.x).abs() < 0.01);
    assert!(((r1.x + r1.w) - r2.x).abs() < 0.01);
}

#[test]
fn wasm_nested_layout() {
    let mut b = LayoutBuilder::new();
    let left = b.panel("left").unwrap();
    let top_right = b.panel("top_right").unwrap();
    let bot_right = b.panel("bot_right").unwrap();
    b.row(|r| {
        r.add(left);
        r.col(|c| {
            c.add(top_right);
            c.add(bot_right);
        });
    })
    .unwrap();
    let layout = b.build().unwrap();
    let rects = to_wasm(&layout, 120.0, 40.0);

    let l = &rects[&left];
    let tr = &rects[&top_right];
    let br = &rects[&bot_right];

    // All 3 panels have nonzero f64 rects
    assert!(l.w > 0.0 && l.h > 0.0);
    assert!(tr.w > 0.0 && tr.h > 0.0);
    assert!(br.w > 0.0 && br.h > 0.0);

    // Left panel and right column share the same edge
    assert!(((l.x + l.w) - tr.x).abs() < f64::EPSILON);
    assert!(((l.x + l.w) - br.x).abs() < f64::EPSILON);

    // Top-right and bottom-right are stacked vertically
    assert!(((tr.y + tr.h) - br.y).abs() < f64::EPSILON);
}
