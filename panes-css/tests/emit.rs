#![allow(clippy::unwrap_used, clippy::panic)]
use panes::{Align, CardSpan, Layout, LayoutBuilder, Overlay, SizeMode, fixed, grow};

/// Build a scrollable-pattern layout: a root row containing a TaffyPassthrough
/// with row direction, nowrap, and fixed-width panel children.
fn scrollable_layout(kinds: &[&str], width: f32) -> Layout {
    let mut b = LayoutBuilder::new();
    b.row(|r| {
        let style = taffy::Style {
            display: taffy::Display::Flex,
            flex_direction: taffy::FlexDirection::Row,
            flex_wrap: taffy::FlexWrap::NoWrap,
            ..Default::default()
        };
        r.taffy_node(style, |c| {
            for &kind in kinds {
                c.panel_with(kind, fixed(width));
            }
        });
    })
    .unwrap();
    b.build().unwrap()
}

#[test]
fn simple_row_emits_flex_row() {
    let layout = Layout::row(["left", "right"]).unwrap();
    let css = panes_css::emit(&layout);

    assert!(css.contains("[data-pane-root]"), "missing root selector");
    assert!(css.contains("display: flex"), "missing display: flex");
    assert!(
        css.contains("flex-direction: row"),
        "missing flex-direction: row"
    );
    assert!(css.contains(r#"[data-pane="left"]"#), "missing left panel");
    assert!(
        css.contains(r#"[data-pane="right"]"#),
        "missing right panel"
    );
    assert!(css.contains("flex-grow: 1"), "missing flex-grow");
}

#[test]
fn col_with_gap_emits_gap() {
    let mut b = LayoutBuilder::new();
    b.col_gap(4.0, |c| {
        c.panel("top");
        c.panel("bot");
    })
    .unwrap();
    let layout = b.build().unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("flex-direction: column"),
        "missing column direction"
    );
    assert!(css.contains("gap: 4px"), "missing gap: 4px");
}

#[test]
fn fixed_panel_emits_flex_basis() {
    let mut b = LayoutBuilder::new();
    b.row(|r| {
        r.panel_with("sidebar", fixed(20.0));
        r.panel("content");
    })
    .unwrap();
    let layout = b.build().unwrap();
    let css = panes_css::emit(&layout);

    assert!(css.contains("flex-basis: 20px"), "missing flex-basis: 20px");
    assert!(
        css.contains("flex-shrink: 0"),
        "missing flex-shrink: 0 for fixed panel"
    );
    assert!(
        css.contains("flex-grow: 0"),
        "missing flex-grow: 0 for fixed panel"
    );
    assert!(
        css.contains(r#"[data-pane="content"]"#),
        "missing content panel"
    );
}

#[test]
fn grow_panel_emits_flex_grow() {
    let mut b = LayoutBuilder::new();
    b.row(|r| {
        r.panel_with("main", grow(2.0));
        r.panel("side");
    })
    .unwrap();
    let layout = b.build().unwrap();
    let css = panes_css::emit(&layout);

    assert!(css.contains("flex-grow: 2"), "missing flex-grow: 2");
    assert!(css.contains(r#"[data-pane="main"]"#), "missing main panel");
    assert!(css.contains(r#"[data-pane="side"]"#), "missing side panel");
}

#[test]
fn nested_layout_emits_container_selectors() {
    let mut b = LayoutBuilder::new();
    b.row(|r| {
        r.panel("left");
        r.col(|c| {
            c.panel("top");
            c.panel("bot");
        });
    })
    .unwrap();
    let layout = b.build().unwrap();
    let css = panes_css::emit(&layout);

    // Root is [data-pane-root] with flex-direction: row
    assert!(css.contains("[data-pane-root]"), "missing root selector");
    assert!(css.contains("flex-direction: row"), "missing row direction");
    // Nested col is [data-pane-node="N"] with flex-direction: column
    assert!(
        css.contains(r#"[data-pane-node="1"]"#),
        "missing node selector"
    );
    assert!(
        css.contains("flex-direction: column"),
        "missing column direction"
    );
    // All 3 panels present
    assert!(css.contains(r#"[data-pane="left"]"#), "missing left panel");
    assert!(css.contains(r#"[data-pane="top"]"#), "missing top panel");
    assert!(css.contains(r#"[data-pane="bot"]"#), "missing bot panel");
}

#[test]
fn min_max_constraints_emit_correctly() {
    let mut b = LayoutBuilder::new();
    b.row(|r| {
        r.panel_with("panel", grow(1.0).min(20.0).max(80.0));
    })
    .unwrap();
    let layout = b.build().unwrap();
    let css = panes_css::emit(&layout);

    assert!(css.contains("min-width: 20px"), "missing min-width: 20px");
    assert!(css.contains("max-width: 80px"), "missing max-width: 80px");
}

#[test]
fn col_min_max_uses_height() {
    let mut b = LayoutBuilder::new();
    b.col(|c| {
        c.panel_with("panel", grow(1.0).min(10.0).max(50.0));
    })
    .unwrap();
    let layout = b.build().unwrap();
    let css = panes_css::emit(&layout);

    assert!(css.contains("min-height: 10px"), "missing min-height: 10px");
    assert!(css.contains("max-height: 50px"), "missing max-height: 50px");
}

#[test]
fn dashboard_emits_css_grid() {
    let layout = Layout::dashboard([("chart", 2), ("stats", 1), ("logs", 1)])
        .columns(3)
        .gap(8.0)
        .build()
        .unwrap();
    let css = panes_css::emit(&layout);

    assert!(css.contains("display: grid"), "missing display: grid");
    assert!(
        css.contains("grid-template-columns: repeat(3, 1fr)"),
        "missing grid-template-columns"
    );
    assert!(css.contains("gap: 8px"), "missing gap: 8px");
    assert!(
        css.contains("grid-column: span 2"),
        "missing grid-column: span 2 for chart"
    );
    assert!(
        css.contains(r#"[data-pane="chart"]"#),
        "missing chart panel"
    );
    assert!(
        css.contains(r#"[data-pane="stats"]"#),
        "missing stats panel"
    );
}

#[test]
fn dashboard_auto_fill_css() {
    let layout = Layout::dashboard([("a", 1), ("b", 1)])
        .auto_fill(300.0)
        .build()
        .unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("repeat(auto-fill, minmax(300px, 1fr))"),
        "missing auto-fill grid-template-columns, got: {css}"
    );
}

#[test]
fn dashboard_auto_fit_css() {
    let layout = Layout::dashboard([("a", 1)])
        .auto_fit(250.0)
        .build()
        .unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("repeat(auto-fit, minmax(250px, 1fr))"),
        "missing auto-fit grid-template-columns, got: {css}"
    );
}

#[test]
fn dashboard_fixed_css_unchanged() {
    let layout = Layout::dashboard([("a", 1), ("b", 1)])
        .columns(3)
        .build()
        .unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("repeat(3, 1fr)"),
        "fixed columns should still use repeat(N, 1fr), got: {css}"
    );
}

#[test]
fn adaptive_media_queries() {
    let narrow = Layout::stacked(["a", "b"]).build().unwrap();
    let wide = Layout::master_stack(["a", "b"]).build().unwrap();

    let css = panes_css::emit_adaptive(&[(0, &narrow), (600, &wide)]).unwrap();

    assert!(
        css.contains("@media (max-width: 599px)"),
        "missing narrow query, got: {css}"
    );
    assert!(
        css.contains("@media (min-width: 600px)"),
        "missing wide query, got: {css}"
    );
    assert!(css.contains("[data-pane=\"a\"]"), "missing panel a");
}

#[test]
fn adaptive_three_breakpoints() {
    let small = Layout::stacked(["a", "b"]).build().unwrap();
    let medium = Layout::row(["a", "b"]).unwrap();
    let large = Layout::master_stack(["a", "b"]).build().unwrap();

    let css = panes_css::emit_adaptive(&[(0, &small), (600, &medium), (1200, &large)]).unwrap();

    assert!(
        css.contains("@media (max-width: 599px)"),
        "missing small query"
    );
    assert!(
        css.contains("@media (min-width: 600px) and (max-width: 1199px)"),
        "missing medium query, got: {css}"
    );
    assert!(
        css.contains("@media (min-width: 1200px)"),
        "missing large query"
    );
}

#[test]
fn adaptive_single_breakpoint_no_media_query() {
    let layout = Layout::row(["a", "b"]).unwrap();
    let css = panes_css::emit_adaptive(&[(0, &layout)]).unwrap();

    assert!(
        !css.contains("@media"),
        "single breakpoint should not wrap in @media, got: {css}"
    );
    assert!(css.contains("[data-pane=\"a\"]"), "missing panel a");
}

#[test]
fn adaptive_unsorted_breakpoints_are_rejected() {
    let narrow = Layout::stacked(["a", "b"]).build().unwrap();
    let wide = Layout::master_stack(["a", "b"]).build().unwrap();

    let error = panes_css::emit_adaptive(&[(600, &wide), (0, &narrow)]).unwrap_err();

    assert_eq!(
        error,
        panes_css::AdaptiveCssError::UnsortedOrDuplicate {
            index: 1,
            previous_width: 600,
            width: 0,
        }
    );
}

#[test]
fn adaptive_duplicate_breakpoints_are_rejected() {
    let narrow = Layout::stacked(["a", "b"]).build().unwrap();
    let wide = Layout::master_stack(["a", "b"]).build().unwrap();

    let error = panes_css::emit_adaptive(&[(0, &narrow), (0, &wide)]).unwrap_err();

    assert_eq!(
        error,
        panes_css::AdaptiveCssError::UnsortedOrDuplicate {
            index: 1,
            previous_width: 0,
            width: 0,
        }
    );
}

#[test]
fn dashboard_auto_fill_css_via_emit() {
    let layout = Layout::dashboard([("a", 1), ("b", 1), ("c", 1), ("d", 1)])
        .auto_fill(250.0)
        .build()
        .unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("repeat(auto-fill, minmax(250px, 1fr))"),
        "missing auto-fill grid-template-columns, got: {css}"
    );
}

#[test]
fn dashboard_auto_fit_css_via_emit() {
    let layout = Layout::dashboard([("a", 1), ("b", 1)])
        .auto_fit(300.0)
        .build()
        .unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("repeat(auto-fit, minmax(300px, 1fr))"),
        "missing auto-fit grid-template-columns, got: {css}"
    );
}

#[test]
fn dashboard_fixed_css_emits_grid_via_emit() {
    let layout = Layout::dashboard([("a", 1), ("b", 1), ("c", 1)])
        .columns(3)
        .build()
        .unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("display: grid"),
        "dashboard should use CSS Grid, got: {css}"
    );
    assert!(
        css.contains("repeat(3, 1fr)"),
        "fixed dashboard should use repeat(N, 1fr), got: {css}"
    );
}

#[test]
fn dashboard_full_width_card_emits_1_to_neg1() {
    let layout = Layout::dashboard([
        ("sidebar", CardSpan::Columns(1)),
        ("content", CardSpan::FullWidth),
    ])
    .auto_fill(200.0)
    .build()
    .unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("grid-column: 1 / -1"),
        "full-width card should emit grid-column: 1 / -1, got: {css}"
    );
}

#[test]
fn dashboard_full_width_with_fixed_columns() {
    let layout = Layout::dashboard([
        ("narrow", CardSpan::Columns(1)),
        ("wide", CardSpan::FullWidth),
    ])
    .columns(4)
    .build()
    .unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("grid-column: 1 / -1"),
        "full-width card should emit 1 / -1 even with fixed columns, got: {css}"
    );
}

#[test]
fn dashboard_span_and_full_width_coexist() {
    let layout = Layout::dashboard([
        ("a", CardSpan::Columns(2)),
        ("b", CardSpan::FullWidth),
        ("c", CardSpan::Columns(1)),
    ])
    .columns(4)
    .gap(4.0)
    .build()
    .unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("grid-column: span 2"),
        "span-2 card should use span 2, got: {css}"
    );
    assert!(
        css.contains("grid-column: 1 / -1"),
        "full-width card should use 1 / -1, got: {css}"
    );
}

#[test]
fn dashboard_auto_rows_css_emits_auto() {
    let layout = Layout::dashboard([("a", 1), ("b", 1), ("c", 1), ("d", 1)])
        .columns(2)
        .auto_rows()
        .build()
        .unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("grid-auto-rows: auto"),
        "auto_rows should emit grid-auto-rows: auto, got: {css}"
    );
    assert!(
        !css.contains("grid-auto-rows: 1fr"),
        "auto_rows should NOT emit grid-auto-rows: 1fr, got: {css}"
    );
}

#[test]
fn dashboard_default_rows_css_emits_1fr() {
    let layout = Layout::dashboard([("a", 1), ("b", 1)])
        .columns(2)
        .build()
        .unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("grid-auto-rows: 1fr"),
        "default rows should emit grid-auto-rows: 1fr, got: {css}"
    );
}

#[test]
fn css_emits_min_max_height() {
    let mut b = LayoutBuilder::new();
    b.row(|r| {
        r.panel_with("a", grow(1.0).max_height(200.0));
    })
    .unwrap();
    let layout = b.build().unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("max-height: 200px"),
        "missing max-height: 200px, got: {css}"
    );
}

#[test]
fn css_emits_align_self() {
    let mut b = LayoutBuilder::new();
    b.row(|r| {
        r.panel_with("a", grow(1.0).align(Align::Center));
    })
    .unwrap();
    let layout = b.build().unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("align-self: center"),
        "missing align-self: center, got: {css}"
    );
}

#[test]
fn css_omits_defaults() {
    let mut b = LayoutBuilder::new();
    b.row(|r| {
        r.panel("a");
    })
    .unwrap();
    let layout = b.build().unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        !css.contains("min-height"),
        "should not contain min-height, got: {css}"
    );
    assert!(
        !css.contains("max-height"),
        "should not contain max-height, got: {css}"
    );
    assert!(
        !css.contains("min-width"),
        "should not contain min-width, got: {css}"
    );
    assert!(
        !css.contains("max-width"),
        "should not contain max-width, got: {css}"
    );
    assert!(
        !css.contains("align-self"),
        "should not contain align-self, got: {css}"
    );
}

#[test]
fn css_emits_min_content() {
    let mut b = LayoutBuilder::new();
    b.row(|r| {
        r.panel_with("a", grow(1.0).size_mode(SizeMode::MinContent));
        r.panel("b");
    })
    .unwrap();
    let layout = b.build().unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("min-content"),
        "should contain min-content, got: {css}"
    );
    // size_mode emits flex-basis override
    assert!(
        css.contains("flex-basis: min-content"),
        "should contain flex-basis: min-content, got: {css}"
    );
}

#[test]
fn css_emits_fit_content() {
    let mut b = LayoutBuilder::new();
    b.row(|r| {
        r.panel_with("a", grow(1.0).size_mode(SizeMode::FitContent(300.0)));
    })
    .unwrap();
    let layout = b.build().unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("fit-content(300px)"),
        "should contain fit-content(300px), got: {css}"
    );
}

