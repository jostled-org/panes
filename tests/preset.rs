#![allow(clippy::unwrap_used, clippy::panic)]
use panes::{CardSpan, Grid, Layout, PanelInputKind, Rect};

// -- Step 1: master_stack, sidebar, split --

#[test]
fn master_stack_basic() {
    let resolved = Layout::master_stack(["editor", "chat", "status"])
        .resolve(80.0, 24.0)
        .unwrap();

    let editor = resolved.by_kind("editor")[0];
    let chat = resolved.by_kind("chat")[0];
    let status = resolved.by_kind("status")[0];

    // Master takes left half, stack splits right half vertically
    assert_eq!(
        *resolved.get(editor).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 40.0,
            h: 24.0
        }
    );
    assert_eq!(
        *resolved.get(chat).unwrap(),
        Rect {
            x: 40.0,
            y: 0.0,
            w: 40.0,
            h: 12.0
        }
    );
    assert_eq!(
        *resolved.get(status).unwrap(),
        Rect {
            x: 40.0,
            y: 12.0,
            w: 40.0,
            h: 12.0
        }
    );
}

#[test]
fn master_stack_custom_ratio() {
    let resolved = Layout::master_stack(["editor", "chat", "status"])
        .master_ratio(0.6)
        .resolve(100.0, 24.0)
        .unwrap();

    let editor = resolved.by_kind("editor")[0];
    let chat = resolved.by_kind("chat")[0];

    // Master gets 60%, stack gets 40%
    assert_eq!(resolved.get(editor).unwrap().w, 60.0);
    assert_eq!(resolved.get(chat).unwrap().w, 40.0);
}

#[test]
fn master_stack_with_gap() {
    let resolved = Layout::master_stack(["a", "b", "c"])
        .gap(10.0)
        .resolve(100.0, 30.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];
    let c = resolved.by_kind("c")[0];

    let a_rect = resolved.get(a).unwrap();
    let b_rect = resolved.get(b).unwrap();
    let c_rect = resolved.get(c).unwrap();

    // Gap between master and stack column
    assert!(b_rect.x > a_rect.x + a_rect.w);
    // Gap between stack items
    assert!(c_rect.y > b_rect.y + b_rect.h);
}

#[test]
fn master_stack_single_panel() {
    let resolved = Layout::master_stack(["solo"]).resolve(80.0, 24.0).unwrap();

    let solo = resolved.by_kind("solo")[0];
    assert_eq!(
        *resolved.get(solo).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 80.0,
            h: 24.0
        }
    );
}

#[test]
fn sidebar_basic() {
    let resolved = Layout::sidebar("nav", "content")
        .resolve(100.0, 24.0)
        .unwrap();

    let nav = resolved.by_kind("nav")[0];
    let content = resolved.by_kind("content")[0];

    assert_eq!(
        *resolved.get(nav).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 20.0,
            h: 24.0
        }
    );
    assert_eq!(
        *resolved.get(content).unwrap(),
        Rect {
            x: 20.0,
            y: 0.0,
            w: 80.0,
            h: 24.0
        }
    );
}

#[test]
fn sidebar_custom_width() {
    let resolved = Layout::sidebar("nav", "content")
        .sidebar_width(30.0)
        .resolve(100.0, 24.0)
        .unwrap();

    let nav = resolved.by_kind("nav")[0];
    let content = resolved.by_kind("content")[0];

    assert_eq!(resolved.get(nav).unwrap().w, 30.0);
    assert_eq!(resolved.get(content).unwrap().w, 70.0);
}

#[test]
fn split_horizontal() {
    let resolved = Layout::split("left", "right").resolve(100.0, 24.0).unwrap();

    let left = resolved.by_kind("left")[0];
    let right = resolved.by_kind("right")[0];

    assert_eq!(
        *resolved.get(left).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 50.0,
            h: 24.0
        }
    );
    assert_eq!(
        *resolved.get(right).unwrap(),
        Rect {
            x: 50.0,
            y: 0.0,
            w: 50.0,
            h: 24.0
        }
    );
}

