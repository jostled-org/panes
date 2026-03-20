use std::fmt::Write as _;

// `fmt::Write` for `String` is infallible. `let _ =` discards the unused `Result`.

use panes::Direction;
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
    emit_node(tree, root_id, Direction::Horizontal, true, &mut ctx);
    ctx.css
}

fn emit_node(
    tree: &LayoutTree,
    nid: NodeId,
    parent_axis: Direction,
    is_root: bool,
    ctx: &mut EmitCtx,
) {
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
            emit_children(tree, children, Direction::Horizontal, ctx);
        }
        Node::Col { gap, children } => {
            let sel = container_selector(is_root, &mut ctx.counter);
            write_container_rule(&sel, "column", *gap, is_root, &mut ctx.css);
            emit_children(tree, children, Direction::Vertical, ctx);
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

fn emit_children(tree: &LayoutTree, children: &[NodeId], axis: Direction, ctx: &mut EmitCtx) {
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

fn write_panel_rule(
    kind: &str,
    constraints: &Constraints,
    parent_axis: Direction,
    css: &mut String,
) {
    let _ = write!(css, "[data-pane=\"{kind}\"] {{ ");
    write_flex_sizing(constraints, css);
    write_min_max(constraints, parent_axis, css);
    css.push_str(" }\n");
}

enum GridMode {
    Fixed(usize),
    AutoRepeat { kind: &'static str, min_px: f32 },
}

fn auto_repeat_kind(count: taffy::style::RepetitionCount) -> Option<&'static str> {
    match count {
        taffy::style::RepetitionCount::AutoFill => Some("auto-fill"),
        taffy::style::RepetitionCount::AutoFit => Some("auto-fit"),
        taffy::style::RepetitionCount::Count(_) => None,
    }
}

fn detect_grid_mode(columns: &[taffy::style::GridTemplateComponent<String>]) -> GridMode {
    let Some(taffy::style::GridTemplateComponent::Repeat(rep)) = columns.first() else {
        return GridMode::Fixed(columns.len());
    };
    let Some(kind) = auto_repeat_kind(rep.count) else {
        return GridMode::Fixed(columns.len());
    };
    let min_px = rep
        .tracks
        .first()
        .map(|t| t.min_sizing_function().into_raw().value())
        .unwrap_or(0.0);
    GridMode::AutoRepeat { kind, min_px }
}

fn write_grid_rule(selector: &str, style: &taffy::Style, is_root: bool, css: &mut String) {
    let _ = write!(css, "{selector} {{ display: grid;");
    match detect_grid_mode(&style.grid_template_columns) {
        GridMode::Fixed(cols) => {
            let _ = write!(css, " grid-template-columns: repeat({cols}, 1fr);");
        }
        GridMode::AutoRepeat { kind, min_px } => {
            let _ = write!(
                css,
                " grid-template-columns: repeat({kind}, minmax({min_px}px, 1fr));"
            );
        }
    }
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
        match tree.node(child_id) {
            Some(Node::TaffyPassthrough { style, .. }) => {
                let sel = container_selector(false, counter);
                write_grid_card_rule(&sel, style, css);
                emit_grid_card_panels(tree, child_id, css);
            }
            Some(Node::Panel {
                kind, constraints, ..
            }) => {
                write_panel_rule(kind, constraints, Direction::Horizontal, css);
            }
            _ => {}
        }
    }
}

fn write_grid_card_rule(sel: &str, style: &taffy::Style, css: &mut String) {
    let _ = write!(css, "{sel} {{ display: flex;");
    match grid_column_placement(style) {
        GridColumnPlacement::FullWidth => {
            css.push_str(" grid-column: 1 / -1;");
        }
        GridColumnPlacement::Span(n) if n > 1 => {
            let _ = write!(css, " grid-column: span {n};");
        }
        _ => {}
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
        write_panel_rule(kind, constraints, Direction::Horizontal, css);
    }
}

enum GridColumnPlacement {
    Span(u16),
    FullWidth,
}

fn grid_column_placement(style: &taffy::Style) -> GridColumnPlacement {
    match (&style.grid_column.start, &style.grid_column.end) {
        (taffy::GridPlacement::Line(s), taffy::GridPlacement::Line(e))
            if s.as_i16() == 1 && e.as_i16() == -1 =>
        {
            GridColumnPlacement::FullWidth
        }
        (_, taffy::GridPlacement::Span(n)) => GridColumnPlacement::Span(*n),
        _ => GridColumnPlacement::Span(1),
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

/// Emit CSS with `@media` wrappers for adaptive breakpoints.
///
/// Each entry is `(min_width_px, layout)`. Breakpoints must be sorted ascending
/// by min_width. The first breakpoint gets only a max-width query, the last gets
/// only a min-width query, and middle breakpoints get both.
pub fn emit_adaptive(breakpoints: &[(u32, &Layout)]) -> String {
    let mut css = String::new();
    let len = breakpoints.len();
    for (i, (min_width, layout)) in breakpoints.iter().enumerate() {
        let inner = emit(layout);
        let query = match (i, i + 1 < len) {
            (0, true) => {
                let next_min = breakpoints[i + 1].0;
                format!("@media (max-width: {}px)", next_min.saturating_sub(1))
            }
            (0, false) => {
                // Single breakpoint — no media query needed
                css.push_str(&inner);
                continue;
            }
            (_, true) => {
                let next_min = breakpoints[i + 1].0;
                format!(
                    "@media (min-width: {min_width}px) and (max-width: {}px)",
                    next_min.saturating_sub(1)
                )
            }
            (_, false) => format!("@media (min-width: {min_width}px)"),
        };
        let _ = write!(css, "{query} {{\n{inner}}}\n");
    }
    css
}

fn write_min_max(constraints: &Constraints, axis: Direction, css: &mut String) {
    let (min_prop, max_prop) = match axis {
        Direction::Horizontal => ("min-width", "max-width"),
        Direction::Vertical => ("min-height", "max-height"),
    };
    if let Some(min) = constraints.min {
        let _ = write!(css, " {min_prop}: {min}px;");
    }
    if let Some(max) = constraints.max {
        let _ = write!(css, " {max_prop}: {max}px;");
    }
}
