use panes::{Layout, LayoutBuilder, fixed, gap, grow};

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
    b.col(gap(4.0), |c| {
        c.panel("top", grow(1.0))?;
        c.panel("bot", grow(1.0))?;
        Ok(())
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
    b.row(gap(0.0), |r| {
        r.panel("sidebar", fixed(20.0))?;
        r.panel("content", grow(1.0))?;
        Ok(())
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
    b.row(gap(0.0), |r| {
        r.panel("main", grow(2.0))?;
        r.panel("side", grow(1.0))?;
        Ok(())
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
    b.row(gap(0.0), |r| {
        r.panel("left", grow(1.0))?;
        r.col(gap(0.0), |c| {
            c.panel("top", grow(1.0))?;
            c.panel("bot", grow(1.0))?;
            Ok(())
        })?;
        Ok(())
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
    b.row(gap(0.0), |r| {
        r.panel("panel", grow(1.0).min(20.0).max(80.0))?;
        Ok(())
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
    b.col(gap(0.0), |c| {
        c.panel("panel", grow(1.0).min(10.0).max(50.0))?;
        Ok(())
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