#[test]
fn split_vertical() {
    let resolved = Layout::split("top", "bottom")
        .vertical()
        .resolve(80.0, 24.0)
        .unwrap();

    let top = resolved.by_kind("top")[0];
    let bottom = resolved.by_kind("bottom")[0];

    assert_eq!(
        *resolved.get(top).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 80.0,
            h: 12.0
        }
    );
    assert_eq!(
        *resolved.get(bottom).unwrap(),
        Rect {
            x: 0.0,
            y: 12.0,
            w: 80.0,
            h: 12.0
        }
    );
}

#[test]
fn split_custom_ratio() {
    let resolved = Layout::split("left", "right")
        .ratio(0.7)
        .resolve(100.0, 24.0)
        .unwrap();

    let left = resolved.by_kind("left")[0];
    let right = resolved.by_kind("right")[0];

    assert_eq!(resolved.get(left).unwrap().w, 70.0);
    assert_eq!(resolved.get(right).unwrap().w, 30.0);
}

// -- Step 1: TryFrom --

#[test]
fn master_stack_try_from() {
    let preset = Layout::master_stack(["a", "b"]);
    let layout: Layout = preset.build().unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();
    assert_eq!(resolved.by_kind("a").len(), 1);
    assert_eq!(resolved.by_kind("b").len(), 1);
}

// -- Step 2: Grid-family presets --

#[test]
fn dashboard_uniform() {
    // 4 cards all span-1 in 4-col grid
    let resolved = Layout::dashboard([("a", 1), ("b", 1), ("c", 1), ("d", 1)])
        .resolve(100.0, 100.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];

    assert_eq!(resolved.get(a).unwrap().w, 25.0);
    assert_eq!(resolved.get(b).unwrap().w, 25.0);
}

#[test]
fn dashboard_mixed_spans() {
    // 3 cards: span-2, span-1, span-1 in 4-col grid
    let resolved = Layout::dashboard([("wide", 2), ("narrow1", 1), ("narrow2", 1)])
        .resolve(100.0, 100.0)
        .unwrap();

    let wide = resolved.by_kind("wide")[0];
    let n1 = resolved.by_kind("narrow1")[0];

    assert_eq!(resolved.get(wide).unwrap().w, 50.0);
    assert_eq!(resolved.get(n1).unwrap().w, 25.0);
}

// -- Dashboard auto-fill / auto-fit --

#[test]
fn dashboard_auto_fill_resolves() {
    // 800px wide viewport, 200px min → expect 4 columns
    let resolved = Layout::dashboard([("a", 1), ("b", 1), ("c", 1), ("d", 1)])
        .auto_fill(200.0)
        .resolve(800.0, 600.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];
    assert_eq!(resolved.get(a).unwrap().w, 200.0);
    assert_eq!(resolved.get(b).unwrap().w, 200.0);
}

