use panes::runtime::LayoutRuntime;
use panes::{Layout, Overlay};

fn make_runtime() -> LayoutRuntime {
    Layout::master_stack(["editor", "chat", "status"])
        .master_ratio(0.6)
        .gap(1.0)
        .into_runtime()
        .unwrap()
}

#[test]
fn add_overlay_returns_id() {
    let mut rt = make_runtime();
    let id = rt
        .add_overlay("palette", Overlay::bottom(2.0).full_width().height(5.0))
        .unwrap();
    assert_eq!(id.raw(), 0);
}

#[test]
fn add_overlay_deduplicates_by_kind() {
    let mut rt = make_runtime();
    let id1 = rt
        .add_overlay("palette", Overlay::bottom(2.0).full_width().height(5.0))
        .unwrap();
    let id2 = rt
        .add_overlay("palette", Overlay::center().fixed(100.0, 100.0))
        .unwrap();
    assert_eq!(id1, id2);
}

#[test]
fn remove_overlay_is_noop_for_missing() {
    let mut rt = make_runtime();
    rt.remove_overlay("nonexistent"); // should not panic
}

#[test]
fn remove_overlay_removes() {
    let mut rt = make_runtime();
    rt.add_overlay("palette", Overlay::bottom(2.0).full_width().height(5.0))
        .unwrap();
    rt.remove_overlay("palette");
    assert!(rt.overlay("palette").is_none());
}

#[test]
fn set_overlay_visible_hides_and_shows() {
    let mut rt = make_runtime();
    rt.add_overlay("palette", Overlay::bottom(2.0).full_width().height(5.0))
        .unwrap();

    rt.set_overlay_visible("palette", false);
    assert!(!rt.overlay("palette").unwrap().visible());

    rt.set_overlay_visible("palette", true);
    assert!(rt.overlay("palette").unwrap().visible());
}

#[test]
fn set_overlay_height_updates_dynamic_size() {
    let mut rt = make_runtime();
    rt.add_overlay("palette", Overlay::bottom(2.0).full_width().height(5.0))
        .unwrap();

    rt.set_overlay_height("palette", 10.0).unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    let overlay = frame
        .layout()
        .overlays()
        .find(|e| e.kind == "palette")
        .unwrap();
    assert!((overlay.rect.h - 10.0).abs() < 0.01);
}

#[test]
fn resolve_center_overlay() {
    let mut rt = make_runtime();
    rt.add_overlay("picker", Overlay::center().fixed(200.0, 100.0))
        .unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    let overlay = frame
        .layout()
        .overlays()
        .find(|e| e.kind == "picker")
        .unwrap();
    assert!((overlay.rect.x - 300.0).abs() < 0.01);
    assert!((overlay.rect.y - 250.0).abs() < 0.01);
    assert!((overlay.rect.w - 200.0).abs() < 0.01);
    assert!((overlay.rect.h - 100.0).abs() < 0.01);
}

#[test]
fn resolve_bottom_overlay() {
    let mut rt = make_runtime();
    rt.add_overlay("palette", Overlay::bottom(10.0).full_width().height(50.0))
        .unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    let overlay = frame
        .layout()
        .overlays()
        .find(|e| e.kind == "palette")
        .unwrap();
    assert!((overlay.rect.y - 540.0).abs() < 0.01);
    assert!((overlay.rect.w - 800.0).abs() < 0.01);
}

#[test]
fn resolve_corner_overlays() {
    let mut rt = make_runtime();
    rt.add_overlay("health", Overlay::bottom_left(8.0, 4.0).fixed(120.0, 20.0))
        .unwrap();
    rt.add_overlay("minimap", Overlay::top_right(8.0, 8.0).fixed(150.0, 150.0))
        .unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();

    let health = frame
        .layout()
        .overlays()
        .find(|e| e.kind == "health")
        .unwrap();
    assert!((health.rect.x - 8.0).abs() < 0.01);
    assert!((health.rect.y - 576.0).abs() < 0.01);

    let minimap = frame
        .layout()
        .overlays()
        .find(|e| e.kind == "minimap")
        .unwrap();
    assert!((minimap.rect.x - 642.0).abs() < 0.01);
    assert!((minimap.rect.y - 8.0).abs() < 0.01);
}

#[test]
fn resolve_percent_with_clamp() {
    let mut rt = make_runtime();
    rt.add_overlay(
        "grep",
        Overlay::center()
            .percent_width(75.0)
            .percent_height(75.0)
            .clamp_width(40.0, 700.0)
            .clamp_height(10.0, 500.0),
    )
    .unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    let overlay = frame
        .layout()
        .overlays()
        .find(|e| e.kind == "grep")
        .unwrap();
    // 75% of 800 = 600, clamped to [40, 700] => 600
    assert!((overlay.rect.w - 600.0).abs() < 0.01);
    // 75% of 600 = 450, clamped to [10, 500] => 450
    assert!((overlay.rect.h - 450.0).abs() < 0.01);
}

