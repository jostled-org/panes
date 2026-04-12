use std::fmt::{Display, Write as _};

use panes::{
    Align, Axis, CardSpan, Constraints, ExtentValue, GridColumnMode, HAlign, Layout, LayoutTree,
    Node, NodeId, OverlayAnchor, OverlayDef, SizeMode, VAlign,
};
use thiserror::Error;

struct EmitCtx {
    css: String,
    counter: u32,
    root_position_relative: bool,
    transitions: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AdaptiveCssError {
    #[error(
        "adaptive breakpoints must be strictly increasing; index {index} width {width}px follows {previous_width}px"
    )]
    UnsortedOrDuplicate {
        index: usize,
        previous_width: u32,
        width: u32,
    },
}

/// Emit a CSS string from a `Layout` tree.
///
/// The browser acts as the layout solver via flexbox properties.
/// Panels use `[data-pane="kind"]` selectors, containers use
/// `[data-pane-node="N"]`, and the root uses `[data-pane-root]`.
pub fn emit(layout: &Layout) -> String {
    emit_tree(layout, false, false)
}

/// Emit CSS including absolute-positioned overlay rules.
///
/// The root selector gets `position: relative` so overlays can anchor
/// against it. Each `OverlayDef` produces a `[data-pane-overlay="kind"]`
/// rule with positioning, size, and z-index.
pub fn emit_with_overlays(layout: &Layout, overlays: &[OverlayDef]) -> String {
    emit_with_options(layout, overlays, false)
}

/// Emit CSS with transition properties on all panel selectors.
///
/// The root gets a `--pane-transition` custom property. Each panel selector
/// gets a `transition` shorthand referencing that variable for position and
/// size properties.
pub fn emit_with_transitions(layout: &Layout) -> String {
    emit_tree(layout, false, true)
}

/// Emit CSS with both overlay positioning and transition properties.
pub fn emit_full(layout: &Layout, overlays: &[OverlayDef]) -> String {
    emit_with_options(layout, overlays, true)
}

fn emit_with_options(layout: &Layout, overlays: &[OverlayDef], transitions: bool) -> String {
    let mut css = emit_tree(layout, !overlays.is_empty(), transitions);
    for (i, def) in overlays.iter().enumerate() {
        write_overlay_rule(def, i + 1, &mut css);
    }
    css
}

fn emit_tree(layout: &Layout, root_position_relative: bool, transitions: bool) -> String {
    let tree = layout.tree();
    let Some(root_id) = tree.root() else {
        return String::new();
    };
    let estimated_bytes = tree.node_count() * 80;
    let mut ctx = EmitCtx {
        css: String::with_capacity(estimated_bytes),
        counter: 0,
        root_position_relative,
        transitions,
    };
    emit_node(tree, root_id, Axis::Row, true, &mut ctx);
    ctx.css
}

fn emit_node(tree: &LayoutTree, nid: NodeId, parent_axis: Axis, is_root: bool, ctx: &mut EmitCtx) {
    let Some(node) = tree.node(nid) else { return };
    match node {
        Node::Panel {
            kind, constraints, ..
        } => {
            write_panel_rule(
                kind,
                constraints,
                parent_axis,
                ctx.transitions,
                &mut ctx.css,
            );
        }
        Node::Row { children, .. } | Node::Col { children, .. } => {
            emit_flex_container(tree, node, children, parent_axis, is_root, ctx);
        }
        Node::Grid {
            columns,
            gap,
            auto_rows,
            children,
        } => {
            write_container_selector(is_root, &mut ctx.counter, &mut ctx.css);
            write_native_grid_rule(*columns, *gap, *auto_rows, is_root, &mut ctx.css);
            inject_root_extras(is_root, ctx);
            emit_native_grid_children(tree, children, ctx);
        }
        Node::TaffyPassthrough { style, children }
            if is_scrollable_container(style, tree, children) =>
        {
            write_container_selector(is_root, &mut ctx.counter, &mut ctx.css);
            let axis = scroll_axis(style);
            write_scrollable_rule(axis, is_root, &mut ctx.css);
            inject_root_extras(is_root, ctx);
            emit_scrollable_children(tree, children, axis, ctx);
        }
        Node::GridItemWrapper { .. } => {}
        Node::TaffyPassthrough { children, .. } => {
            write_container_selector(is_root, &mut ctx.counter, &mut ctx.css);
            write_passthrough_rule(is_root, &mut ctx.css);
            inject_root_extras(is_root, ctx);
            emit_children(tree, children, parent_axis, ctx);
        }
    }
}