#[test]
fn dashboard_auto_fill_narrow_viewport() {
    // 300px wide viewport, 200px min → fewer columns than cards
    let resolved = Layout::dashboard([("a", 1), ("b", 1), ("c", 1), ("d", 1)])
        .auto_fill(200.0)
        .resolve(300.0, 600.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    // With 300px and 200px min, only 1 column fits
    assert_eq!(resolved.get(a).unwrap().w, 300.0);
}

#[test]
fn dashboard_auto_fill_with_spans() {
    // span-2 card still works with auto-fill
    let resolved = Layout::dashboard([("wide", 2), ("narrow", 1)])
        .auto_fill(100.0)
        .resolve(400.0, 400.0)
        .unwrap();

    let wide = resolved.by_kind("wide")[0];
    let narrow = resolved.by_kind("narrow")[0];
    assert!(resolved.get(wide).unwrap().w > resolved.get(narrow).unwrap().w);
}

#[test]
fn dashboard_full_width_spans_all_columns() {
    let resolved = Layout::dashboard([
        ("narrow", CardSpan::Columns(1)),
        ("wide", CardSpan::FullWidth),
    ])
    .columns(4)
    .resolve(100.0, 100.0)
    .unwrap();

    let wide = resolved.by_kind("wide")[0];
    assert_eq!(resolved.get(wide).unwrap().w, 100.0);
}

#[test]
fn dashboard_full_width_with_auto_fill() {
    let resolved = Layout::dashboard([
        ("sidebar", CardSpan::Columns(1)),
        ("content", CardSpan::FullWidth),
        ("footer", CardSpan::Columns(1)),
    ])
    .auto_fill(200.0)
    .resolve(800.0, 600.0)
    .unwrap();

    let content = resolved.by_kind("content")[0];
    assert_eq!(resolved.get(content).unwrap().w, 800.0);
}

#[test]
fn dashboard_auto_fill_rejects_zero() {
    let err = Layout::dashboard([("a", 1)])
        .auto_fill(0.0)
        .build()
        .unwrap_err();
    assert!(
        err.to_string().contains("min_column_width"),
        "expected min_column_width error, got: {err}"
    );
}

#[test]
fn dashboard_auto_fit_resolves() {
    let resolved = Layout::dashboard([("a", 1), ("b", 1)])
        .auto_fit(200.0)
        .resolve(800.0, 600.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    // auto-fit with 2 cards in 800px: cards expand to fill
    assert!(resolved.get(a).unwrap().w >= 200.0);
}

// -- Step 3: Recursive presets --

#[test]
fn dwindle_two_panels() {
    // Two panels: simple horizontal split
    let resolved = Layout::dwindle(["a", "b"]).resolve(100.0, 100.0).unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];

    assert_eq!(
        *resolved.get(a).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 50.0,
            h: 100.0
        }
    );
    assert_eq!(
        *resolved.get(b).unwrap(),
        Rect {
            x: 50.0,
            y: 0.0,
            w: 50.0,
            h: 100.0
        }
    );
}

#[test]
fn dwindle_four_panels() {
    // [a,b,c,d] → row { a(50), col { b(50), row { c(25), d(25) } } }
    let resolved = Layout::dwindle(["a", "b", "c", "d"])
        .resolve(100.0, 100.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];
    let c = resolved.by_kind("c")[0];
    let d = resolved.by_kind("d")[0];

    // a takes left half
    assert_eq!(resolved.get(a).unwrap().w, 50.0);
    assert_eq!(resolved.get(a).unwrap().h, 100.0);
    // b takes top-right quarter
    assert_eq!(resolved.get(b).unwrap().x, 50.0);
    assert_eq!(resolved.get(b).unwrap().h, 50.0);
    // c and d split bottom-right
    assert_eq!(resolved.get(c).unwrap().y, 50.0);
    assert_eq!(resolved.get(d).unwrap().y, 50.0);
    assert_eq!(resolved.get(c).unwrap().w, 25.0);
    assert_eq!(resolved.get(d).unwrap().w, 25.0);
}

#[test]
fn dwindle_custom_ratio() {
    let resolved = Layout::dwindle(["a", "b"])
        .ratio(0.6)
        .resolve(100.0, 100.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];

    assert_eq!(resolved.get(a).unwrap().w, 60.0);
    assert_eq!(resolved.get(b).unwrap().w, 40.0);
}

#[test]
fn spiral_two_panels() {
    // For 2 panels, spiral is identical to dwindle
    let resolved = Layout::spiral(["a", "b"]).resolve(100.0, 100.0).unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];

    assert_eq!(
        *resolved.get(a).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 50.0,
            h: 100.0
        }
    );
    assert_eq!(
        *resolved.get(b).unwrap(),
        Rect {
            x: 50.0,
            y: 0.0,
            w: 50.0,
            h: 100.0
        }
    );
}

#[test]
fn spiral_four_panels() {
    // Spiral reverses child order every other level to create rotation.
    // Level 0 (row): [a, rest] — a on left
    // Level 1 (col): [b, rest] — b on top  (same as dwindle)
    // Level 2 (row): [rest, c] — c on RIGHT (reversed from dwindle)
    // So d is on left of bottom-right, c is on right
    let resolved = Layout::spiral(["a", "b", "c", "d"])
        .resolve(100.0, 100.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];
    let c = resolved.by_kind("c")[0];
    let d = resolved.by_kind("d")[0];

    // a takes left half (same as dwindle)
    assert_eq!(resolved.get(a).unwrap().w, 50.0);
    // b takes top of right half (same as dwindle)
    assert_eq!(resolved.get(b).unwrap().x, 50.0);
    assert_eq!(resolved.get(b).unwrap().h, 50.0);
    // At level 2, order is reversed: d first (left), c second (right)
    assert!(resolved.get(d).unwrap().x < resolved.get(c).unwrap().x);
}

