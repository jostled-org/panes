use std::sync::Arc;

use panes::Strategy;
use panes::runtime::LayoutRuntime;

// ---------------------------------------------------------------------------
// Factory → with_panels → into_runtime → resolve
// ---------------------------------------------------------------------------

fn assert_all_panels_resolve(name: &str, mut rt: LayoutRuntime, w: f32, h: f32, kinds: &[&str]) {
    let frame = rt.resolve(w, h).unwrap();
    for kind in kinds {
        assert_eq!(
            frame.layout().by_kind(kind).len(),
            1,
            "{name}: expected exactly one panel of kind \"{kind}\""
        );
    }
}

#[test]
fn preset_smoke_tests() {
    let cases: Vec<(&str, LayoutRuntime, f32, f32, Vec<&str>)> = vec![
        (
            "master_stack",
            Strategy::master_stack()
                .master_ratio(0.6)
                .gap(1.0)
                .with_panels(["editor", "chat", "status"])
                .into_runtime()
                .unwrap(),
            100.0,
            24.0,
            vec!["editor", "chat", "status"],
        ),
        (
            "centered_master",
            Strategy::centered_master()
                .master_ratio(0.5)
                .with_panels(["a", "b", "c", "d", "e"])
                .into_runtime()
                .unwrap(),
            100.0,
            100.0,
            vec!["a", "b", "c", "d", "e"],
        ),
        (
            "deck",
            Strategy::deck()
                .master_ratio(0.5)
                .gap(2.0)
                .with_panels(["master", "s1", "s2"])
                .into_runtime()
                .unwrap(),
            100.0,
            100.0,
            vec!["master", "s1", "s2"],
        ),
        (
            "monocle",
            Strategy::monocle()
                .with_panels(["a", "b", "c"])
                .into_runtime()
                .unwrap(),
            100.0,
            100.0,
            vec!["a", "b", "c"],
        ),
        (
            "tabbed",
            Strategy::tabbed()
                .bar_height(2.0)
                .with_panels(["t1", "t2", "t3"])
                .into_runtime()
                .unwrap(),
            100.0,
            100.0,
            vec!["t1", "t2", "t3"],
        ),
        (
            "stacked",
            Strategy::stacked()
                .bar_height(1.5)
                .with_panels(["s1", "s2"])
                .into_runtime()
                .unwrap(),
            100.0,
            100.0,
            vec!["s1", "s2"],
        ),
        (
            "scrollable",
            Strategy::scrollable()
                .size(3)
                .gap(1.0)
                .with_panels(["p1", "p2", "p3", "p4", "p5"])
                .into_runtime()
                .unwrap(),
            100.0,
            100.0,
            vec!["p1", "p2", "p3", "p4", "p5"],
        ),
        (
            "dwindle",
            Strategy::dwindle()
                .ratio(0.6)
                .gap(1.0)
                .with_panels(["a", "b", "c", "d"])
                .into_runtime()
                .unwrap(),
            100.0,
            100.0,
            vec!["a", "b", "c", "d"],
        ),
        (
            "spiral",
            Strategy::spiral()
                .ratio(0.5)
                .with_panels(["a", "b", "c"])
                .into_runtime()
                .unwrap(),
            100.0,
            100.0,
            vec!["a", "b", "c"],
        ),
    ];

    for (name, rt, w, h, kinds) in cases {
        assert_all_panels_resolve(name, rt, w, h, &kinds);
    }
}

// ---------------------------------------------------------------------------
// Split (fixed 2 panels)
// ---------------------------------------------------------------------------

#[test]
fn split_horizontal_runtime() {
    let mut rt = Strategy::split()
        .ratio(0.7)
        .gap(1.0)
        .with_panels("editor", "terminal")
        .into_runtime()
        .unwrap();

    let frame = rt.resolve(100.0, 100.0).unwrap();
    assert_eq!(frame.layout().by_kind("editor").len(), 1);
    assert_eq!(frame.layout().by_kind("terminal").len(), 1);
}

#[test]
fn split_vertical_runtime() {
    let mut rt = Strategy::split()
        .ratio(0.5)
        .vertical()
        .with_panels("top", "bottom")
        .into_runtime()
        .unwrap();

    let frame = rt.resolve(100.0, 100.0).unwrap();
    let top_rect = frame
        .layout()
        .get(frame.layout().by_kind("top")[0])
        .unwrap();
    let bot_rect = frame
        .layout()
        .get(frame.layout().by_kind("bottom")[0])
        .unwrap();
    assert!(top_rect.y < bot_rect.y);
}

// ---------------------------------------------------------------------------
// Dashboard
// ---------------------------------------------------------------------------

