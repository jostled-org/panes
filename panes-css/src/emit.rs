use std::fmt::Write as _;

// `fmt::Write` for `String` is infallible. `let _ =` discards the unused `Result`.

use panes::compiler::Axis;
use panes::{Constraints, Layout, LayoutTree, Node, NodeId};

/// Mutable state threaded through recursive CSS emission.
struct EmitCtx {
    css: String,
    counter: u32,
}

/// Emit a CSS string from a `Layout` tree.
///
/// The browser acts as the layout solver via flexbox properties.
/// Panels use `[data-pane="kind"]` selectors, containers use
/// `[data-pane-node="N"]`, and the root uses `[data-pane-root]`.
pub fn emit(layout: &Layout) -> String {
    let tree = layout.tree();
    let Some(root_id) = tree.root() else {
        return String::new();
    };
    let mut ctx = EmitCtx {
        css: String::new(),
        counter: 0,
    };
    emit_node(tree, root_id, Axis::Horizontal, true, &mut ctx);
    ctx.css
}

fn emit_node(tree: &LayoutTree, nid: NodeId, parent_axis: Axis, is_root: bool, ctx: &mut EmitCtx) {
    let Some(node) = tree.node(nid) else { return };
    match node {
        Node::Panel {
            kind, constraints, ..
        } => {
            write_panel_rule(kind, constraints, parent_axis, &mut ctx.css);
        }
        Node::Row { gap, children } => {
            let sel = container_selector(is_root, &mut ctx.counter);
            write_container_rule(&sel, "row", *gap, is_root, &mut ctx.css);
            emit_children(tree, children, Axis::Horizontal, ctx);
        }
        Node::Col { gap, children } => {
            let sel = container_selector(is_root, &mut ctx.counter);
            write_container_rule(&sel, "column", *gap, is_root, &mut ctx.css);
            emit_children(tree, children, Axis::Vertical, ctx);
        }
        Node::TaffyPassthrough { style, children } if style.display == taffy::Display::Grid => {
            let sel = container_selector(is_root, &mut ctx.counter);
            write_grid_rule(&sel, style, is_root, &mut ctx.css);
            emit_grid_children(tree, children, &mut ctx.counter, &mut ctx.css);
        }
        Node::TaffyPassthrough { children, .. } => {
            let sel = container_selector(is_root, &mut ctx.counter);
            write_passthrough_rule(&sel, is_root, &mut ctx.css);
            emit_children(tree, children, parent_axis, ctx);
        }
    }
}

fn emit_children(tree: &LayoutTree, children: &[NodeId], axis: Axis, ctx: &mut EmitCtx) {
    for &child_id in children {
        emit_node(tree, child_id, axis, false, ctx);
    }
}

fn container_selector(is_root: bool, counter: &mut u32) -> String {
    match is_root {
        true => "[data-pane-root]".to_string(),
        false => {
            *counter += 1;
            format!("[data-pane-node=\"{}\"]", counter)
        }
    }
}

fn write_container_rule(
    selector: &str,
    direction: &str,
    gap: f32,
    is_root: bool,
    css: &mut String,
) {
    let _ = write!(
        css,
        "{selector} {{ display: flex; flex-direction: {direction}; gap: {gap}px;"
    );
    if !is_root {
        css.push_str(" flex-grow: 1; flex-basis: 0px; flex-shrink: 1;");
    }
    css.push_str(" }\n");
}

fn write_panel_rule(kind: &str, constraints: &Constraints, parent_axis: Axis, css: &mut String) {
    let _ = write!(css, "[data-pane=\"{kind}\"] {{ ");
    write_flex_sizing(constraints, css);
    write_min_max(constraints, parent_axis, css);
    css.push_str(" }\n");
}

fn write_grid_rule(selector: &str, style: &taffy::Style, is_root: bool, css: &mut String) {
    let cols = style.grid_template_columns.len();
    let _ = write!(
        css,
        "{selector} {{ display: grid; grid-template-columns: repeat({cols}, 1fr);"
    );
    if !style.grid_auto_rows.is_empty() {
        css.push_str(" grid-auto-rows: 1fr;");
    }
    let gap = style.gap.width.into_raw().value();
    if gap > 0.0 {
        let _ = write!(css, " gap: {gap}px;");
    }
    if !is_root {
        css.push_str(" flex-grow: 1; flex-basis: 0px;");
    }
    css.push_str(" }\n");
}

fn emit_grid_children(tree: &LayoutTree, children: &[NodeId], counter: &mut u32, css: &mut String) {
    for &child_id in children {
        let Some(Node::TaffyPassthrough { style, .. }) = tree.node(child_id) else {
            continue;
        };
        let sel = container_selector(false, counter);
        write_grid_card_rule(&sel, style, css);
        emit_grid_card_panels(tree, child_id, css);
    }
}

fn write_grid_card_rule(sel: &str, style: &taffy::Style, css: &mut String) {
    let span = grid_column_span(style);
    let _ = write!(css, "{sel} {{ display: flex;");
    if span > 1 {
        let _ = write!(css, " grid-column: span {span};");
    }
    css.push_str(" flex-grow: 1; flex-basis: 0px; flex-shrink: 1; }\n");
}

fn emit_grid_card_panels(tree: &LayoutTree, card_id: NodeId, css: &mut String) {
    let Some(node) = tree.node(card_id) else {
        return;
    };
    for &grandchild in node.children() {
        let Some(Node::Panel {
            kind, constraints, ..
        }) = tree.node(grandchild)
        else {
            continue;
        };
        write_panel_rule(kind, constraints, Axis::Horizontal, css);
    }
}

fn grid_column_span(style: &taffy::Style) -> u16 {
    match style.grid_column.end {
        taffy::GridPlacement::Span(n) => n,
        _ => 1,
    }
}

fn write_passthrough_rule(selector: &str, is_root: bool, css: &mut String) {
    let _ = write!(css, "{selector} {{ display: flex;");
    if !is_root {
        css.push_str(" flex-grow: 1; flex-basis: 0px; flex-shrink: 1;");
    }
    css.push_str(" }\n");
}

fn write_flex_sizing(constraints: &Constraints, css: &mut String) {
    match (constraints.grow, constraints.fixed) {
        (Some(g), _) => {
            let _ = write!(css, "flex-grow: {g}; flex-basis: 0px; flex-shrink: 1;");
        }
        (_, Some(f)) => {
            let _ = write!(css, "flex-grow: 0; flex-basis: {f}px; flex-shrink: 0;");
        }
        (None, None) => {
            css.push_str("flex-grow: 1; flex-basis: 0px; flex-shrink: 1;");
        }
    }
}

fn write_min_max(constraints: &Constraints, axis: Axis, css: &mut String) {
    let (min_prop, max_prop) = match axis {
        Axis::Horizontal => ("min-width", "max-width"),
        Axis::Vertical => ("min-height", "max-height"),
    };
    if let Some(min) = constraints.min {
        let _ = write!(css, " {min_prop}: {min}px;");
    }
    if let Some(max) = constraints.max {
        let _ = write!(css, " {max_prop}: {max}px;");
    }
}