#[test]
fn dwindle_single_panel() {
    let resolved = Layout::dwindle(["solo"]).resolve(80.0, 24.0).unwrap();

    let solo = resolved.by_kind("solo")[0];
    assert_eq!(
        *resolved.get(solo).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 80.0,
            h: 24.0
        }
    );
}

// -- Step 4: Multi-region presets --

#[test]
fn centered_master_basic() {
    // 5 panels: master in center, rest alternate left/right
    let resolved = Layout::centered_master(["master", "a", "b", "c", "d"])
        .resolve(100.0, 100.0)
        .unwrap();

    let master = resolved.by_kind("master")[0];
    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];

    let master_rect = resolved.get(master).unwrap();
    let a_rect = resolved.get(a).unwrap();
    let b_rect = resolved.get(b).unwrap();

    // Master is centered (not at x=0, not at rightmost)
    assert!(master_rect.x > 0.0);
    // Left panels are to the left of master
    assert!(a_rect.x < master_rect.x);
    // Right panels are to the right of master
    assert!(b_rect.x > master_rect.x);
}

#[test]
fn centered_master_three_panels() {
    // master + one left + one right
    let resolved = Layout::centered_master(["master", "left", "right"])
        .resolve(100.0, 100.0)
        .unwrap();

    let master = resolved.by_kind("master")[0];
    let left = resolved.by_kind("left")[0];
    let right = resolved.by_kind("right")[0];

    assert!(resolved.get(left).unwrap().x < resolved.get(master).unwrap().x);
    assert!(resolved.get(right).unwrap().x > resolved.get(master).unwrap().x);
}

#[test]
fn centered_master_custom_ratio() {
    let resolved = Layout::centered_master(["master", "a", "b"])
        .master_ratio(0.4)
        .resolve(100.0, 100.0)
        .unwrap();

    let master = resolved.by_kind("master")[0];
    assert_eq!(resolved.get(master).unwrap().w, 40.0);
}

#[test]
fn holy_grail_basic() {
    let resolved = Layout::holy_grail("header", "footer", "left", "main", "right")
        .resolve(100.0, 100.0)
        .unwrap();

    let header = resolved.by_kind("header")[0];
    let footer = resolved.by_kind("footer")[0];
    let left = resolved.by_kind("left")[0];
    let main = resolved.by_kind("main")[0];
    let right = resolved.by_kind("right")[0];

    // Header at top, full width
    assert_eq!(resolved.get(header).unwrap().y, 0.0);
    assert_eq!(resolved.get(header).unwrap().w, 100.0);
    assert_eq!(resolved.get(header).unwrap().h, 1.0);
    // Footer at bottom
    assert_eq!(resolved.get(footer).unwrap().w, 100.0);
    assert_eq!(resolved.get(footer).unwrap().h, 1.0);
    // Left sidebar
    assert_eq!(resolved.get(left).unwrap().w, 20.0);
    // Main content grows
    assert!(resolved.get(main).unwrap().w > resolved.get(left).unwrap().w);
    // Right sidebar
    assert_eq!(resolved.get(right).unwrap().w, 20.0);
}

#[test]
fn holy_grail_custom_sizes() {
    let resolved = Layout::holy_grail("header", "footer", "left", "main", "right")
        .header_height(3.0)
        .footer_height(2.0)
        .sidebar_width(15.0)
        .resolve(100.0, 100.0)
        .unwrap();

    let header = resolved.by_kind("header")[0];
    let footer = resolved.by_kind("footer")[0];
    let left = resolved.by_kind("left")[0];

    assert_eq!(resolved.get(header).unwrap().h, 3.0);
    assert_eq!(resolved.get(footer).unwrap().h, 2.0);
    assert_eq!(resolved.get(left).unwrap().w, 15.0);
}

// -- Step 5: Stateful presets --