/// Append root-only properties. Called after the rule body is written but
/// before the closing ` }\n`. The rule writers end with ` }\n`, so we replace
/// the last 3 bytes with the extras and re-close.
///
/// This approach keeps each rule writer self-contained while allowing the root
/// to inject additional properties.
fn inject_root_extras(is_root: bool, ctx: &mut EmitCtx) {
    let extra = match (is_root, ctx.root_position_relative, ctx.transitions) {
        (true, true, true) => " position: relative; --pane-transition: 0.2s ease;",
        (true, true, false) => " position: relative;",
        (true, false, true) => " --pane-transition: 0.2s ease;",
        _ => return,
    };
    // Rule writers close with " }\n" (3 bytes). Reopen, append, re-close.
    debug_assert!(ctx.css.ends_with(" }\n"));
    ctx.css.truncate(ctx.css.len() - 3);
    ctx.css.push_str(extra);
    ctx.css.push_str(" }\n");
}

fn emit_children(tree: &LayoutTree, children: &[NodeId], axis: Axis, ctx: &mut EmitCtx) {
    for &child_id in children {
        emit_node(tree, child_id, axis, false, ctx);
    }
}

/// Write the selector portion of a container rule directly into the buffer.
fn write_container_selector(is_root: bool, counter: &mut u32, css: &mut String) {
    match is_root {
        true => css.push_str("[data-pane-root]"),
        false => {
            *counter += 1;
            let _ = write!(css, "[data-pane-node=\"{}\"]", counter);
        }
    }
}

fn emit_flex_container(
    tree: &LayoutTree,
    node: &Node,
    children: &[NodeId],
    parent_axis: Axis,
    is_root: bool,
    ctx: &mut EmitCtx,
) {
    let (direction, gap, constraints, axis) = match node {
        Node::Row {
            gap, constraints, ..
        } => ("row", *gap, constraints.as_ref(), Axis::Row),
        Node::Col {
            gap, constraints, ..
        } => ("column", *gap, constraints.as_ref(), Axis::Col),
        _ => return,
    };
    write_container_selector(is_root, &mut ctx.counter, &mut ctx.css);
    let _ = write!(
        ctx.css,
        " {{ display: flex; flex-direction: {direction}; gap: {gap}px;"
    );
    write_container_flex(is_root, constraints, parent_axis, &mut ctx.css);
    inject_root_extras(is_root, ctx);
    emit_children(tree, children, axis, ctx);
}

fn write_container_flex(
    is_root: bool,
    constraints: Option<&Constraints>,
    parent_axis: Axis,
    css: &mut String,
) {
    match (is_root, constraints) {
        (true, _) => {}
        (false, Some(c)) => write_constraint_flex(c, parent_axis, css),
        (false, None) => {
            css.push_str(" flex-grow: 1; flex-basis: 0px; flex-shrink: 1;");
        }
    }
    css.push_str(" }\n");
}

fn write_panel_rule(
    kind: &str,
    constraints: &Constraints,
    parent_axis: Axis,
    transitions: bool,
    css: &mut String,
) {
    write_attr_selector("data-pane", kind, css);
    css.push_str(" { ");
    write_flex_sizing(constraints, parent_axis, css);
    write_min_max(constraints, parent_axis, css);
    write_cross_axis_constraints(constraints, css);
    write_align_self(constraints, css);
    write_transition(transitions, css);
    css.push_str(" }\n");
}

/// Write CSS Grid properties from native `Node::Grid` fields.
fn write_native_grid_rule(
    columns: GridColumnMode,
    gap: f32,
    auto_rows: bool,
    is_root: bool,
    css: &mut String,
) {
    css.push_str(" { display: grid;");
    match columns {
        GridColumnMode::Fixed(cols) => {
            let _ = write!(css, " grid-template-columns: repeat({cols}, 1fr);");
        }
        GridColumnMode::AutoFill { min_width } => {
            let _ = write!(
                css,
                " grid-template-columns: repeat(auto-fill, minmax({min_width}px, 1fr));"
            );
        }
        GridColumnMode::AutoFit { min_width } => {
            let _ = write!(
                css,
                " grid-template-columns: repeat(auto-fit, minmax({min_width}px, 1fr));"
            );
        }
    }
    match auto_rows {
        true => css.push_str(" grid-auto-rows: auto;"),
        false => css.push_str(" grid-auto-rows: 1fr;"),
    }
    if gap > 0.0 {
        let _ = write!(css, " gap: {gap}px;");
    }
    if !is_root {
        css.push_str(" flex-grow: 1; flex-basis: 0px;");
    }
    css.push_str(" }\n");
}