#[test]
fn dashboard_with_cards() {
    let mut rt = Strategy::dashboard()
        .columns(3)
        .gap(2.0)
        .with_cards([("chart", 2), ("stats", 1), ("log", 1)])
        .into_runtime()
        .unwrap();

    let frame = rt.resolve(300.0, 200.0).unwrap();
    assert_eq!(frame.layout().by_kind("chart").len(), 1);
    assert_eq!(frame.layout().by_kind("stats").len(), 1);
    assert_eq!(frame.layout().by_kind("log").len(), 1);
}

#[test]
fn dashboard_with_panels_default_spans() {
    let mut rt = Strategy::dashboard()
        .columns(2)
        .with_panels(["a", "b", "c", "d"])
        .into_runtime()
        .unwrap();

    let frame = rt.resolve(200.0, 200.0).unwrap();
    assert_eq!(frame.layout().by_kind("a").len(), 1);
    assert_eq!(frame.layout().by_kind("d").len(), 1);
}

#[test]
fn dashboard_auto_fill() {
    let mut rt = Strategy::dashboard()
        .auto_fill(100.0)
        .gap(8.0)
        .with_cards([("chart", 2), ("stats", 1)])
        .into_runtime()
        .unwrap();

    let frame = rt.resolve(500.0, 300.0).unwrap();
    assert_eq!(frame.layout().by_kind("chart").len(), 1);
}

#[test]
fn dashboard_auto_fit() {
    let mut rt = Strategy::dashboard()
        .auto_fit(80.0)
        .with_panels(["x", "y", "z"])
        .into_runtime()
        .unwrap();

    let frame = rt.resolve(400.0, 200.0).unwrap();
    assert_eq!(frame.layout().by_kind("x").len(), 1);
}

// ---------------------------------------------------------------------------
// Sidebar (fixed 2 named slots)
// ---------------------------------------------------------------------------

#[test]
fn sidebar_runtime() {
    let mut rt = Strategy::sidebar()
        .sidebar_width(25.0)
        .gap(1.0)
        .with_panels("nav", "content")
        .into_runtime()
        .unwrap();

    let frame = rt.resolve(100.0, 100.0).unwrap();
    let nav_rect = frame
        .layout()
        .get(frame.layout().by_kind("nav")[0])
        .unwrap();
    let content_rect = frame
        .layout()
        .get(frame.layout().by_kind("content")[0])
        .unwrap();
    assert_eq!(nav_rect.x, 0.0);
    assert!(content_rect.x > nav_rect.w);
}

// ---------------------------------------------------------------------------
// Holy grail (fixed 5 named slots)
// ---------------------------------------------------------------------------

#[test]
fn holy_grail_runtime() {
    let mut rt = Strategy::holy_grail()
        .header_height(2.0)
        .footer_height(2.0)
        .sidebar_width(15.0)
        .gap(1.0)
        .with_panels("header", "footer", "left", "main", "right")
        .unwrap()
        .into_runtime()
        .unwrap();

    let frame = rt.resolve(100.0, 100.0).unwrap();
    assert_eq!(frame.layout().by_kind("header").len(), 1);
    assert_eq!(frame.layout().by_kind("footer").len(), 1);
    assert_eq!(frame.layout().by_kind("left").len(), 1);
    assert_eq!(frame.layout().by_kind("main").len(), 1);
    assert_eq!(frame.layout().by_kind("right").len(), 1);
}

// ---------------------------------------------------------------------------
// Factory → with_panels → build (static Layout)
// ---------------------------------------------------------------------------

#[test]
fn master_stack_static_build() {
    let layout = Strategy::master_stack()
        .master_ratio(0.6)
        .with_panels(["a", "b", "c"])
        .build()
        .unwrap();

    let resolved = layout.resolve(100.0, 24.0).unwrap();
    assert_eq!(resolved.by_kind("a").len(), 1);
}

#[test]
fn holy_grail_static_build() {
    let layout = Strategy::holy_grail()
        .with_panels("h", "f", "l", "m", "r")
        .unwrap()
        .build()
        .unwrap();

    let resolved = layout.resolve(100.0, 100.0).unwrap();
    assert_eq!(resolved.by_kind("h").len(), 1);
    assert_eq!(resolved.by_kind("m").len(), 1);
}

// ---------------------------------------------------------------------------
// Strategy reuse — same shape, different panels
// ---------------------------------------------------------------------------

