use panes::{CardSpan, Layout, LayoutBuilder, fixed, grow};

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

    let css = panes_css::emit_adaptive(&[(0, &narrow), (600, &wide)]);

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

    let css = panes_css::emit_adaptive(&[(0, &small), (600, &medium), (1200, &large)]);

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
    let css = panes_css::emit_adaptive(&[(0, &layout)]);

    assert!(
        !css.contains("@media"),
        "single breakpoint should not wrap in @media, got: {css}"
    );
    assert!(css.contains("[data-pane=\"a\"]"), "missing panel a");
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