/// Emit children of a native `Node::Grid`.
fn emit_native_grid_children(tree: &LayoutTree, children: &[NodeId], ctx: &mut EmitCtx) {
    for &child_id in children {
        match tree.node(child_id) {
            Some(Node::GridItemWrapper { span, child }) => {
                write_container_selector(false, &mut ctx.counter, &mut ctx.css);
                write_native_grid_card_rule(*span, &mut ctx.css);
                emit_grid_wrapper_child(tree, *child, ctx.transitions, &mut ctx.css);
            }
            Some(Node::Panel {
                kind, constraints, ..
            }) => {
                write_panel_rule(kind, constraints, Axis::Row, ctx.transitions, &mut ctx.css);
            }
            _ => {}
        }
    }
}

/// Emit the inner child of a `GridItemWrapper` (typically a panel).
fn emit_grid_wrapper_child(tree: &LayoutTree, child: NodeId, transitions: bool, css: &mut String) {
    let Some(Node::Panel {
        kind, constraints, ..
    }) = tree.node(child)
    else {
        return;
    };
    write_panel_rule(kind, constraints, Axis::Row, transitions, css);
}

/// Write CSS for a native grid item wrapper with span from `CardSpan`.
fn write_native_grid_card_rule(span: CardSpan, css: &mut String) {
    css.push_str(" { display: flex;");
    match span {
        CardSpan::FullWidth => css.push_str(" grid-column: 1 / -1;"),
        CardSpan::Columns(n) if n > 1 => {
            let _ = write!(css, " grid-column: span {n};");
        }
        CardSpan::Columns(_) => {}
    }
    css.push_str(" flex-grow: 1; flex-basis: 0px; flex-shrink: 1; }\n");
}

fn write_passthrough_rule(is_root: bool, css: &mut String) {
    css.push_str(" { display: flex;");
    write_container_flex(is_root, None, Axis::Row, css);
}

/// A TaffyPassthrough is scrollable when it uses flex-row, nowrap, and all
/// children are panels with a fixed width.
fn is_scrollable_container(style: &taffy::Style, tree: &LayoutTree, children: &[NodeId]) -> bool {
    style.display == taffy::Display::Flex
        && matches!(
            style.flex_direction,
            taffy::FlexDirection::Row | taffy::FlexDirection::Column
        )
        && style.flex_wrap == taffy::FlexWrap::NoWrap
        && !children.is_empty()
        && children.iter().all(|&nid| {
            matches!(
                tree.node(nid),
                Some(Node::Panel { constraints, .. }) if constraints.fixed.is_some()
            )
        })
}

#[derive(Clone, Copy)]
enum ScrollAxis {
    X,
    Y,
}

fn scroll_axis(style: &taffy::Style) -> ScrollAxis {
    match style.flex_direction {
        taffy::FlexDirection::Column | taffy::FlexDirection::ColumnReverse => ScrollAxis::Y,
        _ => ScrollAxis::X,
    }
}

fn write_scrollable_rule(axis: ScrollAxis, is_root: bool, css: &mut String) {
    css.push_str(" { display: flex;");
    match axis {
        ScrollAxis::X => {
            css.push_str(" flex-direction: row;");
            css.push_str(" overflow-x: auto; scroll-snap-type: x mandatory;");
        }
        ScrollAxis::Y => {
            css.push_str(" flex-direction: column;");
            css.push_str(" overflow-y: auto; scroll-snap-type: y mandatory;");
        }
    }
    css.push_str(" overscroll-behavior: contain;");
    write_container_flex(is_root, None, Axis::Row, css);
}

fn emit_scrollable_children(
    tree: &LayoutTree,
    children: &[NodeId],
    axis: ScrollAxis,
    ctx: &mut EmitCtx,
) {
    let parent_axis = match axis {
        ScrollAxis::X => Axis::Row,
        ScrollAxis::Y => Axis::Col,
    };
    for &child_id in children {
        let Some(Node::Panel {
            kind, constraints, ..
        }) = tree.node(child_id)
        else {
            continue;
        };
        write_panel_rule(
            kind,
            constraints,
            parent_axis,
            ctx.transitions,
            &mut ctx.css,
        );
        // Insert scroll-snap-align before the closing " }\n".
        debug_assert!(ctx.css.ends_with(" }\n"));
        ctx.css.truncate(ctx.css.len() - 3);
        ctx.css.push_str(" scroll-snap-align: start; }\n");
    }
}

