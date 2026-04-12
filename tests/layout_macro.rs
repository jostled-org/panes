#![allow(clippy::unwrap_used, clippy::panic)]
use panes::{Align, LayoutBuilder, Rect, fixed, grow};

// -- Step 1: Basic row/col/panel macro with grow and fixed --

#[test]
fn single_panel_row() {
    let layout = panes::layout! {
        row {
            panel("editor")
        }
    }
    .unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();

    let ids = resolved.by_kind("editor");
    assert_eq!(ids.len(), 1);
    assert_eq!(
        *resolved.get(ids[0]).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 80.0,
            h: 24.0
        }
    );
}

#[test]
fn two_panels_equal_grow() {
    let layout = panes::layout! {
        row {
            panel("left")
            panel("right")
        }
    }
    .unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();

    let left = resolved.by_kind("left")[0];
    let right = resolved.by_kind("right")[0];
    assert_eq!(resolved.get(left).unwrap().w, 40.0);
    assert_eq!(resolved.get(right).unwrap().w, 40.0);
    assert_eq!(resolved.get(right).unwrap().x, 40.0);
}

#[test]
fn grow_with_explicit_factor() {
    let layout = panes::layout! {
        row {
            panel("a", grow: 2.0)
            panel("b", grow: 1.0)
        }
    }
    .unwrap();
    let resolved = layout.resolve(90.0, 30.0).unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];
    assert_eq!(resolved.get(a).unwrap().w, 60.0);
    assert_eq!(resolved.get(b).unwrap().w, 30.0);
}

#[test]
fn fixed_panel() {
    let layout = panes::layout! {
        row {
            panel("side", fixed: 20.0)
            panel("main")
        }
    }
    .unwrap();
    let resolved = layout.resolve(100.0, 40.0).unwrap();

    let side = resolved.by_kind("side")[0];
    let main = resolved.by_kind("main")[0];
    assert_eq!(resolved.get(side).unwrap().w, 20.0);
    assert_eq!(resolved.get(main).unwrap().w, 80.0);
}

#[test]
fn nested_col_in_row() {
    let layout = panes::layout! {
        row {
            panel("editor", grow: 2.0)
            col {
                panel("chat")
                panel("status", fixed: 3.0)
            }
        }
    }
    .unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();

    assert_eq!(resolved.iter().count(), 3);
    let status = resolved.by_kind("status")[0];
    assert_eq!(resolved.get(status).unwrap().h, 3.0);
}

#[test]
fn deeply_nested() {
    let layout = panes::layout! {
        row {
            col {
                panel("a")
                row {
                    panel("b")
                    panel("c")
                }
            }
            panel("d")
        }
    }
    .unwrap();
    let resolved = layout.resolve(100.0, 100.0).unwrap();

    assert_eq!(resolved.iter().count(), 4);
    for (_, rect) in resolved.iter() {
        assert!(rect.w > 0.0, "panel has zero width: {rect:?}");
        assert!(rect.h > 0.0, "panel has zero height: {rect:?}");
    }
}

#[test]
fn equivalence_with_builder_api() {
    // Build with macro
    let macro_layout = panes::layout! {
        row {
            panel("editor", grow: 2.0)
            col {
                panel("chat", grow: 1.0)
                panel("status", fixed: 3.0)
            }
        }
    }
    .unwrap();
    let macro_resolved = macro_layout.resolve(120.0, 40.0).unwrap();

    // Build with builder API
    let mut b = LayoutBuilder::new();
    b.row(|r| {
        r.panel_with("editor", grow(2.0));
        r.col(|c| {
            c.panel("chat");
            c.panel_with("status", fixed(3.0));
        });
    })
    .unwrap();
    let builder_layout = b.build().unwrap();
    let builder_resolved = builder_layout.resolve(120.0, 40.0).unwrap();

    // Compare rects by kind
    for kind in ["editor", "chat", "status"] {
        let m = macro_resolved.by_kind(kind)[0];
        let b = builder_resolved.by_kind(kind)[0];
        assert_eq!(
            *macro_resolved.get(m).unwrap(),
            *builder_resolved.get(b).unwrap(),
            "mismatch for panel kind '{kind}'"
        );
    }
}

// -- Step 2: Gap parameter and min/max constraints --