#[test]
fn css_omits_size_mode_when_none() {
    let mut b = LayoutBuilder::new();
    b.row(|r| {
        r.panel("a");
    })
    .unwrap();
    let layout = b.build().unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        !css.contains("min-content"),
        "should not contain min-content, got: {css}"
    );
    assert!(
        !css.contains("max-content"),
        "should not contain max-content, got: {css}"
    );
    assert!(
        !css.contains("fit-content"),
        "should not contain fit-content, got: {css}"
    );
}

#[test]
fn css_scrollable_emits_overflow() {
    let layout = scrollable_layout(&["a", "b", "c"], 80.0);
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("overflow-x: auto"),
        "scrollable container should emit overflow-x: auto, got: {css}"
    );
}

#[test]
fn css_scrollable_emits_snap() {
    let layout = scrollable_layout(&["a", "b", "c"], 80.0);
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("scroll-snap-type: x mandatory"),
        "scrollable container should emit scroll-snap-type, got: {css}"
    );
    assert!(
        css.contains("overscroll-behavior: contain"),
        "scrollable container should emit overscroll-behavior, got: {css}"
    );
    // Each panel should get scroll-snap-align
    for kind in &["a", "b", "c"] {
        let panel_css = css
            .lines()
            .find(|l| l.contains(&format!("[data-pane=\"{kind}\"]")))
            .unwrap_or("");
        assert!(
            panel_css.contains("scroll-snap-align: start"),
            "panel {kind} should have scroll-snap-align: start, got: {panel_css}"
        );
    }
}