/// Emit flex constraints for a child container with explicit constraints.
fn write_constraint_flex(constraints: &Constraints, parent_axis: Axis, css: &mut String) {
    write_flex_sizing(constraints, parent_axis, css);
    write_min_max(constraints, parent_axis, css);
    write_cross_axis_constraints(constraints, css);
    write_align_self(constraints, css);
}

fn write_flex_sizing(constraints: &Constraints, parent_axis: Axis, css: &mut String) {
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
    write_size_mode(constraints.size_mode, parent_axis, css);
}

fn write_size_mode(size_mode: Option<SizeMode>, parent_axis: Axis, css: &mut String) {
    let Some(mode) = size_mode else { return };
    let prop = match parent_axis {
        Axis::Row => "width",
        Axis::Col => "height",
    };
    match mode {
        SizeMode::MinContent => {
            let _ = write!(css, " flex-basis: min-content; {prop}: min-content;");
        }
        SizeMode::MaxContent => {
            let _ = write!(css, " flex-basis: max-content; {prop}: max-content;");
        }
        SizeMode::FitContent(v) => {
            let _ = write!(
                css,
                " flex-basis: fit-content({v}px); {prop}: fit-content({v}px);"
            );
        }
    }
}

/// Emit CSS with `@media` wrappers for adaptive breakpoints.
///
/// Each entry is `(min_width_px, layout)`. Breakpoints must be strictly increasing
/// by `min_width`. The first breakpoint gets only a max-width query, the last gets
/// only a min-width query, and middle breakpoints get both.
pub fn emit_adaptive(breakpoints: &[(u32, &Layout)]) -> Result<String, AdaptiveCssError> {
    validate_breakpoints(breakpoints)?;
    let mut css = String::new();
    let len = breakpoints.len();
    for (i, (min_width, layout)) in breakpoints.iter().enumerate() {
        let inner = emit_tree(layout, false, false);
        match (i, i + 1 < len) {
            (0, true) => {
                let next_min = breakpoints[i + 1].0;
                let _ = write!(
                    css,
                    "@media (max-width: {}px) {{\n{inner}}}\n",
                    next_min.saturating_sub(1)
                );
            }
            (0, false) => {
                css.push_str(&inner);
            }
            (_, true) => {
                let next_min = breakpoints[i + 1].0;
                let _ = write!(
                    css,
                    "@media (min-width: {min_width}px) and (max-width: {}px) {{\n{inner}}}\n",
                    next_min.saturating_sub(1)
                );
            }
            (_, false) => {
                let _ = write!(css, "@media (min-width: {min_width}px) {{\n{inner}}}\n");
            }
        }
    }
    Ok(css)
}

fn validate_breakpoints(breakpoints: &[(u32, &Layout)]) -> Result<(), AdaptiveCssError> {
    for (index, pair) in breakpoints.windows(2).enumerate() {
        let previous_width = pair[0].0;
        let width = pair[1].0;
        if width <= previous_width {
            return Err(AdaptiveCssError::UnsortedOrDuplicate {
                index: index + 1,
                previous_width,
                width,
            });
        }
    }

    Ok(())
}

fn write_min_max(constraints: &Constraints, axis: Axis, css: &mut String) {
    let (min_prop, max_prop) = match axis {
        Axis::Row => ("min-width", "max-width"),
        Axis::Col => ("min-height", "max-height"),
    };
    if let Some(min) = constraints.min {
        let _ = write!(css, " {min_prop}: {min}px;");
    }
    if let Some(max) = constraints.max {
        let _ = write!(css, " {max_prop}: {max}px;");
    }
}

fn write_cross_axis_constraints(constraints: &Constraints, css: &mut String) {
    if let Some(v) = constraints.min_width {
        let _ = write!(css, " min-width: {v}px;");
    }
    if let Some(v) = constraints.max_width {
        let _ = write!(css, " max-width: {v}px;");
    }
    if let Some(v) = constraints.min_height {
        let _ = write!(css, " min-height: {v}px;");
    }
    if let Some(v) = constraints.max_height {
        let _ = write!(css, " max-height: {v}px;");
    }
}

fn write_align_self(constraints: &Constraints, css: &mut String) {
    let Some(align) = constraints.align else {
        return;
    };
    let value = match align {
        Align::Start => "start",
        Align::Center => "center",
        Align::End => "end",
        Align::Stretch => return,
    };
    let _ = write!(css, " align-self: {value};");
}