#[test]
fn strategy_reuse() {
    let strategy = Strategy::master_stack().master_ratio(0.6).gap(1.0).build();

    let mut rt1 = strategy
        .clone()
        .with_panels(["editor", "chat", "status"])
        .into_runtime()
        .unwrap();
    let frame1 = rt1.resolve(100.0, 24.0).unwrap();
    assert_eq!(frame1.layout().by_kind("editor").len(), 1);

    let mut rt2 = strategy
        .with_panels(["code", "terminal"])
        .into_runtime()
        .unwrap();
    let frame2 = rt2.resolve(100.0, 24.0).unwrap();
    assert_eq!(frame2.layout().by_kind("code").len(), 1);
    assert_eq!(frame2.layout().by_kind("terminal").len(), 1);
}

#[test]
fn strategy_reuse_dashboard() {
    let strategy = Strategy::dashboard().auto_fill(200.0).gap(8.0);

    let mut rt1 = strategy
        .clone()
        .with_cards([("chart", 2), ("stats", 1)])
        .into_runtime()
        .unwrap();
    let frame1 = rt1.resolve(800.0, 400.0).unwrap();
    assert_eq!(frame1.layout().by_kind("chart").len(), 1);

    let mut rt2 = strategy
        .with_cards([("logs", 1), ("metrics", 1), ("alerts", 1)])
        .into_runtime()
        .unwrap();
    let frame2 = rt2.resolve(800.0, 400.0).unwrap();
    assert_eq!(frame2.layout().by_kind("logs").len(), 1);
}

// ---------------------------------------------------------------------------
// Strategy::from_kind round-trip
// ---------------------------------------------------------------------------

#[test]
fn from_kind_round_trip() {
    let kind = panes::StrategyKind::MasterStack {
        master_ratio: 0.7,
        gap: 2.0,
    };
    let strategy = Strategy::from_kind(kind);

    let mut rt = strategy
        .with_panels(["a", "b", "c"])
        .into_runtime()
        .unwrap();
    let frame = rt.resolve(100.0, 100.0).unwrap();
    assert_eq!(frame.layout().by_kind("a").len(), 1);
}

// ---------------------------------------------------------------------------
// Error cases
// ---------------------------------------------------------------------------

#[test]
fn empty_panels_is_error() {
    let result = Strategy::master_stack()
        .with_panels(Vec::<&str>::new())
        .into_runtime();
    assert!(result.is_err());
}

#[test]
fn dashboard_empty_cards_is_error() {
    let result = Strategy::dashboard()
        .with_cards(Vec::<(&str, usize)>::new())
        .into_runtime();
    assert!(result.is_err());
}

#[test]
fn split_ratio_preserved_through_rebuild() {
    let mut rt = Strategy::split()
        .ratio(0.7)
        .gap(1.0)
        .with_panels("editor", "terminal")
        .into_runtime()
        .unwrap();

    let frame = rt.resolve(100.0, 100.0).unwrap();
    let editor_w = frame
        .layout()
        .get(frame.layout().by_kind("editor")[0])
        .unwrap()
        .w;
    let terminal_w = frame
        .layout()
        .get(frame.layout().by_kind("terminal")[0])
        .unwrap()
        .w;

    // With ratio 0.7, editor should get ~70% of available width (minus gap).
    // The gap is 1.0, so available = 99.0, editor ≈ 69.3, terminal ≈ 29.7.
    assert!(
        editor_w > terminal_w * 2.0,
        "editor ({editor_w}) should be more than twice terminal ({terminal_w}) at 0.7 ratio"
    );

    // Add and remove a panel to trigger a rebuild via the strategy.
    let pid = rt.add_panel(Arc::from("temp")).unwrap();
    rt.remove_panel(pid).unwrap();

    let frame2 = rt.resolve(100.0, 100.0).unwrap();
    let editor_w2 = frame2
        .layout()
        .get(frame2.layout().by_kind("editor")[0])
        .unwrap()
        .w;
    let terminal_w2 = frame2
        .layout()
        .get(frame2.layout().by_kind("terminal")[0])
        .unwrap()
        .w;

    // Ratio must survive the rebuild.
    let tolerance = 0.1;
    assert!(
        (editor_w2 - editor_w).abs() < tolerance,
        "editor width changed after rebuild: {editor_w} → {editor_w2}"
    );
    assert!(
        (terminal_w2 - terminal_w).abs() < tolerance,
        "terminal width changed after rebuild: {terminal_w} → {terminal_w2}"
    );
}

#[test]
fn builder_build_then_with_panels() {
    let strategy = Strategy::dwindle().ratio(0.6).gap(1.0).build();
    let mut rt = strategy
        .with_panels(["a", "b", "c"])
        .into_runtime()
        .unwrap();
    let frame = rt.resolve(100.0, 100.0).unwrap();
    assert_eq!(frame.layout().by_kind("a").len(), 1);
}
