use panes::{Layout, LayoutBuilder};
use rustc_hash::FxHashMap;

fn to_egui(layout: &Layout, w: f32, h: f32) -> FxHashMap<panes::PanelId, egui::Rect> {
    let resolved = layout.resolve(w, h).unwrap();
    panes_egui::convert(&resolved)
}

#[test]
fn egui_two_panels_correct_rects() {
    let mut b = LayoutBuilder::new();
    let left = b.panel("left").unwrap();
    let right = b.panel("right").unwrap();
    b.row(|r| {
        r.add(left);
        r.add(right);
    })
    .unwrap();
    let layout = b.build().unwrap();
    let rects = to_egui(&layout, 80.0, 24.0);

    let l = rects[&left];
    let r = rects[&right];

    assert_eq!(l.min, egui::pos2(0.0, 0.0));
    assert_eq!(l.max, egui::pos2(40.0, 24.0));
    assert_eq!(r.min, egui::pos2(40.0, 0.0));
    assert_eq!(r.max, egui::pos2(80.0, 24.0));
}

#[test]
fn egui_preserves_fractional_values() {
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
    let rects = to_egui(&layout, 100.0, 30.0);

    let r0 = rects[&p0];
    let r1 = rects[&p1];
    let r2 = rects[&p2];

    // Fractional positions preserved, not rounded
    let total_w = r0.width() + r1.width() + r2.width();
    assert!((total_w - 100.0).abs() < 0.01);

    // Adjacent panels share edges
    assert!((r0.max.x - r1.min.x).abs() < f32::EPSILON);
    assert!((r1.max.x - r2.min.x).abs() < f32::EPSILON);
}

#[test]
fn egui_nested_layout() {
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
    let rects = to_egui(&layout, 120.0, 40.0);

    let l = rects[&left];
    let tr = rects[&top_right];
    let br = rects[&bot_right];

    // All 3 panels have nonzero rects
    assert!(l.width() > 0.0 && l.height() > 0.0);
    assert!(tr.width() > 0.0 && tr.height() > 0.0);
    assert!(br.width() > 0.0 && br.height() > 0.0);

    // Left panel and right column share the same edge
    assert!((l.max.x - tr.min.x).abs() < f32::EPSILON);
    assert!((l.max.x - br.min.x).abs() < f32::EPSILON);

    // Top-right and bottom-right are stacked vertically
    assert!((tr.max.y - br.min.y).abs() < f32::EPSILON);
}
