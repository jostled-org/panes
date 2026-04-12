#![allow(clippy::unwrap_used, clippy::expect_used)]

use panes::{Layout, LayoutBuilder, fixed};
use rustc_hash::FxHashMap;

fn to_ratatui(layout: &Layout, w: f32, h: f32) -> FxHashMap<panes::PanelId, ratatui::layout::Rect> {
    let resolved = layout.resolve(w, h).unwrap();
    panes_ratatui::convert(&resolved)
}

#[test]
fn two_equal_panels_no_gap() {
    let mut b = LayoutBuilder::new();
    let left = b.panel("left").unwrap();
    let right = b.panel("right").unwrap();
    b.row(|r| {
        r.add(left);
        r.add(right);
    })
    .unwrap();
    let layout = b.build().unwrap();
    let rects = to_ratatui(&layout, 80.0, 24.0);

    let l = rects[&left];
    let r = rects[&right];

    assert_eq!(l.x, 0);
    assert_eq!(l.width, 40);
    assert_eq!(r.x, 40);
    assert_eq!(r.width, 40);
    // No gap: left edge of right == right edge of left
    assert_eq!(l.x + l.width, r.x);
}

#[test]
fn fractional_positions_round_correctly() {
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
    let rects = to_ratatui(&layout, 100.0, 30.0);

    let r0 = rects[&p0];
    let r1 = rects[&p1];
    let r2 = rects[&p2];

    // Total width must equal viewport width — no pixel lost or gained
    let total_w = r0.width + r1.width + r2.width;
    assert_eq!(total_w, 100);

    // Each panel is adjacent to the next — no gap, no overlap
    assert_eq!(r0.x + r0.width, r1.x);
    assert_eq!(r1.x + r1.width, r2.x);
}

#[test]
fn fixed_panel_quantizes() {
    let mut b = LayoutBuilder::new();
    let sidebar = b.panel_with("sidebar", fixed(20.0)).unwrap();
    let content = b.panel("content").unwrap();
    b.row(|r| {
        r.add(sidebar);
        r.add(content);
    })
    .unwrap();
    let layout = b.build().unwrap();
    let rects = to_ratatui(&layout, 100.0, 40.0);

    let s = rects[&sidebar];
    let c = rects[&content];

    assert_eq!(s.x, 0);
    assert_eq!(s.width, 20);
    assert_eq!(c.x, 20);
    assert_eq!(c.width, 80);
}

#[test]
fn zero_size_panel() {
    let layout = Layout::monocle(["a", "b", "c"]).active(0).build().unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();
    let rects = panes_ratatui::convert(&resolved);

    // Should not panic, and inactive panels have zero dimension
    assert_eq!(rects.len(), 3);
    let mut zero_count = 0;
    for rect in rects.values() {
        if rect.width == 0 || rect.height == 0 {
            zero_count += 1;
        }
    }
    assert_eq!(
        zero_count, 2,
        "two inactive panels should have zero dimension"
    );
}

#[test]
fn nested_layout_quantizes() {
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
    let rects = to_ratatui(&layout, 120.0, 40.0);

    let l = rects[&left];
    let tr = rects[&top_right];
    let br = rects[&bot_right];

    // All 3 panels have nonzero rects
    assert!(l.width > 0 && l.height > 0);
    assert!(tr.width > 0 && tr.height > 0);
    assert!(br.width > 0 && br.height > 0);

    // Left panel and right column share the same edge
    assert_eq!(l.x + l.width, tr.x);
    assert_eq!(l.x + l.width, br.x);

    // Top-right and bottom-right are stacked vertically
    assert_eq!(tr.y + tr.height, br.y);

    // Total width matches viewport
    assert_eq!(l.width + tr.width, 120);
    // Total height of right column matches viewport
    assert_eq!(tr.height + br.height, 40);
}