#[test]
fn monocle_shows_active() {
    let resolved = Layout::monocle(["a", "b", "c"])
        .resolve(80.0, 24.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];
    let c = resolved.by_kind("c")[0];

    // Active (0) fills viewport
    assert_eq!(
        *resolved.get(a).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 80.0,
            h: 24.0
        }
    );
    // Others have zero height
    assert_eq!(resolved.get(b).unwrap().h, 0.0);
    assert_eq!(resolved.get(c).unwrap().h, 0.0);
}

#[test]
fn monocle_second_active() {
    let resolved = Layout::monocle(["a", "b", "c"])
        .active(1)
        .resolve(80.0, 24.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];

    assert_eq!(resolved.get(a).unwrap().h, 0.0);
    assert_eq!(
        *resolved.get(b).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 80.0,
            h: 24.0
        }
    );
}

#[test]
fn deck_basic() {
    let resolved = Layout::deck(["master", "a", "b"])
        .resolve(80.0, 24.0)
        .unwrap();

    let master = resolved.by_kind("master")[0];
    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];

    // Master takes left half
    assert_eq!(resolved.get(master).unwrap().w, 40.0);
    assert_eq!(resolved.get(master).unwrap().h, 24.0);
    // Active stack card (index 0 = "a") visible
    assert_eq!(resolved.get(a).unwrap().h, 24.0);
    // Inactive card hidden
    assert_eq!(resolved.get(b).unwrap().h, 0.0);
}

#[test]
fn deck_switch_active() {
    let resolved = Layout::deck(["master", "a", "b"])
        .active(1)
        .resolve(80.0, 24.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];

    assert_eq!(resolved.get(a).unwrap().h, 0.0);
    assert_eq!(resolved.get(b).unwrap().h, 24.0);
}

#[test]
fn tabbed_basic() {
    let resolved = Layout::tabbed(["a", "b"]).resolve(80.0, 24.0).unwrap();

    // Tab decorations do not appear in kind-based lookup
    assert!(resolved.by_kind("a_tab").is_empty());

    let a_content = resolved.by_kind("a")[0];
    let b_content = resolved.by_kind("b")[0];

    // Active content fills remaining height (24 - 1 tab bar = 23)
    assert_eq!(resolved.get(a_content).unwrap().h, 23.0);
    // Inactive content hidden
    assert_eq!(resolved.get(b_content).unwrap().h, 0.0);

    // Tab decoration panels exist with geometry via decoration_panels()
    let tab_decorations: Vec<_> = resolved
        .decoration_panels()
        .iter()
        .filter(|d| d.role == panes::DecorationRole::Tab)
        .collect();
    assert_eq!(tab_decorations.len(), 2);
    let a_tab_rect = resolved.get(tab_decorations[0].id).unwrap();
    assert_eq!(a_tab_rect.h, 1.0);
}

#[test]
fn tabbed_switch() {
    let resolved = Layout::tabbed(["a", "b"])
        .active(1)
        .resolve(80.0, 24.0)
        .unwrap();

    let a_content = resolved.by_kind("a")[0];
    let b_content = resolved.by_kind("b")[0];

    assert_eq!(resolved.get(a_content).unwrap().h, 0.0);
    assert_eq!(resolved.get(b_content).unwrap().h, 23.0);
}

#[test]
fn stacked_basic() {
    let resolved = Layout::stacked(["a", "b"]).resolve(80.0, 24.0).unwrap();

    // Title decorations do not appear in kind-based lookup
    assert!(resolved.by_kind("a_title").is_empty());
    assert!(resolved.by_kind("b_title").is_empty());

    let a_content = resolved.by_kind("a")[0];
    let b_content = resolved.by_kind("b")[0];

    // Active content grows to fill remaining space (24 - 2 titles = 22)
    assert_eq!(resolved.get(a_content).unwrap().h, 22.0);
    // Inactive content hidden
    assert_eq!(resolved.get(b_content).unwrap().h, 0.0);

    // Title decoration panels exist with geometry
    let title_decorations: Vec<_> = resolved
        .decoration_panels()
        .iter()
        .filter(|d| d.role == panes::DecorationRole::Title)
        .collect();
    assert_eq!(title_decorations.len(), 2);
    for d in &title_decorations {
        assert_eq!(resolved.get(d.id).unwrap().h, 1.0);
    }
}