#[test]
fn css_non_scrollable_omits_scroll() {
    let layout = Layout::row(["a", "b", "c"]).unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        !css.contains("overflow"),
        "non-scrollable layout should not contain overflow, got: {css}"
    );
    assert!(
        !css.contains("scroll-snap"),
        "non-scrollable layout should not contain scroll-snap, got: {css}"
    );
}

/// Helper: build a runtime with overlays, return the layout and overlay defs.
fn layout_with_overlays(overlays: Vec<(&str, Overlay)>) -> (Layout, Vec<panes::OverlayDef>) {
    let mut rt = Layout::master_stack(["a", "b"]).into_runtime().unwrap();
    for (kind, builder) in overlays {
        rt.add_overlay(kind, builder).unwrap();
    }
    let defs: Vec<panes::OverlayDef> = rt.overlays().to_vec();
    let layout = Layout::row(["a", "b"]).unwrap();
    (layout, defs)
}

#[test]
fn css_overlay_center_positioning() {
    let (layout, overlays) =
        layout_with_overlays(vec![("picker", Overlay::center().fixed(400.0, 300.0))]);
    let css = panes_css::emit_with_overlays(&layout, &overlays);

    assert!(
        css.contains(r#"[data-pane-overlay="picker"]"#),
        "missing overlay selector, got: {css}"
    );
    assert!(
        css.contains("position: absolute"),
        "missing position: absolute, got: {css}"
    );
    assert!(
        css.contains("width: 400px"),
        "missing width: 400px, got: {css}"
    );
    assert!(
        css.contains("height: 300px"),
        "missing height: 300px, got: {css}"
    );
    // Center positioning uses transform
    assert!(css.contains("top: 50%"), "missing top: 50%, got: {css}");
    assert!(css.contains("left: 50%"), "missing left: 50%, got: {css}");
    assert!(
        css.contains("translate(-50%, -50%)"),
        "missing centering transform, got: {css}"
    );
}

#[test]
fn css_overlay_bottom_anchor() {
    let (layout, overlays) = layout_with_overlays(vec![(
        "bar",
        Overlay::bottom(10.0).full_width().height(50.0),
    )]);
    let css = panes_css::emit_with_overlays(&layout, &overlays);

    assert!(
        css.contains("bottom: 10px"),
        "missing bottom: 10px, got: {css}"
    );
}

#[test]
fn css_overlay_z_order() {
    let (layout, overlays) = layout_with_overlays(vec![
        ("first", Overlay::center().fixed(100.0, 100.0)),
        ("second", Overlay::top(0.0).fixed(200.0, 50.0)),
    ]);
    let css = panes_css::emit_with_overlays(&layout, &overlays);

    // Extract lines for each overlay
    let first_line = css
        .lines()
        .find(|l| l.contains(r#"[data-pane-overlay="first"]"#))
        .unwrap_or("");
    let second_line = css
        .lines()
        .find(|l| l.contains(r#"[data-pane-overlay="second"]"#))
        .unwrap_or("");

    assert!(
        first_line.contains("z-index: 1"),
        "first overlay should have z-index: 1, got: {first_line}"
    );
    assert!(
        second_line.contains("z-index: 2"),
        "second overlay should have z-index: 2, got: {second_line}"
    );
}

#[test]
fn css_root_gets_position_relative() {
    let (layout, overlays) =
        layout_with_overlays(vec![("picker", Overlay::center().fixed(100.0, 100.0))]);
    let css = panes_css::emit_with_overlays(&layout, &overlays);

    // Root selector should include position: relative
    let root_line = css
        .lines()
        .find(|l| l.contains("[data-pane-root]"))
        .unwrap_or("");
    assert!(
        root_line.contains("position: relative"),
        "root should have position: relative when overlays present, got: {root_line}"
    );
}

#[test]
fn css_emit_without_overlays_unchanged() {
    let layout = Layout::row(["a", "b"]).unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        !css.contains("position: relative"),
        "emit() without overlays should not include position: relative, got: {css}"
    );
    assert!(
        !css.contains("data-pane-overlay"),
        "emit() without overlays should not include overlay selectors, got: {css}"
    );
}

#[test]
fn css_panel_anchor_overlay_adds_anchor_container_rule() {
    let mut runtime = Layout::master_stack(["editor", "chat"])
        .into_runtime()
        .unwrap();
    let frame = runtime.resolve(600.0, 300.0).unwrap();
    let editor = frame.layout().by_kind("editor")[0];
    let key = runtime.panel_key(editor).unwrap();
    runtime
        .add_overlay("tooltip", Overlay::above_key(key).fixed(60.0, 10.0))
        .unwrap();

    let overlays = runtime.overlays().to_vec();
    let layout = Layout::row(["editor", "chat"]).unwrap();
    let css = panes_css::emit_with_overlays(&layout, &overlays);

    assert!(
        css.contains(&format!(
            r#"[data-pane-key="{key}"] {{ position: relative; }}"#
        )),
        "missing panel anchor container rule, got: {css}"
    );
}

#[test]
fn css_escapes_panel_kind_in_selector() {
    let layout = Layout::row(["kind\"\\line\nfeed"]).unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("[data-pane=\"kind\\\"\\\\line\\a feed\"]"),
        "missing escaped panel selector, got: {css}"
    );
}

#[test]
fn css_escapes_overlay_and_anchor_kinds_in_selectors() {
    let mut runtime = Layout::master_stack(["panel\"kind", "chat"])
        .into_runtime()
        .unwrap();
    runtime
        .add_overlay(
            "overlay\"kind\\line\nfeed",
            Overlay::below("panel\"kind").fixed(60.0, 10.0),
        )
        .unwrap();

    let overlays = runtime.overlays().to_vec();
    let layout = Layout::row(["panel\"kind", "chat"]).unwrap();
    let css = panes_css::emit_with_overlays(&layout, &overlays);

    assert!(
        css.contains("[data-pane=\"panel\\\"kind\"] { position: relative; }"),
        "missing escaped anchor selector, got: {css}"
    );
    assert!(
        css.contains("[data-pane-overlay=\"overlay\\\"kind\\\\line\\a feed\"]"),
        "missing escaped overlay selector, got: {css}"
    );
}

#[test]
fn css_escapes_null_bytes_in_selector() {
    let layout = Layout::row(["kind\0embedded"]).unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("[data-pane=\"kind\\0 embedded\"]"),
        "missing null-byte escaped selector, got: {css}"
    );
}

#[test]
fn css_transitions_on_panels() {
    let layout = Layout::row(["editor", "terminal"]).unwrap();
    let css = panes_css::emit_with_transitions(&layout);

    // Each panel should have transition properties
    for kind in &["editor", "terminal"] {
        let panel_line = css
            .lines()
            .find(|l| l.contains(&format!("[data-pane=\"{kind}\"]")))
            .unwrap_or("");
        assert!(
            panel_line.contains("transition:"),
            "panel {kind} should have transition property, got: {panel_line}"
        );
        // Transition should cover position and size
        assert!(
            panel_line.contains("var(--pane-transition)"),
            "panel {kind} transition should use custom property, got: {panel_line}"
        );
    }
}

#[test]
fn css_transitions_custom_property() {
    let layout = Layout::row(["a", "b"]).unwrap();
    let css = panes_css::emit_with_transitions(&layout);

    let root_line = css
        .lines()
        .find(|l| l.contains("[data-pane-root]"))
        .unwrap_or("");
    assert!(
        root_line.contains("--pane-transition:"),
        "root should have --pane-transition custom property, got: {root_line}"
    );
}

#[test]
fn css_emit_no_transitions_by_default() {
    let layout = Layout::row(["a", "b"]).unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        !css.contains("transition:"),
        "emit() should not include transition properties by default, got: {css}"
    );
    assert!(
        !css.contains("--pane-transition"),
        "emit() should not include --pane-transition by default, got: {css}"
    );
}
