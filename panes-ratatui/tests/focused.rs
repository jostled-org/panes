use panes::{LayoutBuilder, fixed};

#[test]
fn focused_panel_yields_true() {
    let mut b = LayoutBuilder::new();
    let editor = b.panel("editor").unwrap();
    let terminal = b.panel("terminal").unwrap();
    b.row(|r| {
        r.add(editor);
        r.add(terminal);
    })
    .unwrap();
    let layout = b.build().unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();

    let results: Vec<_> = panes_ratatui::focused_panels(&resolved, Some(editor)).collect();
    let editor_entry = results.iter().find(|(e, _)| e.id == editor).unwrap();
    let terminal_entry = results.iter().find(|(e, _)| e.id == terminal).unwrap();

    assert!(editor_entry.1, "focused panel should be true");
    assert!(!terminal_entry.1, "unrelated panel should be false");
}

#[test]
fn decoration_panel_yields_focused() {
    let mut b = LayoutBuilder::new();
    let editor = b.panel("editor").unwrap();
    let editor_tab = b.panel_with("editor_tab", fixed(1.0)).unwrap();
    let terminal = b.panel("terminal").unwrap();
    b.col(|c| {
        c.add(editor_tab);
        c.add(editor);
        c.add(terminal);
    })
    .unwrap();
    let layout = b.build().unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();

    let results: Vec<_> = panes_ratatui::focused_panels(&resolved, Some(editor)).collect();
    let tab = results.iter().find(|(e, _)| e.id == editor_tab).unwrap();
    let term = results.iter().find(|(e, _)| e.id == terminal).unwrap();

    assert!(
        tab.1,
        "editor_tab decoration should be focused when editor is focused"
    );
    assert!(!term.1, "unrelated panel should not be focused");
}

#[test]
fn title_decoration_yields_focused() {
    let mut b = LayoutBuilder::new();
    let editor = b.panel("editor").unwrap();
    let editor_title = b.panel_with("editor_title", fixed(1.0)).unwrap();
    b.col(|c| {
        c.add(editor_title);
        c.add(editor);
    })
    .unwrap();
    let layout = b.build().unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();

    let results: Vec<_> = panes_ratatui::focused_panels(&resolved, Some(editor)).collect();
    let title = results.iter().find(|(e, _)| e.id == editor_title).unwrap();

    assert!(title.1, "editor_title decoration should be focused");
}

#[test]
fn no_focus_yields_all_false() {
    let mut b = LayoutBuilder::new();
    let a = b.panel("a").unwrap();
    let b_panel = b.panel("b").unwrap();
    b.row(|r| {
        r.add(a);
        r.add(b_panel);
    })
    .unwrap();
    let layout = b.build().unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();

    let all_unfocused = panes_ratatui::focused_panels(&resolved, None).all(|(_, focused)| !focused);
    assert!(all_unfocused, "no focused panel means all should be false");
}

#[test]
fn focused_panels_at_offsets_rects() {
    let mut b = LayoutBuilder::new();
    let editor = b.panel("editor").unwrap();
    let terminal = b.panel("terminal").unwrap();
    b.row(|r| {
        r.add(editor);
        r.add(terminal);
    })
    .unwrap();
    let layout = b.build().unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();

    let origin = ratatui::layout::Rect {
        x: 10,
        y: 5,
        width: 80,
        height: 24,
    };

    let base: Vec<_> = panes_ratatui::focused_panels(&resolved, Some(editor)).collect();
    let offset: Vec<_> =
        panes_ratatui::focused_panels_at(&resolved, Some(editor), origin).collect();

    for (base_item, offset_item) in base.iter().zip(offset.iter()) {
        assert_eq!(base_item.0.id, offset_item.0.id);
        assert_eq!(offset_item.0.rect.x, base_item.0.rect.x + 10);
        assert_eq!(offset_item.0.rect.y, base_item.0.rect.y + 5);
        assert_eq!(base_item.1, offset_item.1, "focus state should match");
    }
}
