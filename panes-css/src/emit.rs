use std::fmt::Write as _;

// `fmt::Write` for `String` is infallible. `let _ =` discards the unused `Result`.

use panes::Direction;
use panes::{
    Align, Constraints, ExtentValue, HAlign, Layout, LayoutTree, Node, NodeId, OverlayAnchor,
    OverlayDef, SizeMode, VAlign,
};

/// Mutable state threaded through recursive CSS emission.
struct EmitCtx {
    css: String,
    counter: u32,
    root_position_relative: bool,
    transitions: bool,
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
            write_panel_rule(
                kind,
                constraints,
                parent_axis,
                ctx.transitions,
                &mut ctx.css,
            );
        }
        Node::Row { gap, children } => {
            emit_flex_container(
                tree,
                children,
                "row",
                *gap,
                Direction::Horizontal,
                is_root,
                ctx,
            );
        }
        Node::Col { gap, children } => {
            emit_flex_container(
                tree,
                children,
                "column",
                *gap,
                Direction::Vertical,
                is_root,
                ctx,
            );
        }
        Node::TaffyPassthrough { style, children } if style.display == taffy::Display::Grid => {
            write_container_selector(is_root, &mut ctx.counter, &mut ctx.css);
            write_grid_rule(style, is_root, &mut ctx.css);
            inject_root_extras(is_root, ctx);
            emit_grid_children(tree, children, ctx);
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

fn emit_children(tree: &LayoutTree, children: &[NodeId], axis: Direction, ctx: &mut EmitCtx) {
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
    children: &[NodeId],
    direction: &str,
    gap: f32,
    axis: Direction,
    is_root: bool,
    ctx: &mut EmitCtx,
) {
    write_container_selector(is_root, &mut ctx.counter, &mut ctx.css);
    let _ = write!(
        ctx.css,
        " {{ display: flex; flex-direction: {direction}; gap: {gap}px;"
    );
    write_container_flex(is_root, &mut ctx.css);
    inject_root_extras(is_root, ctx);
    emit_children(tree, children, axis, ctx);
}

fn write_container_flex(is_root: bool, css: &mut String) {
    if !is_root {
        css.push_str(" flex-grow: 1; flex-basis: 0px; flex-shrink: 1;");
    }
    css.push_str(" }\n");
}

fn write_panel_rule(
    kind: &str,
    constraints: &Constraints,
    parent_axis: Direction,
    transitions: bool,
    css: &mut String,
) {
    let _ = write!(css, "[data-pane=\"{kind}\"] {{ ");
    write_flex_sizing(constraints, parent_axis, css);
    write_min_max(constraints, parent_axis, css);
    write_cross_axis_constraints(constraints, css);
    write_align_self(constraints, css);
    write_transition(transitions, css);
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

fn write_grid_rule(style: &taffy::Style, is_root: bool, css: &mut String) {
    css.push_str(" { display: grid;");
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
    match style.grid_auto_rows.is_empty() {
        true => {}
        false if is_auto_rows(&style.grid_auto_rows) => {
            css.push_str(" grid-auto-rows: auto;");
        }
        false => css.push_str(" grid-auto-rows: 1fr;"),
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

fn emit_grid_children(tree: &LayoutTree, children: &[NodeId], ctx: &mut EmitCtx) {
    for &child_id in children {
        match tree.node(child_id) {
            Some(Node::TaffyPassthrough { style, .. }) => {
                write_container_selector(false, &mut ctx.counter, &mut ctx.css);
                write_grid_card_rule(style, &mut ctx.css);
                emit_grid_card_panels(tree, child_id, ctx.transitions, &mut ctx.css);
            }
            Some(Node::Panel {
                kind, constraints, ..
            }) => {
                write_panel_rule(
                    kind,
                    constraints,
                    Direction::Horizontal,
                    ctx.transitions,
                    &mut ctx.css,
                );
            }
            _ => {}
        }
    }
}

fn write_grid_card_rule(style: &taffy::Style, css: &mut String) {
    css.push_str(" { display: flex;");
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

fn emit_grid_card_panels(tree: &LayoutTree, card_id: NodeId, transitions: bool, css: &mut String) {
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
        write_panel_rule(kind, constraints, Direction::Horizontal, transitions, css);
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

fn is_auto_rows(tracks: &[taffy::style::TrackSizingFunction]) -> bool {
    let auto_track = taffy::prelude::minmax(
        taffy::style::MinTrackSizingFunction::auto(),
        taffy::style::MaxTrackSizingFunction::auto(),
    );
    matches!(tracks.first(), Some(t) if *t == auto_track)
}

fn write_passthrough_rule(is_root: bool, css: &mut String) {
    css.push_str(" { display: flex;");
    write_container_flex(is_root, css);
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
    write_container_flex(is_root, css);
}

fn emit_scrollable_children(
    tree: &LayoutTree,
    children: &[NodeId],
    axis: ScrollAxis,
    ctx: &mut EmitCtx,
) {
    let parent_axis = match axis {
        ScrollAxis::X => Direction::Horizontal,
        ScrollAxis::Y => Direction::Vertical,
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

fn write_flex_sizing(constraints: &Constraints, parent_axis: Direction, css: &mut String) {
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

fn write_size_mode(size_mode: Option<SizeMode>, parent_axis: Direction, css: &mut String) {
    let Some(mode) = size_mode else { return };
    let prop = match parent_axis {
        Direction::Horizontal => "width",
        Direction::Vertical => "height",
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
/// Each entry is `(min_width_px, layout)`. Breakpoints must be sorted ascending
/// by min_width. The first breakpoint gets only a max-width query, the last gets
/// only a min-width query, and middle breakpoints get both.
pub fn emit_adaptive(breakpoints: &[(u32, &Layout)]) -> String {
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
    let _ = write!(css, "[data-pane-overlay=\"{kind}\"] {{ position: absolute;");
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
            let _ = writeln!(css, "[data-pane=\"{kind}\"] {{ position: relative; }}");
        }
        OverlayAnchor::Viewport { .. } => {}
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
