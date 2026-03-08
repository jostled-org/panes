use panes::compiler::{compile, compute_layout};
use panes::resolver::resolve;
use panes::{Layout, LayoutBuilder, LayoutTree, Rect, fixed, grow};

// -- Step 1: Builder end-to-end pipeline --

#[test]
fn builder_two_equal_panels_row() {
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

    assert_eq!(
        *resolved.get(left).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 40.0,
            h: 24.0
        }
    );
    assert_eq!(
        *resolved.get(right).unwrap(),
        Rect {
            x: 40.0,
            y: 0.0,
            w: 40.0,
            h: 24.0
        }
    );
}

#[test]
fn builder_grow_ratio() {
    let mut b = LayoutBuilder::new();
    let a = b.panel_with("a", grow(2.0)).unwrap();
    let bb = b.panel("b").unwrap();
    b.row(|r| {
        r.add(a);
        r.add(bb);
    })
    .unwrap();
    let layout = b.build().unwrap();
    let resolved = layout.resolve(90.0, 30.0).unwrap();

    assert_eq!(resolved.get(a).unwrap().w, 60.0);
    assert_eq!(resolved.get(bb).unwrap().w, 30.0);
    assert_eq!(resolved.get(bb).unwrap().x, 60.0);
}

#[test]
fn builder_fixed_plus_grow() {
    let mut b = LayoutBuilder::new();
    let side = b.panel_with("side", fixed(20.0)).unwrap();
    let main = b.panel("main").unwrap();
    b.row(|r| {
        r.add(side);
        r.add(main);
    })
    .unwrap();
    let layout = b.build().unwrap();
    let resolved = layout.resolve(100.0, 50.0).unwrap();

    assert_eq!(resolved.get(side).unwrap().w, 20.0);
    assert_eq!(resolved.get(main).unwrap().w, 80.0);
}

#[test]
fn builder_gap_reduces_space() {
    let mut b = LayoutBuilder::new();
    let a = b.panel("a").unwrap();
    let bb = b.panel("b").unwrap();
    b.row_gap(10.0, |r| {
        r.add(a);
        r.add(bb);
    })
    .unwrap();
    let layout = b.build().unwrap();
    let resolved = layout.resolve(100.0, 50.0).unwrap();

    assert_eq!(resolved.get(a).unwrap().w, 45.0);
    assert_eq!(resolved.get(bb).unwrap().w, 45.0);
}

// -- Step 2: Nested closures and inline panels --

#[test]
fn builder_nested_row_col() {
    let mut b = LayoutBuilder::new();
    let editor = b.panel_with("editor", grow(2.0)).unwrap();
    let chat = b.panel("chat").unwrap();
    let status = b.panel("status").unwrap();
    b.row(|r| {
        r.add(editor);
        r.col(|c| {
            c.add(chat);
            c.add(status);
        });
    })
    .unwrap();
    let layout = b.build().unwrap();
    let resolved = layout.resolve(90.0, 24.0).unwrap();

    assert_eq!(
        *resolved.get(editor).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 60.0,
            h: 24.0
        }
    );
    assert_eq!(
        *resolved.get(chat).unwrap(),
        Rect {
            x: 60.0,
            y: 0.0,
            w: 30.0,
            h: 12.0
        }
    );
    assert_eq!(
        *resolved.get(status).unwrap(),
        Rect {
            x: 60.0,
            y: 12.0,
            w: 30.0,
            h: 12.0
        }
    );
}

#[test]
fn builder_inline_panel_creation() {
    let mut b = LayoutBuilder::new();
    b.row(|r| {
        r.panel("a");
        r.panel("b");
    })
    .unwrap();
    let layout = b.build().unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();

    let a_ids = resolved.by_kind("a");
    let b_ids = resolved.by_kind("b");
    assert_eq!(a_ids.len(), 1);
    assert_eq!(b_ids.len(), 1);
    assert_eq!(resolved.get(a_ids[0]).unwrap().w, 40.0);
    assert_eq!(resolved.get(b_ids[0]).unwrap().w, 40.0);
}

#[test]
fn builder_mixed_pre_created_and_inline() {
    let mut b = LayoutBuilder::new();
    let editor = b.panel_with("editor", grow(2.0)).unwrap();
    b.row(|r| {
        r.add(editor);
        r.col(|c| {
            c.panel("chat");
            c.panel("status");
        });
    })
    .unwrap();
    let layout = b.build().unwrap();
    let resolved = layout.resolve(90.0, 24.0).unwrap();

    assert_eq!(
        *resolved.get(editor).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 60.0,
            h: 24.0
        }
    );
    let chat = resolved.by_kind("chat")[0];
    let status = resolved.by_kind("status")[0];
    assert_eq!(
        *resolved.get(chat).unwrap(),
        Rect {
            x: 60.0,
            y: 0.0,
            w: 30.0,
            h: 12.0
        }
    );
    assert_eq!(
        *resolved.get(status).unwrap(),
        Rect {
            x: 60.0,
            y: 12.0,
            w: 30.0,
            h: 12.0
        }
    );
}

#[test]
fn builder_deeply_nested() {
    let mut b = LayoutBuilder::new();
    let deep = b.panel("deep").unwrap();
    b.row(|r| {
        r.col(|c| {
            c.row(|r2| {
                r2.add(deep);
            });
        });
    })
    .unwrap();
    let layout = b.build().unwrap();
    let resolved = layout.resolve(100.0, 100.0).unwrap();

    assert_eq!(
        *resolved.get(deep).unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0
        }
    );
}

