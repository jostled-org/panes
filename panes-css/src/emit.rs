use std::fmt::Write as _;

use crate::error::AdaptiveCssError;
use crate::escape::write_attr_selector;
use crate::selector::PaneSelector;
use crate::skeleton::{Context, PaneElement, ScrollAxis, Shape, skeleton};
use panes::{
    Align, Axis, CardSpan, ColumnSpan, Constraints, ExtentValue, GridColumnMode, HAlign, Layout,
    LayoutTree, OverlayAnchor, OverlayDef, SizeMode, VAlign,
};

struct EmitCtx {
    css: String,
    root_position_relative: bool,
    transitions: bool,
}

/// Emit a CSS string from a `Layout` or `LayoutTree`.
///
/// The browser acts as the layout solver via flexbox properties.
/// Panels use `[data-pane="kind"]` selectors, containers use
/// `[data-pane-node="N"]`, and the root uses `[data-pane-root]`.
/// [`skeleton`](crate::skeleton()) lists the elements these rules target.
///
/// Pass `runtime.tree()` to emit CSS for a runtime's current tree.
pub fn emit<T: AsRef<LayoutTree> + ?Sized>(layout: &T) -> String {
    emit_tree(layout.as_ref(), false, false)
}

/// Emit CSS including absolute-positioned overlay rules.
///
/// The root selector gets `position: relative` so overlays can anchor
/// against it. Each `OverlayDef` produces a `[data-pane-overlay="kind"]`
/// rule with positioning, size, and z-index.
pub fn emit_with_overlays<T: AsRef<LayoutTree> + ?Sized>(
    layout: &T,
    overlays: &[OverlayDef],
) -> String {
    emit_with_options(layout.as_ref(), overlays, false)
}

/// Emit CSS with transition properties on all panel selectors.
///
/// The root gets a `--pane-transition` custom property. Each panel selector
/// gets a `transition` shorthand referencing that variable for position and
/// size properties.
pub fn emit_with_transitions<T: AsRef<LayoutTree> + ?Sized>(layout: &T) -> String {
    emit_tree(layout.as_ref(), false, true)
}

/// Emit CSS with both overlay positioning and transition properties.
pub fn emit_full<T: AsRef<LayoutTree> + ?Sized>(layout: &T, overlays: &[OverlayDef]) -> String {
    emit_with_options(layout.as_ref(), overlays, true)
}

fn emit_with_options(tree: &LayoutTree, overlays: &[OverlayDef], transitions: bool) -> String {
    let mut css = emit_tree(tree, !overlays.is_empty(), transitions);
    for (i, def) in overlays.iter().enumerate() {
        write_overlay_rule(def, i + 1, &mut css);
    }
    css
}

fn emit_tree(tree: &LayoutTree, root_position_relative: bool, transitions: bool) -> String {
    let mut ctx = EmitCtx {
        css: String::with_capacity(tree.node_count() * 80),
        root_position_relative,
        transitions,
    };
    skeleton(tree)
        .iter()
        .for_each(|element| write_element(element, &mut ctx));
    ctx.css
}

/// Write the rule for one skeleton element.
fn write_element(element: &PaneElement<'_>, ctx: &mut EmitCtx) {
    let is_root = matches!(element.selector, PaneSelector::Root);
    let parent_axis = element.context.parent_axis();
    let css = &mut ctx.css;
    let _ = write!(css, "{}", element.selector);
    match element.shape {
        Shape::Panel(constraints) => {
            write_panel_body(
                constraints,
                parent_axis,
                element.context,
                ctx.transitions,
                css,
            );
        }
        Shape::Flex {
            axis,
            gap,
            constraints,
        } => write_flex_rule(axis, gap, constraints, parent_axis, is_root, css),
        Shape::Grid {
            columns,
            gap,
            auto_rows,
        } => write_native_grid_rule(columns, gap, auto_rows, is_root, css),
        Shape::GridCard(span) => write_native_grid_card_rule(span, css),
        Shape::Scroll(axis) => write_scrollable_rule(axis, is_root, css),
        Shape::Passthrough => write_passthrough_rule(is_root, css),
    }
    inject_root_extras(is_root, ctx);
}

/// Append root-only properties. Called after the rule body is written.
/// The rule writers end with ` }\n`, so we replace the last 3 bytes with the
/// extras and re-close.
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

fn write_flex_rule(
    axis: Axis,
    gap: f32,
    constraints: Option<&Constraints>,
    parent_axis: Axis,
    is_root: bool,
    css: &mut String,
) {
    let direction = match axis {
        Axis::Row => "row",
        Axis::Col => "column",
    };
    let _ = write!(
        css,
        " {{ display: flex; flex-direction: {direction}; gap: {gap}px;"
    );
    write_container_flex(is_root, constraints, parent_axis, css);
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

fn write_panel_body(
    constraints: &Constraints,
    parent_axis: Axis,
    context: Context,
    transitions: bool,
    css: &mut String,
) {
    css.push_str(" { ");
    write_flex_sizing(constraints, parent_axis, css);
    write_min_max(constraints, parent_axis, css);
    write_cross_axis_constraints(constraints, css);
    write_align_self(constraints, css);
    write_transition(transitions, css);
    if matches!(context, Context::Scroll(_)) {
        css.push_str(" scroll-snap-align: start;");
    }
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

/// Write CSS for a native grid item wrapper with span from `CardSpan`.
fn write_native_grid_card_rule(span: CardSpan, css: &mut String) {
    css.push_str(" { display: flex;");
    match span.columns {
        ColumnSpan::FullWidth => css.push_str(" grid-column: 1 / -1;"),
        ColumnSpan::Count(n) if n > 1 => {
            let _ = write!(css, " grid-column: span {n};");
        }
        ColumnSpan::Count(_) => {}
    }
    if span.rows > 1 {
        let _ = write!(css, " grid-row: span {};", span.rows);
    }
    css.push_str(" flex-grow: 1; flex-basis: 0px; flex-shrink: 1; }\n");
}

fn write_passthrough_rule(is_root: bool, css: &mut String) {
    css.push_str(" { display: flex;");
    write_container_flex(is_root, None, Axis::Row, css);
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
        let inner = emit_tree(layout.tree(), false, false);
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
    let _ = write_attr_selector("data-pane-overlay", kind, css);
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
            let _ = write_attr_selector("data-pane", kind, css);
            css.push_str(" { position: relative; }\n");
        }
        OverlayAnchor::PanelAnchor { key, .. } => {
            let _ = write_attr_selector("data-pane-key", key, css);
            css.push_str(" { position: relative; }\n");
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