fn write_transition(transitions: bool, css: &mut String) {
    match transitions {
        true => css.push_str(concat!(
            " transition: left var(--pane-transition),",
            " top var(--pane-transition),",
            " width var(--pane-transition),",
            " height var(--pane-transition);"
        )),
        false => {}
    }
}

fn write_overlay_rule(def: &OverlayDef, z_index: usize, css: &mut String) {
    let kind = def.kind();
    write_panel_anchor_container(def.anchor(), css);
    write_attr_selector("data-pane-overlay", kind, css);
    css.push_str(" { position: absolute;");
    let _ = write!(css, " z-index: {z_index};");
    write_overlay_anchor(def.anchor(), css);
    write_overlay_extent("width", def.width(), css);
    write_overlay_extent("height", def.height(), css);
    css.push_str(" }\n");
}

/// Emit `position: relative` on the anchor panel when anchored to a panel.
fn write_panel_anchor_container(anchor: &OverlayAnchor, css: &mut String) {
    match anchor {
        OverlayAnchor::Panel { kind, .. } => {
            write_attr_selector("data-pane", kind, css);
            css.push_str(" { position: relative; }\n");
        }
        OverlayAnchor::PanelAnchor { key, .. } => {
            write_attr_selector("data-pane-key", key, css);
            css.push_str(" { position: relative; }\n");
        }
        OverlayAnchor::Viewport { .. } => {}
    }
}

fn write_attr_selector(attr: &str, value: impl Display, css: &mut String) {
    let _ = write!(css, "[{attr}=\"");
    write_css_string(&value.to_string(), css);
    css.push_str("\"]");
}

fn write_css_string(value: &str, css: &mut String) {
    for ch in value.chars() {
        match ch {
            '\\' | '"' => {
                css.push('\\');
                css.push(ch);
            }
            '\0' => css.push_str("\\0 "),
            '\n' => css.push_str("\\a "),
            '\r' => css.push_str("\\d "),
            '\u{000C}' => css.push_str("\\c "),
            _ => css.push(ch),
        }
    }
}

fn write_overlay_anchor(anchor: &OverlayAnchor, css: &mut String) {
    match anchor {
        OverlayAnchor::Viewport {
            h,
            v,
            margin_x,
            margin_y,
        } => write_viewport_anchor(*h, *v, *margin_x, *margin_y, css),
        OverlayAnchor::Panel {
            h,
            v,
            offset_x,
            offset_y,
            ..
        }
        | OverlayAnchor::PanelAnchor {
            h,
            v,
            offset_x,
            offset_y,
            ..
        } => write_viewport_anchor(*h, *v, *offset_x, *offset_y, css),
    }
}

fn write_viewport_anchor(h: HAlign, v: VAlign, margin_x: f32, margin_y: f32, css: &mut String) {
    let needs_translate_x = matches!(h, HAlign::Center);
    let needs_translate_y = matches!(v, VAlign::Center);

    match h {
        HAlign::Left => {
            let _ = write!(css, " left: {margin_x}px;");
        }
        HAlign::Center => {
            css.push_str(" left: 50%;");
        }
        HAlign::Right => {
            let _ = write!(css, " right: {margin_x}px;");
        }
    }

    match v {
        VAlign::Top => {
            let _ = write!(css, " top: {margin_y}px;");
        }
        VAlign::Center => {
            css.push_str(" top: 50%;");
        }
        VAlign::Bottom => {
            let _ = write!(css, " bottom: {margin_y}px;");
        }
    }

    match (needs_translate_x, needs_translate_y) {
        (true, true) => css.push_str(" transform: translate(-50%, -50%);"),
        (true, false) => css.push_str(" transform: translateX(-50%);"),
        (false, true) => css.push_str(" transform: translateY(-50%);"),
        (false, false) => {}
    }
}

fn write_overlay_extent(prop: &str, extent: &panes::OverlayExtent, css: &mut String) {
    match extent.value {
        ExtentValue::Fixed(v) => {
            let _ = write!(css, " {prop}: {v}px;");
        }
        ExtentValue::Percent(pct) => {
            let _ = write!(css, " {prop}: {pct}%;");
        }
        ExtentValue::Full => {
            let _ = write!(css, " {prop}: 100%;");
        }
    }
    if let Some(min) = extent.min {
        let _ = write!(css, " min-{prop}: {min}px;");
    }
    if let Some(max) = extent.max {
        let _ = write!(css, " max-{prop}: {max}px;");
    }
}