#[test]
fn resolve_percent_clamped_to_max() {
    let mut rt = make_runtime();
    rt.add_overlay(
        "narrow",
        Overlay::center()
            .percent_width(90.0)
            .clamp_width(10.0, 100.0)
            .height(50.0),
    )
    .unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    let overlay = frame
        .layout()
        .overlays()
        .find(|e| e.kind == "narrow")
        .unwrap();
    // 90% of 800 = 720, clamped to max 100
    assert!((overlay.rect.w - 100.0).abs() < 0.01);
}

#[test]
fn panel_anchored_overlay() {
    let mut rt = make_runtime();
    rt.add_overlay(
        "tooltip",
        Overlay::above("editor").offset(0.0, -2.0).fixed(60.0, 10.0),
    )
    .unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    let overlay = frame
        .layout()
        .overlays()
        .find(|e| e.kind == "tooltip")
        .unwrap();
    assert!((overlay.rect.w - 60.0).abs() < 0.01);
    assert!((overlay.rect.h - 10.0).abs() < 0.01);
}

#[test]
fn panel_anchor_missing_panel_excluded() {
    let mut rt = make_runtime();
    rt.add_overlay("tooltip", Overlay::above("nonexistent").fixed(60.0, 10.0))
        .unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    assert!(
        frame
            .layout()
            .overlays()
            .find(|e| e.kind == "tooltip")
            .is_none()
    );
}

#[test]
fn hidden_overlay_not_resolved() {
    let mut rt = make_runtime();
    rt.add_overlay("palette", Overlay::bottom(2.0).full_width().height(5.0))
        .unwrap();
    rt.set_overlay_visible("palette", false);

    let frame = rt.resolve(800.0, 600.0).unwrap();
    assert!(
        frame
            .layout()
            .overlays()
            .find(|e| e.kind == "palette")
            .is_none()
    );
}

#[test]
fn overlay_diff_first_frame_all_added() {
    let mut rt = make_runtime();
    rt.add_overlay("a", Overlay::center().fixed(100.0, 100.0))
        .unwrap();
    rt.add_overlay("b", Overlay::top(0.0).fixed(200.0, 50.0))
        .unwrap();

    let _frame = rt.resolve(800.0, 600.0).unwrap();
    assert_eq!(rt.last_overlay_diff().added.len(), 2);
    assert!(rt.last_overlay_diff().removed.is_empty());
    assert!(rt.last_overlay_diff().moved.is_empty());
    assert!(rt.last_overlay_diff().resized.is_empty());
    assert!(rt.last_overlay_diff().unchanged.is_empty());
}

#[test]
fn overlay_diff_unchanged() {
    let mut rt = make_runtime();
    rt.add_overlay("a", Overlay::center().fixed(100.0, 100.0))
        .unwrap();

    rt.resolve(800.0, 600.0).unwrap(); // first frame
    let _frame = rt.resolve(800.0, 600.0).unwrap(); // same dimensions

    assert!(rt.last_overlay_diff().added.is_empty());
    assert!(rt.last_overlay_diff().removed.is_empty());
    assert_eq!(rt.last_overlay_diff().unchanged.len(), 1);
}

#[test]
fn overlay_diff_removed() {
    let mut rt = make_runtime();
    rt.add_overlay("a", Overlay::center().fixed(100.0, 100.0))
        .unwrap();

    rt.resolve(800.0, 600.0).unwrap();
    rt.remove_overlay("a");
    let _frame = rt.resolve(800.0, 600.0).unwrap();

    assert_eq!(rt.last_overlay_diff().removed.len(), 1);
    assert!(rt.last_overlay_diff().added.is_empty());
}

#[test]
fn overlay_diff_resized() {
    let mut rt = make_runtime();
    rt.add_overlay("a", Overlay::center().fixed(100.0, 100.0))
        .unwrap();

    rt.resolve(800.0, 600.0).unwrap();
    rt.set_overlay_height("a", 200.0).unwrap();
    let _frame = rt.resolve(800.0, 600.0).unwrap();

    assert!(!rt.last_overlay_diff().resized.is_empty());
}

#[test]
fn overlay_diff_moved() {
    let mut rt = make_runtime();
    rt.add_overlay("a", Overlay::center().fixed(100.0, 100.0))
        .unwrap();

    rt.resolve(800.0, 600.0).unwrap();
    // Resolving at different viewport moves the center overlay
    let _frame = rt.resolve(1000.0, 600.0).unwrap();

    assert!(!rt.last_overlay_diff().moved.is_empty());
}

