use panes::{Grid, Layout, LayoutBuilder, LayoutTree};
use panes_css::{PaneElement, PaneSelector};

#[cfg(test)]
fn sample_layouts() -> Vec<(&'static str, Layout)> {
    vec![
        (
            "dashboard",
            Layout::dashboard([("a", 1), ("b", 2), ("c", 1)])
                .columns(3)
                .build()
                .unwrap(),
        ),
        (
            "master_stack",
            Layout::master_stack(["editor", "chat", "status"])
                .build()
                .unwrap(),
        ),
        (
            "dwindle",
            Layout::dwindle(["a", "b", "c", "d"]).build().unwrap(),
        ),
        (
            "holy_grail",
            Layout::holy_grail("header", "footer", "nav", "main", "aside")
                .build()
                .unwrap(),
        ),
        (
            "scrollable",
            Layout::scrollable(["p1", "p2", "p3"]).build().unwrap(),
        ),
        ("tabbed", Layout::tabbed(["one", "two"]).build().unwrap()),
        (
            "nested_macro",
            panes::layout! {
                row(gap: 8.0) {
                    panel("sidebar", fixed: 200.0)
                    grid(columns: 2) {
                        panel("chart", rows: 2)
                        panel("stats")
                        col {
                            panel("log")
                            panel("alerts")
                        }
                    }
                }
            }
            .unwrap(),
        ),
    ]
}

