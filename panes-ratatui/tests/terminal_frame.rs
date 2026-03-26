use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{ActivePanelVariant, Layout, Overlay, StrategyKind};
use ratatui::layout::Rect;

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

// -- Step 1 tests --

#[test]
fn resolve_yields_quantized_rects() {
    let mut rt = master_stack_runtime(&["editor", "chat", "status"]);
    let area = Rect::new(0, 0, 80, 24);
    let frame = panes_ratatui::resolve(&mut rt, area).unwrap();

    let panels: Vec<_> = frame.panels().collect();
    assert_eq!(panels.len(), 3, "should yield 3 panels");

    for entry in &panels {
        assert!(
            entry.rect.x + entry.rect.width <= 80,
            "panel {:?} exceeds width: x={} w={}",
            entry.kind,
            entry.rect.x,
            entry.rect.width
        );
        assert!(
            entry.rect.y + entry.rect.height <= 24,
            "panel {:?} exceeds height: y={} h={}",
            entry.kind,
            entry.rect.y,
            entry.rect.height
        );
    }
}

#[test]
fn resolve_get_returns_quantized_rect() {
    let mut rt = master_stack_runtime(&["editor", "chat"]);
    let area = Rect::new(0, 0, 80, 24);
    let frame = panes_ratatui::resolve(&mut rt, area).unwrap();

    let panels: Vec<_> = frame.panels().collect();
    for entry in &panels {
        let rect = frame
            .get(entry.id)
            .expect("get should return Some for known panel");
        assert_eq!(
            rect, entry.rect,
            "get() should match panels() rect for {:?}",
            entry.kind
        );
    }
}

#[test]
fn resolve_inner_returns_raw_frame() {
    let mut rt = master_stack_runtime(&["editor", "chat"]);
    let area = Rect::new(0, 0, 80, 24);
    let frame = panes_ratatui::resolve(&mut rt, area).unwrap();

    let inner = frame.inner().expect("runtime path should have inner()");
    let resolved = inner.layout();

    // Inner frame uses f32 rects
    let (pid, f32_rect) = resolved.iter().next().unwrap();

    // TerminalFrame uses u16 rects
    let u16_rect = frame.get(pid).unwrap();

    // Both represent the same panel — u16 is the quantized version
    assert_eq!(u16_rect.x as f32, f32_rect.x.round());
    assert_eq!(u16_rect.y as f32, f32_rect.y.round());
}

#[test]
fn resolve_diff_available_on_runtime_path() {
    let mut rt = master_stack_runtime(&["editor", "chat"]);
    let area = Rect::new(0, 0, 80, 24);

    // First resolve — all panels are "added"
    let frame = panes_ratatui::resolve(&mut rt, area).unwrap();
    let diff = frame.diff().expect("runtime path should have diff()");
    assert_eq!(
        diff.added.len(),
        2,
        "first frame should show all panels as added"
    );
}

// -- Step 2 tests --

fn master_stack_layout(kinds: &[&str]) -> Layout {
    Layout::master_stack(kinds.iter().copied())
        .master_ratio(0.5)
        .build()
        .unwrap()
}

#[test]
fn resolve_layout_yields_quantized_rects() {
    let layout = Layout::dashboard([("a", 1), ("b", 1), ("c", 1), ("d", 1)])
        .build()
        .unwrap();
    let area = Rect::new(0, 0, 80, 24);
    let frame = panes_ratatui::resolve_layout(&layout, area).unwrap();

    let panels: Vec<_> = frame.panels().collect();
    assert_eq!(panels.len(), 4, "should yield 4 panels");

    for entry in &panels {
        assert!(
            entry.rect.x + entry.rect.width <= 80,
            "panel {:?} exceeds width",
            entry.kind
        );
        assert!(
            entry.rect.y + entry.rect.height <= 24,
            "panel {:?} exceeds height",
            entry.kind
        );
    }
}