// -- Step 3: Convenience constructors and mutable bridge --

#[test]
fn layout_row_convenience() {
    let layout = Layout::row(["left", "right"]).unwrap();
    let resolved = layout.resolve(80.0, 24.0).unwrap();

    let left = resolved.by_kind("left")[0];
    let right = resolved.by_kind("right")[0];
    assert_eq!(resolved.get(left).unwrap().w, 40.0);
    assert_eq!(resolved.get(right).unwrap().w, 40.0);
}

#[test]
fn layout_col_convenience() {
    let layout = Layout::col(["top", "mid", "bot"]).unwrap();
    let resolved = layout.resolve(60.0, 90.0).unwrap();

    let top = resolved.by_kind("top")[0];
    let mid = resolved.by_kind("mid")[0];
    let bot = resolved.by_kind("bot")[0];
    assert_eq!(resolved.get(top).unwrap().h, 30.0);
    assert_eq!(resolved.get(mid).unwrap().h, 30.0);
    assert_eq!(resolved.get(bot).unwrap().h, 30.0);
    assert_eq!(resolved.get(top).unwrap().y, 0.0);
    assert_eq!(resolved.get(mid).unwrap().y, 30.0);
    assert_eq!(resolved.get(bot).unwrap().y, 60.0);
}

#[test]
fn layout_to_tree_and_resolve() {
    let layout = Layout::row(["a", "b"]).unwrap();
    let tree: LayoutTree = layout.into();
    let resolved = tree.resolve(80.0, 24.0).unwrap();

    let a = resolved.by_kind("a")[0];
    let b_id = resolved.by_kind("b")[0];
    assert_eq!(resolved.get(a).unwrap().w, 40.0);
    assert_eq!(resolved.get(b_id).unwrap().w, 40.0);
}

#[test]
fn tree_resolve_shorthand() {
    let mut tree = LayoutTree::new();
    let (p1, n1) = tree.add_panel("a", grow(1.0)).unwrap();
    let (p2, n2) = tree.add_panel("b", grow(1.0)).unwrap();
    let root = tree.add_row(0.0, vec![n1, n2]).unwrap();
    tree.set_root(root);

    // Shorthand
    let resolved_short = tree.resolve(80.0, 24.0).unwrap();
    // Manual pipeline
    let mut result = compile(&tree).unwrap();
    compute_layout(&mut result, 80.0, 24.0).unwrap();
    let resolved_manual = resolve(&result, &tree).unwrap();

    assert_eq!(
        resolved_short.get(p1).unwrap(),
        resolved_manual.get(p1).unwrap()
    );
    assert_eq!(
        resolved_short.get(p2).unwrap(),
        resolved_manual.get(p2).unwrap()
    );
}

// -- Step 4: Taffy escape hatch and error edge cases --

#[test]
fn builder_taffy_escape_hatch() {
    let mut b = LayoutBuilder::new();
    let a = b.panel("a").unwrap();
    let b_panel = b.panel("b").unwrap();
    b.row(|r| {
        r.add(a);
        r.taffy_node(
            taffy::Style {
                flex_grow: 2.0,
                flex_basis: taffy::Dimension::length(0.0),
                flex_shrink: 1.0,
                ..Default::default()
            },
            |inner| {
                inner.add(b_panel);
            },
        );
    })
    .unwrap();
    let layout = b.build().unwrap();
    let resolved = layout.resolve(90.0, 30.0).unwrap();

    // a gets grow=1, taffy node gets grow=2 → a=30, taffy=60
    assert_eq!(resolved.get(a).unwrap().w, 30.0);
    assert_eq!(resolved.get(b_panel).unwrap().w, 60.0);
}

#[test]
fn builder_no_root_errors() {
    let mut b = LayoutBuilder::new();
    b.panel("orphan").unwrap();
    let err = b.build().unwrap_err();
    assert!(matches!(err, panes::PaneError::InvalidTree(_)));
}

#[test]
fn builder_reject_nan_gap() {
    let mut b = LayoutBuilder::new();
    let err = b.row_gap(f32::NAN, |_| {}).unwrap_err();
    assert!(matches!(err, panes::PaneError::InvalidConstraint(_)));
}

#[test]
fn builder_reject_negative_gap() {
    let mut b = LayoutBuilder::new();
    let err = b.row_gap(-1.0, |_| {}).unwrap_err();
    assert!(matches!(err, panes::PaneError::InvalidConstraint(_)));
}

#[test]
fn builder_reject_infinite_gap() {
    let mut b = LayoutBuilder::new();
    let err = b.row_gap(f32::INFINITY, |_| {}).unwrap_err();
    assert!(matches!(err, panes::PaneError::InvalidConstraint(_)));
}

#[test]
fn builder_closure_error_propagation() {
    let mut b = LayoutBuilder::new();
    let err = b
        .row(|r| {
            r.panel_with("bad", grow(-1.0));
        })
        .unwrap_err();
    assert!(matches!(err, panes::PaneError::InvalidConstraint(_)));
}

#[test]
fn builder_duplicate_root_errors() {
    let mut b = LayoutBuilder::new();
    b.row(|r| {
        r.panel("a");
    })
    .unwrap();
    let err = b
        .row(|r| {
            r.panel("b");
        })
        .unwrap_err();
    assert!(matches!(err, panes::PaneError::InvalidTree(_)));
}
