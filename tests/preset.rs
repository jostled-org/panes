use panes::{Layout, PanelInputKind, Rect};

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
fn grid_2x2() {
    let resolved = Layout::grid(2, ["a", "b", "c", "d"])
        .resolve(100.0, 100.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];
    let c = resolved.by_kind("c")[0];
    let d = resolved.by_kind("d")[0];

    assert_eq!(
        *resolved.get(a).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 50.0,
            h: 50.0
        }
    );
    assert_eq!(
        *resolved.get(b).unwrap(),
        Rect {
            x: 50.0,
            y: 0.0,
            w: 50.0,
            h: 50.0
        }
    );
    assert_eq!(
        *resolved.get(c).unwrap(),
        Rect {
            x: 0.0,
            y: 50.0,
            w: 50.0,
            h: 50.0
        }
    );
    assert_eq!(
        *resolved.get(d).unwrap(),
        Rect {
            x: 50.0,
            y: 50.0,
            w: 50.0,
            h: 50.0
        }
    );
}

#[test]
fn grid_3x2() {
    let resolved = Layout::grid(3, ["a", "b", "c", "d", "e", "f"])
        .resolve(90.0, 100.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let d = resolved.by_kind("d")[0];

    // First row, first col
    assert_eq!(resolved.get(a).unwrap().w, 30.0);
    assert_eq!(resolved.get(a).unwrap().h, 50.0);
    // Second row, first col
    assert_eq!(resolved.get(d).unwrap().y, 50.0);
}

#[test]
fn grid_uneven() {
    // 5 panels in 3 columns: CSS Grid auto-placement, row 1 has 3, row 2 has 2
    let resolved = Layout::grid(3, ["a", "b", "c", "d", "e"])
        .resolve(90.0, 100.0)
        .unwrap();

    let d = resolved.by_kind("d")[0];
    let e = resolved.by_kind("e")[0];

    // Second row panels still exist and are in the second row
    assert_eq!(resolved.get(d).unwrap().y, 50.0);
    assert_eq!(resolved.get(e).unwrap().y, 50.0);
    // CSS Grid: each cell is 1/3 of the grid width regardless of how many panels in the row
    assert_eq!(resolved.get(d).unwrap().w, 30.0);
    assert_eq!(resolved.get(e).unwrap().w, 30.0);
}

#[test]
fn grid_with_gap() {
    let resolved = Layout::grid(2, ["a", "b", "c", "d"])
        .gap(4.0)
        .resolve(100.0, 100.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];
    let c = resolved.by_kind("c")[0];

    let a_rect = resolved.get(a).unwrap();
    let b_rect = resolved.get(b).unwrap();
    let c_rect = resolved.get(c).unwrap();

    // Horizontal gap between a and b
    assert!(b_rect.x > a_rect.x + a_rect.w);
    // Vertical gap between a and c
    assert!(c_rect.y > a_rect.y + a_rect.h);
}

#[test]
fn columns_3() {
    // 6 panels into 3 columns, CSS Grid row-major: row0=[a,b,c], row1=[d,e,f]
    let resolved = Layout::columns(3, ["a", "b", "c", "d", "e", "f"])
        .resolve(90.0, 100.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];
    let d = resolved.by_kind("d")[0];

    // a in first column, first row
    assert_eq!(resolved.get(a).unwrap().x, 0.0);
    assert_eq!(resolved.get(a).unwrap().y, 0.0);
    // d in first column, second row
    assert_eq!(resolved.get(d).unwrap().x, 0.0);
    assert_eq!(resolved.get(d).unwrap().y, 50.0);
    // b in second column
    assert_eq!(resolved.get(b).unwrap().x, 30.0);
    // Each row is half the height
    assert_eq!(resolved.get(a).unwrap().h, 50.0);
    assert_eq!(resolved.get(d).unwrap().h, 50.0);
}

#[test]
fn columns_uneven() {
    // 5 panels into 3 columns, CSS Grid row-major: row0=[a,b,c], row1=[d,e]
    let resolved = Layout::columns(3, ["a", "b", "c", "d", "e"])
        .resolve(90.0, 100.0)
        .unwrap();

    let c = resolved.by_kind("c")[0];

    // c is in the third column, first row
    assert_eq!(resolved.get(c).unwrap().x, 60.0);
    assert_eq!(resolved.get(c).unwrap().y, 0.0);
    assert_eq!(resolved.get(c).unwrap().h, 50.0);
}

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

    // Tab bar takes 1.0 height, content fills remainder
    let a_tab = resolved.by_kind("a_tab")[0];
    let a_content = resolved.by_kind("a")[0];
    let b_content = resolved.by_kind("b")[0];

    assert_eq!(resolved.get(a_tab).unwrap().h, 1.0);
    // Active content fills remaining height
    assert_eq!(resolved.get(a_content).unwrap().h, 23.0);
    // Inactive content hidden
    assert_eq!(resolved.get(b_content).unwrap().h, 0.0);
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

    let a_title = resolved.by_kind("a_title")[0];
    let b_title = resolved.by_kind("b_title")[0];
    let a_content = resolved.by_kind("a")[0];
    let b_content = resolved.by_kind("b")[0];

    // Both titles visible with height 1.0
    assert_eq!(resolved.get(a_title).unwrap().h, 1.0);
    assert_eq!(resolved.get(b_title).unwrap().h, 1.0);
    // Active content grows to fill remaining space
    assert_eq!(resolved.get(a_content).unwrap().h, 22.0);
    // Inactive content hidden
    assert_eq!(resolved.get(b_content).unwrap().h, 0.0);
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
fn presets_returns_15_entries() {
    assert_eq!(Layout::presets().len(), 15);
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

// -- Grid auto-fill / auto-fit --

#[test]
fn grid_auto_fill_resolves() {
    let resolved = Layout::grid(2, ["a", "b", "c", "d"])
        .auto_fill(200.0)
        .resolve(800.0, 600.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];
    assert_eq!(resolved.get(a).unwrap().w, 200.0);
    assert_eq!(resolved.get(b).unwrap().w, 200.0);
}

#[test]
fn grid_auto_fill_narrow_viewport() {
    let resolved = Layout::grid(2, ["a", "b", "c", "d"])
        .auto_fill(200.0)
        .resolve(300.0, 600.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    assert_eq!(resolved.get(a).unwrap().w, 300.0);
}

#[test]
fn grid_auto_fill_rejects_zero() {
    let err = Layout::grid(2, ["a"]).auto_fill(0.0).build().unwrap_err();
    assert!(
        err.to_string().contains("min_column_width"),
        "expected min_column_width error, got: {err}"
    );
}

#[test]
fn grid_auto_fit_resolves() {
    let resolved = Layout::grid(2, ["a", "b"])
        .auto_fit(200.0)
        .resolve(800.0, 600.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    assert!(resolved.get(a).unwrap().w >= 200.0);
}

// -- Columns auto-fill / auto-fit --

#[test]
fn columns_auto_fill_resolves() {
    let resolved = Layout::columns(3, ["a", "b", "c", "d", "e", "f"])
        .auto_fill(200.0)
        .resolve(900.0, 600.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    assert!(resolved.get(a).unwrap().w > 0.0);
    assert_eq!(resolved.by_kind("f").len(), 1);
}

#[test]
fn columns_auto_fit_resolves() {
    let resolved = Layout::columns(3, ["a", "b", "c"])
        .auto_fit(200.0)
        .resolve(800.0, 600.0)
        .unwrap();

    let a = resolved.by_kind("a")[0];
    assert!(resolved.get(a).unwrap().w >= 200.0);
}

#[test]
fn columns_auto_fill_rejects_zero() {
    let err = Layout::columns(3, ["a"])
        .auto_fill(0.0)
        .build()
        .unwrap_err();
    assert!(
        err.to_string().contains("min_column_width"),
        "expected min_column_width error, got: {err}"
    );
}