#[test]
fn stacked_switch() {
    let resolved = Layout::stacked(["a", "b"])
        .active(1)
        .resolve(80.0, 24.0)
        .unwrap();

    let a_content = resolved.by_kind("a")[0];
    let b_content = resolved.by_kind("b")[0];

    assert_eq!(resolved.get(a_content).unwrap().h, 0.0);
    assert_eq!(resolved.get(b_content).unwrap().h, 22.0);
}

// -- Decoration node identity (Step 1) --

#[test]
fn tabbed_content_kinds_resolve_without_synthesized_tab_kinds() {
    // Use kinds that would previously collide with _tab suffix
    let resolved = Layout::tabbed(["tab", "editor_tab", "logs"])
        .resolve(80.0, 24.0)
        .unwrap();

    // Content panels resolve by their original kinds
    assert_eq!(resolved.by_kind("tab").len(), 1);
    assert_eq!(resolved.by_kind("editor_tab").len(), 1);
    assert_eq!(resolved.by_kind("logs").len(), 1);

    // No synthesized _tab kinds in the kind namespace
    assert!(resolved.by_kind("tab_tab").is_empty());
    assert!(resolved.by_kind("editor_tab_tab").is_empty());
    assert!(resolved.by_kind("logs_tab").is_empty());

    // Decoration panels are accessible via decoration_panels()
    let decorations = resolved.decoration_panels();
    assert_eq!(decorations.len(), 3);
    for d in decorations {
        assert_eq!(d.role, panes::DecorationRole::Tab);
    }
}

#[test]
fn stacked_content_kinds_resolve_without_synthesized_title_kinds() {
    // Use kinds that would previously collide with _title suffix
    let resolved = Layout::stacked(["title", "editor_title", "logs"])
        .resolve(80.0, 24.0)
        .unwrap();

    // Content panels resolve by their original kinds
    assert_eq!(resolved.by_kind("title").len(), 1);
    assert_eq!(resolved.by_kind("editor_title").len(), 1);
    assert_eq!(resolved.by_kind("logs").len(), 1);

    // No synthesized _title kinds in the kind namespace
    assert!(resolved.by_kind("title_title").is_empty());
    assert!(resolved.by_kind("editor_title_title").is_empty());
    assert!(resolved.by_kind("logs_title").is_empty());

    // Decoration panels are accessible via decoration_panels()
    let decorations = resolved.decoration_panels();
    assert_eq!(decorations.len(), 3);
    for d in decorations {
        assert_eq!(d.role, panes::DecorationRole::Title);
    }
}

#[test]
fn scrollable_focus_zero_shows_first_pair() {
    let resolved = Layout::scrollable(["a", "b", "c"])
        .active(0)
        .resolve(100.0, 24.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];
    let c = resolved.by_kind("c")[0];

    // focus=0: window=0, showing (a, b)
    assert!(resolved.get(a).unwrap().w > 0.0);
    assert!(resolved.get(b).unwrap().w > 0.0);
    assert_eq!(resolved.get(c).unwrap().w, 0.0);
}

#[test]
fn scrollable_focus_one_stays_in_first_pair() {
    let resolved = Layout::scrollable(["a", "b", "c"])
        .active(1)
        .resolve(100.0, 24.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];
    let c = resolved.by_kind("c")[0];

    // focus=1: window=0, still showing (a, b)
    assert!(resolved.get(a).unwrap().w > 0.0);
    assert!(resolved.get(b).unwrap().w > 0.0);
    assert_eq!(resolved.get(c).unwrap().w, 0.0);
}

#[test]
fn scrollable_focus_two_shifts_window() {
    let resolved = Layout::scrollable(["a", "b", "c"])
        .active(2)
        .resolve(100.0, 24.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];
    let c = resolved.by_kind("c")[0];

    // focus=2: window=1, showing (b, c)
    assert_eq!(resolved.get(a).unwrap().w, 0.0);
    assert!(resolved.get(b).unwrap().w > 0.0);
    assert!(resolved.get(c).unwrap().w > 0.0);
}

#[test]
fn scrollable_single_panel_fills_viewport() {
    let resolved = Layout::scrollable(["a"]).resolve(100.0, 24.0).unwrap();

    let a = resolved.by_kind("a")[0];
    assert_eq!(resolved.get(a).unwrap().w, 100.0);
}