#[test]
fn base_layout_unaffected_by_overlays() {
    let mut rt1 = make_runtime();
    let frame1 = rt1.resolve(800.0, 600.0).unwrap();
    let panels1: Vec<_> = frame1
        .layout()
        .panels()
        .map(|e| (e.kind.to_string(), *e.rect))
        .collect();

    let mut rt2 = make_runtime();
    rt2.add_overlay("palette", Overlay::bottom(2.0).full_width().height(5.0))
        .unwrap();
    rt2.add_overlay("picker", Overlay::center().fixed(200.0, 100.0))
        .unwrap();
    let frame2 = rt2.resolve(800.0, 600.0).unwrap();
    let panels2: Vec<_> = frame2
        .layout()
        .panels()
        .map(|e| (e.kind.to_string(), *e.rect))
        .collect();

    assert_eq!(panels1.len(), panels2.len());
    for ((k1, r1), (k2, r2)) in panels1.iter().zip(panels2.iter()) {
        assert_eq!(k1, k2);
        assert!((r1.x - r2.x).abs() < 0.01);
        assert!((r1.y - r2.y).abs() < 0.01);
        assert!((r1.w - r2.w).abs() < 0.01);
        assert!((r1.h - r2.h).abs() < 0.01);
    }
}

#[test]
fn overlay_snapshot_round_trip() {
    let mut rt = make_runtime();
    rt.add_overlay("palette", Overlay::bottom(2.0).full_width().height(5.0))
        .unwrap();
    rt.add_overlay("picker", Overlay::center().fixed(200.0, 100.0))
        .unwrap();
    rt.set_overlay_visible("picker", false);

    let snap = rt.snapshot().unwrap();
    let mut rt2 = LayoutRuntime::from_snapshot(snap).unwrap();

    // Palette should exist and be visible
    assert!(rt2.overlay("palette").unwrap().visible());
    // Picker should exist but be hidden
    assert!(!rt2.overlay("picker").unwrap().visible());

    // Resolve should work
    let frame = rt2.resolve(800.0, 600.0).unwrap();
    let palette = frame
        .layout()
        .overlays()
        .find(|e| e.kind == "palette")
        .unwrap();
    assert!((palette.rect.h - 5.0).abs() < 0.01);

    // Hidden picker should not appear in overlays
    assert!(
        frame
            .layout()
            .overlays()
            .find(|e| e.kind == "picker")
            .is_none()
    );
}

#[cfg(feature = "serde")]
#[test]
fn overlay_snapshot_json_round_trip() {
    let mut rt = make_runtime();
    rt.add_overlay("palette", Overlay::bottom(2.0).full_width().height(5.0))
        .unwrap();
    rt.add_overlay("minimap", Overlay::top_right(8.0, 8.0).fixed(150.0, 150.0))
        .unwrap();

    let snap = rt.snapshot().unwrap();
    let json = serde_json::to_string(&snap).unwrap();
    let restored: panes::LayoutSnapshot = serde_json::from_str(&json).unwrap();

    let mut rt2 = LayoutRuntime::from_snapshot(restored).unwrap();
    let frame = rt2.resolve(800.0, 600.0).unwrap();

    assert_eq!(frame.layout().overlays().count(), 2);
}

#[test]
fn overlay_rect_lookup_by_id() {
    let mut rt = make_runtime();
    let id = rt
        .add_overlay("picker", Overlay::center().fixed(200.0, 100.0))
        .unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    let rect = frame.layout().overlay_rect(id).unwrap();
    assert!((rect.w - 200.0).abs() < 0.01);
}

#[test]
fn overlay_z_order_is_insertion_order() {
    let mut rt = make_runtime();
    rt.add_overlay("first", Overlay::center().fixed(100.0, 100.0))
        .unwrap();
    rt.add_overlay("second", Overlay::center().fixed(200.0, 200.0))
        .unwrap();
    rt.add_overlay("third", Overlay::center().fixed(300.0, 300.0))
        .unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    let kinds: Vec<&str> = frame.layout().overlays().map(|e| e.kind).collect();
    assert_eq!(kinds, vec!["first", "second", "third"]);
}

#[test]
fn remove_overlay_preserves_other_indices() {
    let mut rt = make_runtime();
    rt.add_overlay("a", Overlay::center().fixed(100.0, 100.0))
        .unwrap();
    rt.add_overlay("b", Overlay::top(0.0).fixed(200.0, 50.0))
        .unwrap();
    rt.add_overlay("c", Overlay::bottom(0.0).fixed(300.0, 30.0))
        .unwrap();

    rt.remove_overlay("a");

    // b and c should still be resolvable
    let frame = rt.resolve(800.0, 600.0).unwrap();
    let kinds: Vec<&str> = frame.layout().overlays().map(|e| e.kind).collect();
    assert_eq!(kinds, vec!["b", "c"]);
}

#[test]
fn overlay_id_generator_sequential() {
    let mut rt = make_runtime();
    let id0 = rt
        .add_overlay("a", Overlay::center().fixed(10.0, 10.0))
        .unwrap();
    let id1 = rt
        .add_overlay("b", Overlay::center().fixed(10.0, 10.0))
        .unwrap();
    assert_eq!(id0.raw(), 0);
    assert_eq!(id1.raw(), 1);
}