#[test]
fn row_with_gap() {
    let layout = panes::layout! {
        row(gap: 8.0) {
            panel("a")
            panel("b")
        }
    }
    .unwrap();
    let resolved = layout.resolve(100.0, 40.0).unwrap();

    let a = resolved.by_kind("a")[0];
    let b = resolved.by_kind("b")[0];
    let a_rect = resolved.get(a).unwrap();
    let b_rect = resolved.get(b).unwrap();

    // a.w + gap(8) + b.w == 100, equal grow => a.w == 46, b.w == 46
    assert_eq!(a_rect.w, 46.0);
    assert_eq!(b_rect.w, 46.0);
    // b starts after a + gap
    assert_eq!(b_rect.x, 54.0);
}

#[test]
fn col_with_gap() {
    let layout = panes::layout! {
        col(gap: 4.0) {
            panel("top")
            panel("bot")
        }
    }
    .unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();

    let top = resolved.by_kind("top")[0];
    let bot = resolved.by_kind("bot")[0];
    let top_rect = resolved.get(top).unwrap();
    let bot_rect = resolved.get(bot).unwrap();

    // top.h + gap(4) + bot.h == 24 => top.h == 10, bot.h == 10
    assert_eq!(top_rect.h, 10.0);
    assert_eq!(bot_rect.h, 10.0);
    assert_eq!(bot_rect.y, 14.0);
}

#[test]
fn panel_with_min_max() {
    let layout = panes::layout! {
        row {
            panel("a", grow: 1.0, min: 20.0, max: 50.0)
            panel("b")
        }
    }
    .unwrap();
    let resolved = layout.resolve(200.0, 40.0).unwrap();

    let a = resolved.by_kind("a")[0];
    let a_rect = resolved.get(a).unwrap();
    assert!(a_rect.w <= 50.0, "expected a.w <= 50, got {}", a_rect.w);
}

#[test]
fn fixed_with_min() {
    let layout = panes::layout! {
        row {
            panel("a", fixed: 30.0, min: 10.0)
            panel("b")
        }
    }
    .unwrap();
    let resolved = layout.resolve(100.0, 40.0).unwrap();

    let a = resolved.by_kind("a")[0];
    assert_eq!(resolved.get(a).unwrap().w, 30.0);
}

#[test]
fn full_spec_example() {
    let layout = panes::layout! {
        row(gap: 8.0) {
            panel("editor", grow: 2.0)
            col {
                panel("chat", grow: 1.0)
                panel("status", fixed: 3.0)
            }
        }
    }
    .unwrap();
    let resolved = layout.resolve(120.0, 40.0).unwrap();

    assert_eq!(resolved.iter().count(), 3);

    let editor = resolved.by_kind("editor")[0];
    let chat = resolved.by_kind("chat")[0];
    let status = resolved.by_kind("status")[0];

    let editor_rect = resolved.get(editor).unwrap();
    let chat_rect = resolved.get(chat).unwrap();
    let status_rect = resolved.get(status).unwrap();

    // editor gets grow:2, col gets grow:1 (default), gap:8 between them
    // total grow space = 120 - 8 = 112, editor ≈ 2/3 of 112
    // Flexbox rounding may shift by ±1px
    assert!(
        (editor_rect.w - 112.0 * 2.0 / 3.0).abs() <= 1.0,
        "editor.w = {}, expected ≈ {}",
        editor_rect.w,
        112.0 * 2.0 / 3.0
    );
    assert_eq!(status_rect.h, 3.0);
    assert_eq!(chat_rect.h, 37.0);
}

#[test]
fn equivalence_gap_with_builder() {
    // Build with macro
    let macro_layout = panes::layout! {
        row(gap: 8.0) {
            panel("left", grow: 2.0)
            panel("right", grow: 1.0)
        }
    }
    .unwrap();
    let macro_resolved = macro_layout.resolve(100.0, 40.0).unwrap();

    // Build with builder API
    let mut b = LayoutBuilder::new();
    b.row_gap(8.0, |r| {
        r.panel_with("left", grow(2.0));
        r.panel("right");
    })
    .unwrap();
    let builder_layout = b.build().unwrap();
    let builder_resolved = builder_layout.resolve(100.0, 40.0).unwrap();

    for kind in ["left", "right"] {
        let m = macro_resolved.by_kind(kind)[0];
        let b = builder_resolved.by_kind(kind)[0];
        assert_eq!(
            *macro_resolved.get(m).unwrap(),
            *builder_resolved.get(b).unwrap(),
            "mismatch for panel kind '{kind}'"
        );
    }
}

// -- Step 6: Cross-axis constraints and alignment in macro --