// -- Preset catalog --

#[test]
fn presets_returns_13_entries() {
    assert_eq!(Layout::presets().len(), 13);
}

#[test]
fn presets_names_are_sorted() {
    let names: Vec<&str> = Layout::presets().iter().map(|p| p.name).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
}

#[test]
fn presets_fixed_slots_are_sidebar_holy_grail_split() {
    let mut fixed: Vec<&str> = Layout::presets()
        .iter()
        .filter(|p| p.input == PanelInputKind::FixedSlots)
        .map(|p| p.name)
        .collect();
    fixed.sort_unstable();
    assert_eq!(fixed, vec!["holy-grail", "sidebar", "split"]);
}

// -- Dashboard parity with shared grid primitive --

#[test]
fn tabbed_and_stacked_builders_remain_behaviorally_equivalent_under_shared_shell() {
    let kinds = ["alpha", "beta", "gamma"];
    let active = 1;
    let bar_h = 2.0;
    let gap = 4.0;
    let vp_w = 120.0;
    let vp_h = 60.0;

    let tabbed = Layout::tabbed(kinds)
        .active(active)
        .bar_height(bar_h)
        .gap(gap)
        .resolve(vp_w, vp_h)
        .unwrap();

    let stacked = Layout::stacked(kinds)
        .active(active)
        .bar_height(bar_h)
        .gap(gap)
        .resolve(vp_w, vp_h)
        .unwrap();

    // Both have the same content panel count per kind
    for kind in &kinds {
        assert_eq!(tabbed.by_kind(kind).len(), 1);
        assert_eq!(stacked.by_kind(kind).len(), 1);
    }

    // Active panel fills remaining space in both; inactive panels are hidden
    for kind in &kinds {
        let t_id = tabbed.by_kind(kind)[0];
        let s_id = stacked.by_kind(kind)[0];
        let t_rect = tabbed.get(t_id).unwrap();
        let s_rect = stacked.get(s_id).unwrap();

        // Same full width
        assert_eq!(t_rect.w, vp_w);
        assert_eq!(s_rect.w, vp_w);

        match *kind == kinds[active] {
            true => {
                // Active panel has non-zero height in both
                assert!(t_rect.h > 0.0, "tabbed active panel height should be > 0");
                assert!(s_rect.h > 0.0, "stacked active panel height should be > 0");
            }
            false => {
                // Inactive panels hidden in both
                assert_eq!(t_rect.h, 0.0, "tabbed inactive panel should be hidden");
                assert_eq!(s_rect.h, 0.0, "stacked inactive panel should be hidden");
            }
        }
    }

    // Variant-specific: tabbed has Tab decorations, stacked has Title decorations
    let tab_decorations: Vec<_> = tabbed
        .decoration_panels()
        .iter()
        .filter(|d| d.role == panes::DecorationRole::Tab)
        .collect();
    assert_eq!(tab_decorations.len(), kinds.len());

    let title_decorations: Vec<_> = stacked
        .decoration_panels()
        .iter()
        .filter(|d| d.role == panes::DecorationRole::Title)
        .collect();
    assert_eq!(title_decorations.len(), kinds.len());

    // Both decoration heights match bar_height
    for d in &tab_decorations {
        assert_eq!(tabbed.get(d.id).unwrap().h, bar_h);
    }
    for d in &title_decorations {
        assert_eq!(stacked.get(d.id).unwrap().h, bar_h);
    }
}

