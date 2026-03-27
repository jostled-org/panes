use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{Layout, LayoutBuilder, Overlay, StrategyKind};
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

// ---------------------------------------------------------------------------
// panels_at / overlays_at (macro-generated free functions)
// ---------------------------------------------------------------------------

#[test]
fn panels_at_offsets_rects() {
    let mut b = LayoutBuilder::new();
    let left = b.panel("left").unwrap();
    let right = b.panel("right").unwrap();
    b.row(|r| {
        r.add(left);
        r.add(right);
    })
    .unwrap();
    let layout = b.build().unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();

    let origin = egui::pos2(10.0, 5.0);
    let panels: Vec<_> = panes_egui::panels_at(&resolved, origin).collect();
    let base: Vec<_> = panes_egui::panels(&resolved).collect();

    assert_eq!(panels.len(), base.len());
    for (offset_entry, base_entry) in panels.iter().zip(base.iter()) {
        assert!(
            (offset_entry.rect.min.x - (base_entry.rect.min.x + 10.0)).abs() < f32::EPSILON,
            "panels_at should offset x by origin"
        );
        assert!(
            (offset_entry.rect.min.y - (base_entry.rect.min.y + 5.0)).abs() < f32::EPSILON,
            "panels_at should offset y by origin"
        );
    }
}

#[test]
fn overlays_at_offsets_rects() {
    let mut rt = LayoutRuntime::from_strategy(
        StrategyKind::MasterStack {
            master_ratio: 0.5,
            gap: 0.0,
        },
        &[Arc::from("editor"), Arc::from("chat")],
    )
    .unwrap();
    rt.add_overlay("modal", Overlay::center().fixed(40.0, 10.0))
        .unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    let resolved = frame.layout();

    let origin = egui::pos2(20.0, 15.0);
    let base: Vec<_> = panes_egui::overlays(resolved).collect();
    let offset: Vec<_> = panes_egui::overlays_at(resolved, origin).collect();

    assert_eq!(offset.len(), base.len());
    for (offset_entry, base_entry) in offset.iter().zip(base.iter()) {
        assert!(
            (offset_entry.rect.min.x - (base_entry.rect.min.x + 20.0)).abs() < f32::EPSILON,
            "overlays_at should offset x by origin"
        );
        assert!(
            (offset_entry.rect.min.y - (base_entry.rect.min.y + 15.0)).abs() < f32::EPSILON,
            "overlays_at should offset y by origin"
        );
    }
}

// ---------------------------------------------------------------------------
// EguiFrame tests
// ---------------------------------------------------------------------------

fn master_stack_runtime(kinds: &[&str]) -> LayoutRuntime {
    let kind_arcs: Vec<Arc<str>> = kinds.iter().map(|&k| Arc::from(k)).collect();
    LayoutRuntime::from_strategy(
        StrategyKind::MasterStack {
            master_ratio: 0.5,
            gap: 0.0,
        },
        &kind_arcs,
    )
    .unwrap()
}

#[test]
fn resolve_yields_egui_rects() {
    let mut rt = master_stack_runtime(&["editor", "chat", "status"]);
    let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
    let frame = panes_egui::resolve(&mut rt, area).unwrap();

    let panels: Vec<_> = frame.panels().collect();
    assert_eq!(panels.len(), 3, "should yield 3 panels");

    for entry in &panels {
        let r = entry.rect;
        assert!(
            r.max.x <= 800.0 + f32::EPSILON,
            "panel {:?} exceeds width: max_x={}",
            entry.kind,
            r.max.x,
        );
        assert!(
            r.max.y <= 600.0 + f32::EPSILON,
            "panel {:?} exceeds height: max_y={}",
            entry.kind,
            r.max.y,
        );
    }
}

#[test]
fn resolve_get_returns_egui_rect() {
    let mut rt = master_stack_runtime(&["editor", "chat"]);
    let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
    let frame = panes_egui::resolve(&mut rt, area).unwrap();

    let panels: Vec<_> = frame.panels().collect();
    for entry in &panels {
        let rect = frame
            .get(entry.id)
            .expect("get should return Some for known panel");
        assert_eq!(
            rect, entry.rect,
            "get() should match panels() rect for {:?}",
            entry.kind,
        );
    }
}

#[test]
fn resolve_layout_yields_egui_rects() {
    let layout = Layout::dashboard([("a", 1), ("b", 1), ("c", 1), ("d", 1)])
        .build()
        .unwrap();
    let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
    let frame = panes_egui::resolve_layout(&layout, area).unwrap();

    let panels: Vec<_> = frame.panels().collect();
    assert_eq!(panels.len(), 4, "should yield 4 panels");
}

#[test]
fn resolve_layout_diff_is_none() {
    let layout = Layout::master_stack(["editor", "chat"])
        .master_ratio(0.5)
        .build()
        .unwrap();
    let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
    let frame = panes_egui::resolve_layout(&layout, area).unwrap();

    assert!(
        frame.diff().is_none(),
        "stateless resolve should have no diff"
    );
    assert!(
        frame.inner().is_none(),
        "stateless resolve should have no inner frame"
    );
}

#[test]
fn resolve_diff_available_on_runtime_path() {
    let mut rt = master_stack_runtime(&["editor", "chat"]);
    let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));

    let frame = panes_egui::resolve(&mut rt, area).unwrap();
    let diff = frame.diff().expect("runtime path should have diff()");
    assert_eq!(
        diff.added.len(),
        2,
        "first frame should show all panels as added"
    );
    assert!(frame.inner().is_some(), "runtime path should have inner()");
}

#[test]
fn resolve_overlays() {
    let mut rt = master_stack_runtime(&["editor", "chat"]);
    rt.add_overlay("modal", Overlay::center().fixed(40.0, 10.0))
        .unwrap();
    let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
    let frame = panes_egui::resolve(&mut rt, area).unwrap();

    let overlays: Vec<_> = frame.overlays().collect();
    assert_eq!(overlays.len(), 1, "should yield one overlay");
    assert_eq!(overlays[0].kind, "modal");

    let r = overlays[0].rect;
    assert!(
        r.width() > 0.0 && r.height() > 0.0,
        "overlay should have nonzero size"
    );
    assert!(r.max.x <= 800.0 + f32::EPSILON, "overlay exceeds width");
    assert!(r.max.y <= 600.0 + f32::EPSILON, "overlay exceeds height");
}

#[test]
fn get_invalid_panel_id_returns_none() {
    let mut rt = master_stack_runtime(&["editor", "chat"]);
    let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
    let frame = panes_egui::resolve(&mut rt, area).unwrap();

    let invalid = panes::PanelId::from_raw(9999);
    assert!(
        frame.get(invalid).is_none(),
        "should return None for unknown PanelId"
    );
}
