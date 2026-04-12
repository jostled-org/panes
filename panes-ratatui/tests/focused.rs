#![allow(clippy::unwrap_used, clippy::expect_used)]

use panes::{Layout, LayoutBuilder};

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
    let resolved = Layout::tabbed(["editor", "terminal"])
        .resolve(80.0, 24.0)
        .unwrap();

    let editor_pid = resolved.by_kind("editor")[0];
    let results: Vec<_> = panes_ratatui::focused_panels(&resolved, Some(editor_pid)).collect();

    // Find the tab decoration for "editor" via decoration_panels metadata
    let editor_tab_id = resolved
        .decoration_panels()
        .iter()
        .find(|d| d.content_kind.as_ref() == "editor" && d.role == panes::DecorationRole::Tab)
        .expect("should have editor tab decoration")
        .id;

    let tab = results.iter().find(|(e, _)| e.id == editor_tab_id).unwrap();
    let term = results
        .iter()
        .find(|(e, _)| e.kind == "terminal" && resolved.decoration_role(e.id).is_none())
        .unwrap();

    assert!(
        tab.1,
        "tab decoration should be focused when editor is focused"
    );
    assert!(!term.1, "unrelated panel should not be focused");
}

#[test]
fn title_decoration_yields_focused() {
    let resolved = Layout::stacked(["editor", "terminal"])
        .resolve(80.0, 24.0)
        .unwrap();

    let editor_pid = resolved.by_kind("editor")[0];
    let results: Vec<_> = panes_ratatui::focused_panels(&resolved, Some(editor_pid)).collect();

    let editor_title_id = resolved
        .decoration_panels()
        .iter()
        .find(|d| d.content_kind.as_ref() == "editor" && d.role == panes::DecorationRole::Title)
        .expect("should have editor title decoration")
        .id;

    let title = results
        .iter()
        .find(|(e, _)| e.id == editor_title_id)
        .unwrap();
    assert!(title.1, "title decoration should be focused");
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
fn focused_panels_helper_marks_decoration_nodes_via_runtime_metadata() {
    // Tabbed: Tab decorations track focus via metadata
    let tabbed = Layout::tabbed(["editor", "terminal"])
        .resolve(80.0, 24.0)
        .unwrap();
    let editor_pid = tabbed.by_kind("editor")[0];

    let tab_results: Vec<_> = panes_ratatui::focused_panels(&tabbed, Some(editor_pid)).collect();

    for deco in tabbed.decoration_panels() {
        let (_, is_focused) = tab_results
            .iter()
            .find(|(e, _)| e.id == deco.id)
            .expect("every decoration should appear in focused_panels output");
        let expected = deco.content_kind.as_ref() == "editor";
        assert_eq!(
            *is_focused, expected,
            "{:?}({}) focus mismatch",
            deco.role, deco.content_kind
        );
    }

    // Stacked: Title decorations track focus via metadata
    let stacked = Layout::stacked(["editor", "terminal"])
        .resolve(80.0, 24.0)
        .unwrap();
    let editor_pid = stacked.by_kind("editor")[0];

    let title_results: Vec<_> = panes_ratatui::focused_panels(&stacked, Some(editor_pid)).collect();

    for deco in stacked.decoration_panels() {
        let (_, is_focused) = title_results
            .iter()
            .find(|(e, _)| e.id == deco.id)
            .expect("every decoration should appear in focused_panels output");
        let expected = deco.content_kind.as_ref() == "editor";
        assert_eq!(
            *is_focused, expected,
            "Title({}) focus mismatch",
            deco.content_kind
        );
    }
}

#[test]
fn decoration_kind_index_matches_content_kind_group() {
    let resolved = Layout::tabbed(["editor", "terminal"])
        .resolve(80.0, 24.0)
        .unwrap();

    let results: Vec<_> =
        panes_ratatui::focused_panels(&resolved, Some(resolved.by_kind("editor")[0])).collect();

    let editor_kind_index = results
        .iter()
        .find(|(entry, _)| entry.kind == "editor" && resolved.decoration_role(entry.id).is_none())
        .map(|(entry, _)| entry.kind_index)
        .unwrap();

    let editor_tab_kind_index = resolved
        .decoration_panels()
        .iter()
        .find(|panel| panel.content_kind.as_ref() == "editor")
        .and_then(|panel| {
            results
                .iter()
                .find(|(entry, _)| entry.id == panel.id)
                .map(|(entry, _)| entry.kind_index)
        })
        .unwrap();

    assert_eq!(editor_tab_kind_index, editor_kind_index);
}

// ---------------------------------------------------------------------------
// 3.T2: Focus helpers remain correct under shared frame shell
// ---------------------------------------------------------------------------

fn tabbed_runtime(kinds: &[&str]) -> panes::runtime::LayoutRuntime {
    let kind_arcs: Vec<std::sync::Arc<str>> =
        kinds.iter().map(|&k| std::sync::Arc::from(k)).collect();
    panes::runtime::LayoutRuntime::from_strategy(
        panes::StrategyKind::ActivePanel {
            variant: panes::ActivePanelVariant::Tabbed,
            bar_height: 1.0,
        },
        &kind_arcs,
    )
    .unwrap()
}

#[test]
fn focused_panels_correct_under_runtime_frame_shell() {
    let mut rt = tabbed_runtime(&["editor", "chat"]);
    let area = ratatui::layout::Rect::new(0, 0, 80, 24);
    let frame = panes_ratatui::resolve(&mut rt, area).unwrap();

    let editor_pid = frame
        .panels()
        .find(|e| e.kind == "editor")
        .expect("should have editor panel")
        .id;

    // Focus through TerminalFrame method
    let frame_focused: Vec<_> = frame.focused_panels(Some(editor_pid)).collect();

    // Focus through free function on raw resolved layout
    let resolved = frame.inner().unwrap().layout();
    let free_focused: Vec<_> = panes_ratatui::focused_panels(resolved, Some(editor_pid)).collect();

    // Both paths should yield the same panel count and focus states
    assert_eq!(
        frame_focused.len(),
        free_focused.len(),
        "frame method and free function should yield same panel count"
    );

    for ((fe, ff), (fre, frf)) in frame_focused.iter().zip(free_focused.iter()) {
        assert_eq!(fe.id, fre.id, "panel IDs should match");
        assert_eq!(fe.kind_index, fre.kind_index, "kind indices should match");
        assert_eq!(*ff, *frf, "focus state should match for {:?}", fe.kind);
    }

    // Decoration panels should be present and correctly focused
    let focused_decorations: Vec<_> = frame_focused
        .iter()
        .filter(|(e, is_focused)| *is_focused && resolved.decoration_role(e.id).is_some())
        .collect();
    assert!(
        !focused_decorations.is_empty(),
        "editor's decoration should be focused through the shared frame shell"
    );
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