fn node_numbers(skeleton: &[PaneElement<'_>]) -> Vec<u32> {
    skeleton
        .iter()
        .filter_map(|e| match e.selector {
            PaneSelector::Node(n) => Some(n),
            _ => None,
        })
        .collect()
}

#[test]
fn dashboard_skeleton_matches_issue_example() {
    let layout = Layout::dashboard([("a", 1), ("b", 2), ("c", 1)])
        .columns(3)
        .build()
        .unwrap();
    let skeleton = panes_css::skeleton(&layout);

    let selectors: Vec<String> = skeleton.iter().map(|e| e.selector.to_string()).collect();
    assert_eq!(
        selectors,
        [
            "[data-pane-root]",
            "[data-pane-node=\"1\"]",
            "[data-pane=\"a\"]",
            "[data-pane-node=\"2\"]",
            "[data-pane=\"b\"]",
            "[data-pane-node=\"3\"]",
            "[data-pane=\"c\"]",
        ]
    );
    let parents: Vec<Option<usize>> = skeleton.iter().map(|e| e.parent).collect();
    assert_eq!(
        parents,
        [None, Some(0), Some(1), Some(0), Some(3), Some(0), Some(5)]
    );
}

#[test]
fn every_selector_has_a_rule() {
    for (name, layout) in sample_layouts() {
        let css = panes_css::emit(&layout);
        for element in panes_css::skeleton(&layout).iter() {
            let rule = format!("{} {{", element.selector);
            assert!(
                css.contains(&rule),
                "{name}: no rule for {}, css: {css}",
                element.selector
            );
        }
    }
}

#[test]
fn node_numbers_match_emit_numbering() {
    for (name, layout) in sample_layouts() {
        let css = panes_css::emit(&layout);
        let numbers = node_numbers(&panes_css::skeleton(&layout));
        let expected: Vec<u32> = (1..=numbers.len() as u32).collect();
        assert_eq!(numbers, expected, "{name}: numbers must run 1..=N in order");
        assert_eq!(
            css.matches("[data-pane-node=").count(),
            numbers.len(),
            "{name}: emit numbers a different node count, css: {css}"
        );
    }
}

#[test]
fn parents_follow_the_tree() {
    for (name, layout) in sample_layouts() {
        let tree = layout.tree();
        let skeleton = panes_css::skeleton(&layout);
        for element in skeleton.iter() {
            let tree_parent = tree.parent(element.node).unwrap();
            let skeleton_parent = element.parent.map(|i| skeleton[i].node);
            assert_eq!(
                skeleton_parent, tree_parent,
                "{name}: parent of {} differs from the tree",
                element.selector
            );
        }
    }
}

#[test]
fn parents_precede_children() {
    for (name, layout) in sample_layouts() {
        for (index, element) in panes_css::skeleton(&layout).iter().enumerate() {
            if let Some(parent) = element.parent {
                assert!(parent < index, "{name}: parent after child at {index}");
            }
        }
    }
}

#[test]
fn panel_entries_carry_tree_ids() {
    for (name, layout) in sample_layouts() {
        let tree = layout.tree();
        let skeleton = panes_css::skeleton(&layout);
        let panels: Vec<_> = skeleton
            .iter()
            .filter_map(|e| match e.selector {
                PaneSelector::Panel { id, kind } => Some((e.node, id, kind)),
                _ => None,
            })
            .collect();
        assert_eq!(panels.len(), tree.panel_count(), "{name}: panel count");
        for (node, id, kind) in panels {
            assert_eq!(tree.node_for_panel(id), Some(node), "{name}: {kind}");
            assert_eq!(tree.panel_kind(id).unwrap(), kind, "{name}: kind");
        }
    }
}

#[test]
fn attributes_name_the_markup() {
    let layout = Layout::dashboard([("chart", 1)]).build().unwrap();
    let skeleton = panes_css::skeleton(&layout);

    let attributes: Vec<(&str, String)> = skeleton
        .iter()
        .map(|e| {
            let (name, value) = e.selector.attribute();
            (name, value.into_owned())
        })
        .collect();
    assert_eq!(
        attributes,
        [
            ("data-pane-root", String::new()),
            ("data-pane-node", "1".to_owned()),
            ("data-pane", "chart".to_owned()),
        ]
    );
}

#[test]
fn panel_selector_escapes_like_emit() {
    let layout = Layout::build_row(|r| {
        r.panel("say \"hi\"");
    })
    .unwrap();
    let css = panes_css::emit(&layout);
    let skeleton = panes_css::skeleton(&layout);

    let panel = skeleton
        .iter()
        .find(|e| matches!(e.selector, PaneSelector::Panel { .. }))
        .unwrap();
    assert_eq!(panel.selector.to_string(), "[data-pane=\"say \\\"hi\\\"\"]");
    assert!(css.contains(&panel.selector.to_string()));
}

#[test]
fn container_inside_grid_gets_a_rule() {
    let layout = Layout::build_grid(Grid::columns(2), |g| {
        g.panel("chart");
        g.col(|c| {
            c.panel("log");
            c.panel("alerts");
        });
    })
    .unwrap();
    let css = panes_css::emit(&layout);

    assert!(
        css.contains("[data-pane-node=\"1\"] { display: flex; flex-direction: column;"),
        "nested column inside a grid should emit a rule, got: {css}"
    );
    assert!(css.contains("[data-pane=\"log\"]"), "got: {css}");
    assert!(css.contains("[data-pane=\"alerts\"]"), "got: {css}");
}

#[test]
fn skeleton_accepts_a_runtime_tree() {
    let layout = Layout::dashboard([("a", 1), ("b", 1)]).build().unwrap();
    let expected: Vec<String> = panes_css::skeleton(&layout)
        .iter()
        .map(|e| e.selector.to_string())
        .collect();
    let tree: LayoutTree = layout.into();

    let actual: Vec<String> = panes_css::skeleton(&tree)
        .iter()
        .map(|e| e.selector.to_string())
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn empty_tree_has_empty_skeleton() {
    let tree = LayoutTree::new();
    assert!(panes_css::skeleton(&tree).is_empty());
    assert!(panes_css::emit(&tree).is_empty());
}

#[test]
fn builder_layout_skeleton_is_stable() {
    let mut builder = LayoutBuilder::new();
    builder
        .col(|c| {
            c.panel("top");
            c.row(|r| {
                r.panel("left");
                r.panel("right");
            });
        })
        .unwrap();
    let layout = builder.build().unwrap();

    let first: Vec<String> = panes_css::skeleton(&layout)
        .iter()
        .map(|e| e.selector.to_string())
        .collect();
    let second: Vec<String> = panes_css::skeleton(&layout)
        .iter()
        .map(|e| e.selector.to_string())
        .collect();
    assert_eq!(first, second);
    assert_eq!(
        first,
        [
            "[data-pane-root]",
            "[data-pane=\"top\"]",
            "[data-pane-node=\"1\"]",
            "[data-pane=\"left\"]",
            "[data-pane=\"right\"]",
        ]
    );
}