#[test]
fn macro_cross_axis_constraints() {
    let layout = panes::layout! {
        col {
            panel("a", grow: 1.0, max_height: 100.0)
            panel("b", grow: 1.0)
        }
    }
    .unwrap();
    let resolved = layout.resolve(400.0, 400.0).unwrap();

    let a = resolved.by_kind("a")[0];
    let a_rect = resolved.get(a).unwrap();
    assert!(a_rect.h <= 100.0, "expected a.h <= 100, got {}", a_rect.h);

    let b = resolved.by_kind("b")[0];
    let b_rect = resolved.get(b).unwrap();
    assert!(b_rect.h >= 300.0, "expected b.h >= 300, got {}", b_rect.h);
}

#[test]
fn macro_align() {
    let layout = panes::layout! {
        row {
            panel("a", fixed: 50.0, align: center)
        }
    }
    .unwrap();
    let resolved = layout.resolve(400.0, 400.0).unwrap();

    let a = resolved.by_kind("a")[0];
    let a_rect = resolved.get(a).unwrap();
    assert_eq!(a_rect.w, 50.0, "fixed width should be 50");
    assert!(
        a_rect.h < 400.0,
        "aligned panel should not stretch to full height"
    );
}

#[test]
fn macro_multiple_cross_axis_fields() {
    let layout = panes::layout! {
        row {
            panel("a", grow: 1.0, min_width: 50.0, max_height: 200.0)
        }
    }
    .unwrap();
    let resolved = layout.resolve(400.0, 400.0).unwrap();

    let a = resolved.by_kind("a")[0];
    let a_rect = resolved.get(a).unwrap();
    assert!(a_rect.w >= 50.0);
    assert!(a_rect.h <= 200.0);
}

#[test]
fn macro_equivalence_cross_axis_with_builder() {
    let macro_layout = panes::layout! {
        col {
            panel("a", grow: 1.0, max_height: 100.0)
            panel("b", grow: 1.0)
        }
    }
    .unwrap();

    let mut b = LayoutBuilder::new();
    b.col(|c| {
        c.panel_with("a", grow(1.0).max_height(100.0));
        c.panel("b");
    })
    .unwrap();
    let builder_layout = b.build().unwrap();

    for kind in ["a", "b"] {
        let mr = macro_layout.resolve(400.0, 400.0).unwrap();
        let br = builder_layout.resolve(400.0, 400.0).unwrap();
        let m = mr.by_kind(kind)[0];
        let bi = br.by_kind(kind)[0];
        assert_eq!(
            *mr.get(m).unwrap(),
            *br.get(bi).unwrap(),
            "mismatch for panel kind '{kind}'"
        );
    }
}

#[test]
fn macro_size_mode() {
    let layout = panes::layout! {
        row {
            panel("a", grow: 1.0, size_mode: min_content)
        }
    }
    .unwrap();
    let resolved = layout.resolve(400.0, 400.0).unwrap();

    let a = resolved.by_kind("a")[0];
    let a_rect = resolved.get(a).unwrap();
    assert!(a_rect.w > 0.0, "panel should resolve with size_mode");
}

#[test]
fn macro_size_mode_fit_content() {
    let layout = panes::layout! {
        row {
            panel("a", grow: 1.0, size_mode: fit_content(300.0))
        }
    }
    .unwrap();
    let resolved = layout.resolve(400.0, 400.0).unwrap();

    let a = resolved.by_kind("a")[0];
    let a_rect = resolved.get(a).unwrap();
    assert!(
        a_rect.w > 0.0,
        "panel should resolve with fit_content size_mode"
    );
}

#[test]
fn macro_size_mode_max_content() {
    let layout = panes::layout! {
        row {
            panel("a", grow: 1.0, size_mode: max_content)
        }
    }
    .unwrap();
    let resolved = layout.resolve(400.0, 400.0).unwrap();

    let a = resolved.by_kind("a")[0];
    let a_rect = resolved.get(a).unwrap();
    assert!(
        a_rect.w > 0.0,
        "panel should resolve with max_content size_mode"
    );
}

#[test]
fn macro_align_equivalence_with_builder() {
    let macro_layout = panes::layout! {
        row {
            panel("a", fixed: 50.0, align: center)
        }
    }
    .unwrap();

    let mut b = LayoutBuilder::new();
    b.row(|r| {
        r.panel_with("a", fixed(50.0).align(Align::Center));
    })
    .unwrap();
    let builder_layout = b.build().unwrap();

    let mr = macro_layout.resolve(400.0, 400.0).unwrap();
    let br = builder_layout.resolve(400.0, 400.0).unwrap();
    let m = mr.by_kind("a")[0];
    let bk = br.by_kind("a")[0];
    assert_eq!(*mr.get(m).unwrap(), *br.get(bk).unwrap());
}