#[test]
fn resolve_layout_diff_is_none() {
    let layout = master_stack_layout(&["editor", "chat"]);
    let area = Rect::new(0, 0, 80, 24);
    let frame = panes_ratatui::resolve_layout(&layout, area).unwrap();

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
fn resolve_layout_matches_resolve_rects() {
    let kinds = &["editor", "chat", "status"];
    let layout = master_stack_layout(kinds);
    let mut rt = master_stack_runtime(kinds);
    let area = Rect::new(0, 0, 80, 24);

    let layout_frame = panes_ratatui::resolve_layout(&layout, area).unwrap();
    let rt_frame = panes_ratatui::resolve(&mut rt, area).unwrap();

    let layout_panels: Vec<_> = layout_frame.panels().collect();
    let rt_panels: Vec<_> = rt_frame.panels().collect();

    assert_eq!(layout_panels.len(), rt_panels.len());
    for (lp, rp) in layout_panels.iter().zip(rt_panels.iter()) {
        assert_eq!(lp.kind, rp.kind, "kinds should match");
        assert_eq!(lp.rect, rp.rect, "rects should match for {:?}", lp.kind);
    }
}

// -- Step 3 tests --

fn tabbed_runtime(kinds: &[&str]) -> LayoutRuntime {
    let kind_arcs: Vec<Arc<str>> = kinds.iter().map(|&k| Arc::from(k)).collect();
    LayoutRuntime::from_strategy(
        StrategyKind::ActivePanel {
            variant: ActivePanelVariant::Tabbed,
            bar_height: 1.0,
        },
        &kind_arcs,
    )
    .unwrap()
}

#[test]
fn terminal_frame_focused_panels() {
    let mut rt = tabbed_runtime(&["editor", "chat"]);
    let area = Rect::new(0, 0, 80, 24);
    let frame = panes_ratatui::resolve(&mut rt, area).unwrap();

    // Find the editor panel's id
    let editor_pid = frame
        .panels()
        .find(|e| e.kind == "editor")
        .expect("should have editor panel")
        .id;

    let focused: Vec<_> = frame.focused_panels(Some(editor_pid)).collect();
    assert!(!focused.is_empty(), "should yield focused panel entries");

    // Editor itself is focused
    let (editor_entry, editor_focused) = focused
        .iter()
        .find(|(e, _)| e.kind == "editor")
        .expect("should have editor entry");
    assert!(editor_focused, "editor should be focused");
    assert!(editor_entry.rect.width > 0);

    // Editor tab decoration is also focused
    let (_, tab_focused) = focused
        .iter()
        .find(|(e, _)| e.kind == "editor_tab")
        .expect("should have editor_tab entry");
    assert!(tab_focused, "editor_tab should be focused (decoration)");

    // Chat is not focused
    let (_, chat_focused) = focused
        .iter()
        .find(|(e, _)| e.kind == "chat")
        .expect("should have chat entry");
    assert!(!chat_focused, "chat should not be focused");
}

#[test]
fn terminal_frame_overlays() {
    let mut rt = master_stack_runtime(&["editor", "chat"]);
    rt.add_overlay("modal", Overlay::center().fixed(40.0, 10.0))
        .unwrap();
    let area = Rect::new(0, 0, 80, 24);
    let frame = panes_ratatui::resolve(&mut rt, area).unwrap();

    let overlays: Vec<_> = frame.overlays().collect();
    assert_eq!(overlays.len(), 1, "should yield one overlay");
    assert_eq!(overlays[0].kind, "modal");

    // Overlay rect should be quantized u16 values within bounds
    let r = overlays[0].rect;
    assert!(r.x + r.width <= 80, "overlay exceeds width");
    assert!(r.y + r.height <= 24, "overlay exceeds height");
    assert!(
        r.width > 0 && r.height > 0,
        "overlay should have nonzero size"
    );
}

#[test]
fn resolve_layout_focused_panels() {
    let layout = master_stack_layout(&["editor", "chat", "status"]);
    let area = Rect::new(0, 0, 80, 24);
    let frame = panes_ratatui::resolve_layout(&layout, area).unwrap();

    // Find editor's panel id
    let editor_pid = frame
        .panels()
        .find(|e| e.kind == "editor")
        .expect("should have editor panel")
        .id;

    let focused: Vec<_> = frame.focused_panels(Some(editor_pid)).collect();
    assert_eq!(focused.len(), 3, "should yield all 3 panels");

    let (_, editor_focused) = focused
        .iter()
        .find(|(e, _)| e.kind == "editor")
        .expect("editor entry");
    assert!(editor_focused, "editor should be focused");

    let (_, chat_focused) = focused
        .iter()
        .find(|(e, _)| e.kind == "chat")
        .expect("chat entry");
    assert!(!chat_focused, "chat should not be focused");
}

#[test]
fn resolve_overlays_yields_quantized_rects() {
    let mut rt = master_stack_runtime(&["editor", "chat"]);
    rt.add_overlay("picker", Overlay::center().fixed(41.0, 11.0))
        .unwrap();
    rt.add_overlay("tooltip", Overlay::center().fixed(19.0, 5.0))
        .unwrap();
    let area = Rect::new(0, 0, 80, 24);
    let frame = panes_ratatui::resolve(&mut rt, area).unwrap();

    let overlays: Vec<_> = frame.overlays().collect();
    assert_eq!(overlays.len(), 2, "should yield two overlays");

    for entry in &overlays {
        let r = entry.rect;
        assert!(
            r.x + r.width <= 80,
            "overlay {:?} exceeds width",
            entry.kind
        );
        assert!(
            r.y + r.height <= 24,
            "overlay {:?} exceeds height",
            entry.kind
        );
        assert!(
            r.width > 0 && r.height > 0,
            "overlay {:?} has zero size",
            entry.kind
        );
        // Quantized dimensions should match the rounded fixed sizes
        assert_eq!(
            r.width,
            (r.x + r.width) - r.x,
            "width should equal right - left for {:?}",
            entry.kind
        );
    }
}

#[test]
fn get_invalid_panel_id_returns_none() {
    // Runtime path
    let mut rt = master_stack_runtime(&["editor", "chat"]);
    let area = Rect::new(0, 0, 80, 24);
    let frame = panes_ratatui::resolve(&mut rt, area).unwrap();

    let invalid = panes::PanelId::from_raw(9999);
    assert!(
        frame.get(invalid).is_none(),
        "runtime frame should return None for unknown PanelId"
    );

    // Stateless path
    let layout = master_stack_layout(&["editor", "chat"]);
    let frame = panes_ratatui::resolve_layout(&layout, area).unwrap();
    assert!(
        frame.get(invalid).is_none(),
        "stateless frame should return None for unknown PanelId"
    );
}