#[test]
fn active_panel_presets_preserve_runtime_focus_and_sequence_behavior() {
    let kinds = ["x", "y", "z"];

    let mut tabbed_rt = Layout::tabbed(kinds).into_runtime().unwrap();
    let mut stacked_rt = Layout::stacked(kinds).into_runtime().unwrap();

    // Both runtimes produce a 3-panel sequence
    assert_eq!(tabbed_rt.sequence().len(), 3);
    assert_eq!(stacked_rt.sequence().len(), 3);

    // Initial focus: first content panel
    let _ = tabbed_rt.resolve(80.0, 24.0).unwrap();
    let _ = stacked_rt.resolve(80.0, 24.0).unwrap();

    assert_eq!(tabbed_rt.focused_kind(), Some("x"));
    assert_eq!(stacked_rt.focused_kind(), Some("x"));

    // Focus cycling: next wraps through content panels only
    tabbed_rt.focus_next();
    stacked_rt.focus_next();
    let _ = tabbed_rt.resolve(80.0, 24.0).unwrap();
    let _ = stacked_rt.resolve(80.0, 24.0).unwrap();

    assert_eq!(tabbed_rt.focused_kind(), Some("y"));
    assert_eq!(stacked_rt.focused_kind(), Some("y"));

    // Active panel: after focus_next, the newly focused panel should be visible
    let t_frame = tabbed_rt.resolve(80.0, 24.0).unwrap();
    let s_frame = stacked_rt.resolve(80.0, 24.0).unwrap();

    let t_y = t_frame.layout().by_kind("y")[0];
    let s_y = s_frame.layout().by_kind("y")[0];
    assert!(t_frame.layout().get(t_y).unwrap().h > 0.0);
    assert!(s_frame.layout().get(s_y).unwrap().h > 0.0);

    // The previously active panel should now be hidden
    let t_x = t_frame.layout().by_kind("x")[0];
    let s_x = s_frame.layout().by_kind("x")[0];
    assert_eq!(t_frame.layout().get(t_x).unwrap().h, 0.0);
    assert_eq!(s_frame.layout().get(s_x).unwrap().h, 0.0);

    // Prev wraps back
    tabbed_rt.focus_prev();
    stacked_rt.focus_prev();
    assert_eq!(tabbed_rt.focused_kind(), Some("x"));
    assert_eq!(stacked_rt.focused_kind(), Some("x"));

    // Sequence contains only content panel kinds (no decoration panels).
    // Verify by cycling through all positions and checking focused_kind.
    // Reset to first panel.
    let _ = tabbed_rt.resolve(80.0, 24.0).unwrap();
    let _ = stacked_rt.resolve(80.0, 24.0).unwrap();

    let expected_order = ["x", "y", "z"];
    for expected in &expected_order {
        assert_eq!(tabbed_rt.focused_kind(), Some(*expected));
        assert_eq!(stacked_rt.focused_kind(), Some(*expected));
        tabbed_rt.focus_next();
        stacked_rt.focus_next();
    }
    // After cycling through all 3, we're back to the first
    assert_eq!(tabbed_rt.focused_kind(), Some("x"));
    assert_eq!(stacked_rt.focused_kind(), Some("x"));
}

#[test]
fn dashboard_build_matches_shared_grid_primitive_behavior() {
    // Dashboard: 4 fixed columns, gap 8, mixed spans
    let dashboard_resolved = Layout::dashboard([
        ("a", CardSpan::Columns(1)),
        ("b", CardSpan::Columns(2)),
        ("c", CardSpan::Columns(1)),
        ("d", CardSpan::FullWidth),
    ])
    .columns(4)
    .gap(8.0)
    .auto_rows()
    .resolve(400.0, 300.0)
    .unwrap();

    // Equivalent generic grid builder
    let grid_layout = Layout::build_grid(Grid::columns(4).gap(8.0).auto_rows(), |g| {
        g.panel("a");
        g.panel_span("b", CardSpan::Columns(2));
        g.panel("c");
        g.panel_span("d", CardSpan::FullWidth);
    })
    .unwrap();
    let grid_resolved = grid_layout.resolve(400.0, 300.0).unwrap();

    // Both layouts should produce the same geometry for every panel
    for kind in &["a", "b", "c", "d"] {
        let dr = dashboard_resolved
            .get(dashboard_resolved.by_kind(kind)[0])
            .unwrap();
        let gr = grid_resolved.get(grid_resolved.by_kind(kind)[0]).unwrap();
        assert!(
            (dr.x - gr.x).abs() < 1.0
                && (dr.y - gr.y).abs() < 1.0
                && (dr.w - gr.w).abs() < 1.0
                && (dr.h - gr.h).abs() < 1.0,
            "panel '{kind}' geometry differs: dashboard={dr:?}, grid={gr:?}"
        );
    }
}
