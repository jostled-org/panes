use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{Direction, SlotDef, StrategyKind, fixed, grow};

fn sidebar_runtime() -> LayoutRuntime {
    let slots: Arc<[SlotDef]> = vec![
        SlotDef {
            kind: Arc::from("sidebar"),
            constraints: fixed(20.0),
        },
        SlotDef {
            kind: Arc::from("content"),
            constraints: grow(1.0),
        },
    ]
    .into();
    let kinds: Vec<Arc<str>> = vec![Arc::from("sidebar"), Arc::from("content")];
    LayoutRuntime::from_strategy(
        StrategyKind::Slotted {
            slots,
            gap: 0.0,
            direction: Direction::Horizontal,
        },
        &kinds,
    )
    .unwrap()
}

#[test]
fn slotted_move_returns_error() {
    let mut rt = sidebar_runtime();
    let p0 = rt.sequence().get(0).unwrap();
    let result = rt.move_panel(p0, 1);
    assert!(result.is_err());
}

#[test]
fn slotted_swap_returns_error() {
    let mut rt = sidebar_runtime();
    let result = rt.swap_next();
    assert!(result.is_err());
}

#[test]
fn slotted_remove_collapses() {
    let mut rt = sidebar_runtime();
    let sidebar = rt.sequence().get(0).unwrap();
    let new_focus = rt.remove_panel(sidebar).unwrap();
    assert!(new_focus.is_some());

    // Sidebar should be collapsed (fixed 0)
    let c = rt.tree().panel_constraints(sidebar).unwrap();
    assert_eq!(c.fixed, Some(0.0));
}

#[test]
fn slotted_add_uncollapses() {
    let mut rt = sidebar_runtime();
    let sidebar = rt.sequence().get(0).unwrap();

    // Collapse it first
    rt.remove_panel(sidebar).unwrap();

    // Uncollapse via add
    let restored = rt.add_panel(Arc::from("sidebar")).unwrap();
    assert_eq!(restored, sidebar);

    // Should have original constraints back
    let c = rt.tree().panel_constraints(sidebar).unwrap();
    assert_eq!(c.fixed, Some(20.0));
}

#[test]
fn holy_grail_layout_has_nested_row() {
    let rt = panes::Layout::holy_grail("header", "footer", "left", "main", "right")
        .header_height(3.0)
        .footer_height(3.0)
        .sidebar_width(15.0)
        .gap(1.0)
        .into_runtime()
        .unwrap();

    // Resolve at a known size
    let mut rt = rt;
    let frame = rt.resolve(100.0, 50.0).unwrap();
    let layout = frame.layout();

    // left, main, right should be side-by-side (same y, different x)
    let left = layout.by_kind("left")[0];
    let main = layout.by_kind("main")[0];
    let right = layout.by_kind("right")[0];
    let left_r = layout.get(left).unwrap();
    let main_r = layout.get(main).unwrap();
    let right_r = layout.get(right).unwrap();

    // All three in the middle row should share the same y
    assert_eq!(left_r.y, main_r.y);
    assert_eq!(main_r.y, right_r.y);

    // left should be to the left of main, main to the left of right
    assert!(left_r.x < main_r.x);
    assert!(main_r.x < right_r.x);

    // header should be above them, footer below
    let header = layout.by_kind("header")[0];
    let footer = layout.by_kind("footer")[0];
    let header_r = layout.get(header).unwrap();
    let footer_r = layout.get(footer).unwrap();
    assert!(header_r.y < left_r.y);
    assert!(footer_r.y > left_r.y);
}

#[test]
fn holy_grail_focus_cycles_all_panels() {
    let mut rt = panes::Layout::holy_grail("header", "footer", "left", "main", "right")
        .header_height(3.0)
        .footer_height(3.0)
        .sidebar_width(15.0)
        .gap(1.0)
        .into_runtime()
        .unwrap();

    assert_eq!(rt.sequence().len(), 5);

    // Cycle through all 5 panels
    let mut seen = std::collections::HashSet::new();
    for _ in 0..5 {
        seen.insert(rt.focused().unwrap());
        rt.focus_next();
    }
    assert_eq!(seen.len(), 5);
}
