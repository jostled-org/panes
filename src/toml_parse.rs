use std::sync::Arc;

use serde::Deserialize;

use crate::layout::Layout;
use crate::runtime::LayoutRuntime;

macro_rules! apply_opt {
    ($preset:expr, $def:expr, $($field:ident),+ $(,)?) => {
        $(if let Some(v) = $def.$field {
            $preset = $preset.$field(v);
        })+
    };
}

/// Errors arising from TOML configuration parsing.

#[derive(Debug, thiserror::Error)]
pub enum TomlError {
    /// TOML deserialization failed.
    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),

    /// The strategy name is not recognized.
    #[error("unknown strategy: {0}")]
    UnknownStrategy(Box<str>),

    /// A required field is absent.
    #[error("missing field: {0}")]
    MissingField(Box<str>),

    /// A field value is out of range or otherwise invalid.
    #[error("invalid value for field '{field}': {reason}")]
    InvalidValue {
        /// The field name.
        field: Box<str>,
        /// Why the value is invalid.
        reason: Box<str>,
    },

    /// Layout construction failed.
    #[error("layout error: {0}")]
    LayoutError(#[from] crate::error::PaneError),

    /// File I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Deserialize)]
struct TomlDocument {
    layout: LayoutDef,
}

#[derive(Deserialize)]
struct LayoutDef {
    strategy: Box<str>,
    panels: Option<PanelsList>,
    columns: Option<usize>,
    min_column_width: Option<f32>,
    column_mode: Option<Box<str>>,
    // Named-param fields
    sidebar: Option<Box<str>>,
    content: Option<Box<str>>,
    first: Option<Box<str>>,
    second: Option<Box<str>>,
    direction: Option<Box<str>>,
    header: Option<Box<str>>,
    footer: Option<Box<str>>,
    left: Option<Box<str>>,
    main: Option<Box<str>>,
    right: Option<Box<str>>,
    // Common optional tuning params
    gap: Option<f32>,
    master_ratio: Option<f32>,
    ratio: Option<f32>,
    active: Option<usize>,
    bar_height: Option<f32>,
    sidebar_width: Option<f32>,
    header_height: Option<f32>,
    footer_height: Option<f32>,
    // Custom tree
    root: Option<TreeNodeDef>,
    // Adaptive breakpoints
    breakpoints: Option<Vec<BreakpointDef>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PanelsList {
    Strings(Vec<Box<str>>),
    Cards(Vec<CardDef>),
}

#[derive(Deserialize)]
struct CardDef {
    kind: Box<str>,
    #[serde(default = "default_span")]
    span: SpanValue,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SpanValue {
    Number(usize),
    String(Box<str>),
}

impl SpanValue {
    fn to_card_span(&self) -> Result<crate::strategy::CardSpan, TomlError> {
        match self {
            Self::Number(n) => Ok(crate::strategy::CardSpan::Columns(*n)),
            Self::String(s) if s.as_ref() == "full-width" => {
                Ok(crate::strategy::CardSpan::FullWidth)
            }
            Self::String(s) => Err(TomlError::InvalidValue {
                field: "span".into(),
                reason: format!("expected a number or 'full-width', got '{s}'").into(),
            }),
        }
    }
}

fn default_span() -> SpanValue {
    SpanValue::Number(1)
}

#[derive(Deserialize)]
struct TreeNodeDef {
    #[serde(rename = "type")]
    node_type: Option<Box<str>>,
    kind: Option<Box<str>>,
    grow: Option<f32>,
    fixed: Option<f32>,
    min: Option<f32>,
    max: Option<f32>,
    min_width: Option<f32>,
    max_width: Option<f32>,
    min_height: Option<f32>,
    max_height: Option<f32>,
    align: Option<crate::panel::Align>,
    size_mode: Option<crate::panel::SizeMode>,
    gap: Option<f32>,
    #[serde(default)]
    children: Vec<TreeNodeDef>,
}

#[derive(Deserialize)]
struct BreakpointDef {
    min_width: u32,
    strategy: Box<str>,
    gap: Option<f32>,
    master_ratio: Option<f32>,
    ratio: Option<f32>,
    columns: Option<usize>,
    min_column_width: Option<f32>,
    column_mode: Option<Box<str>>,
    bar_height: Option<f32>,
    size: Option<usize>,
}

/// Parse a TOML string into a `Layout`.
pub(crate) fn parse(input: &str) -> Result<Layout, TomlError> {
    let doc: TomlDocument = toml::from_str(input)?;
    build_from_def(doc.layout)
}

macro_rules! build_preset {
    ($def:expr, $ctor:expr $(, $field:ident)*) => {{
        let panels = require_panels_strings(&$def)?;
        let mut preset = $ctor(panels.iter().map(Box::as_ref));
        apply_opt!(preset, $def, $($field),*);
        Ok(preset.build()?)
    }};
}

fn build_from_def(def: LayoutDef) -> Result<Layout, TomlError> {
    match def.strategy.as_ref() {
        "master-stack" => build_preset!(def, Layout::master_stack, master_ratio, gap),
        "centered-master" => build_preset!(def, Layout::centered_master, master_ratio, gap),
        "monocle" => build_preset!(def, Layout::monocle, active),
        "scrollable" => build_preset!(def, Layout::scrollable, active, gap),
        "dwindle" => build_preset!(def, Layout::dwindle, ratio, gap),
        "spiral" => build_preset!(def, Layout::spiral, ratio, gap),
        "deck" => build_preset!(def, Layout::deck, master_ratio, active, gap),
        "tabbed" => build_preset!(def, Layout::tabbed, active, bar_height, gap),
        "stacked" => build_preset!(def, Layout::stacked, active, bar_height, gap),
        "columns" | "grid" => build_grid_or_columns(def),
        "sidebar" => build_sidebar(def),
        "split" => build_split(def),
        "holy-grail" => build_holy_grail(def),
        "dashboard" => build_dashboard(def),
        "custom" => build_custom(def),
        other => Err(TomlError::UnknownStrategy(other.into())),
    }
}

// -- Helpers --

fn require_panels_strings(def: &LayoutDef) -> Result<&[Box<str>], TomlError> {
    match &def.panels {
        Some(PanelsList::Strings(v)) if v.is_empty() => Err(TomlError::InvalidValue {
            field: "panels".into(),
            reason: "panels list must not be empty".into(),
        }),
        Some(PanelsList::Strings(v)) => Ok(v),
        Some(PanelsList::Cards(_)) => Err(TomlError::InvalidValue {
            field: "panels".into(),
            reason: "expected string array, got card objects".into(),
        }),
        None => Err(TomlError::MissingField("panels".into())),
    }
}

fn require_field<'a>(value: &'a Option<Box<str>>, name: &str) -> Result<&'a str, TomlError> {
    match value {
        Some(v) => Ok(v.as_ref()),
        None => Err(TomlError::MissingField(name.into())),
    }
}

// -- Count + list / named-param / dashboard builders --

/// Build a grid or columns strategy as a dashboard with span-1 cards.
fn build_grid_or_columns(def: LayoutDef) -> Result<Layout, TomlError> {
    match (def.columns.is_some(), def.min_column_width.is_some()) {
        (false, false) => return Err(TomlError::MissingField("columns".into())),
        _ => {}
    }
    build_dashboard(def)
}

fn build_sidebar(def: LayoutDef) -> Result<Layout, TomlError> {
    let sidebar = require_field(&def.sidebar, "sidebar")?;
    let content = require_field(&def.content, "content")?;
    let mut preset = Layout::sidebar(sidebar, content);
    apply_opt!(preset, def, sidebar_width, gap);
    Ok(preset.build()?)
}

fn build_split(def: LayoutDef) -> Result<Layout, TomlError> {
    let first = require_field(&def.first, "first")?;
    let second = require_field(&def.second, "second")?;
    let mut preset = Layout::split(first, second);
    apply_opt!(preset, def, ratio, gap);
    match def.direction.as_deref() {
        Some("vertical") => {
            preset = preset.vertical();
        }
        Some("horizontal") | None => {}
        Some(other) => {
            return Err(TomlError::InvalidValue {
                field: "direction".into(),
                reason: format!("expected 'vertical' or 'horizontal', got '{other}'").into(),
            });
        }
    }
    Ok(preset.build()?)
}

fn build_holy_grail(def: LayoutDef) -> Result<Layout, TomlError> {
    let header = require_field(&def.header, "header")?;
    let footer = require_field(&def.footer, "footer")?;
    let left = require_field(&def.left, "left")?;
    let main = require_field(&def.main, "main")?;
    let right = require_field(&def.right, "right")?;
    let mut preset = Layout::holy_grail(header, footer, left, main, right);
    apply_opt!(
        preset,
        def,
        header_height,
        footer_height,
        sidebar_width,
        gap
    );
    Ok(preset.build()?)
}

fn build_dashboard(def: LayoutDef) -> Result<Layout, TomlError> {
    let cards = match &def.panels {
        Some(PanelsList::Cards(cards)) => cards
            .iter()
            .map(|c| Ok((Arc::from(&*c.kind), c.span.to_card_span()?)))
            .collect::<Result<Vec<_>, TomlError>>()?,
        Some(PanelsList::Strings(strings)) => strings
            .iter()
            .map(|s| (Arc::from(&**s), crate::strategy::CardSpan::Columns(1)))
            .collect::<Vec<_>>(),
        None => return Err(TomlError::MissingField("panels".into())),
    };
    match cards.is_empty() {
        true => {
            return Err(TomlError::InvalidValue {
                field: "panels".into(),
                reason: "panels list must not be empty".into(),
            });
        }
        false => {}
    }
    match (def.columns.is_some(), def.min_column_width.is_some()) {
        (true, true) => {
            return Err(TomlError::InvalidValue {
                field: "columns".into(),
                reason: "columns and min_column_width are mutually exclusive".into(),
            });
        }
        _ => {}
    }
    let mut preset = Layout::dashboard(cards);
    preset = match (def.min_column_width, def.column_mode.as_deref()) {
        (Some(w), Some("auto-fit")) => preset.auto_fit(w),
        (Some(w), Some("auto-fill") | None) => preset.auto_fill(w),
        (Some(_), Some(other)) => {
            return Err(TomlError::InvalidValue {
                field: "column_mode".into(),
                reason: format!("expected 'auto-fill' or 'auto-fit', got '{other}'").into(),
            });
        }
        (None, Some(_)) => {
            return Err(TomlError::InvalidValue {
                field: "column_mode".into(),
                reason: "column_mode requires min_column_width".into(),
            });
        }
        (None, None) => preset,
    };
    apply_opt!(preset, def, columns, gap);
    Ok(preset.build()?)
}

// -- Custom tree builder (Step 4) --

fn build_custom(def: LayoutDef) -> Result<Layout, TomlError> {
    let root = def.root.ok_or(TomlError::MissingField("root".into()))?;
    let gap_val = root.gap.unwrap_or(0.0);
    let children = root.children;
    let mut builder = crate::builder::LayoutBuilder::new();
    match root.node_type.as_deref() {
        Some("row") => builder.row_gap(gap_val, |ctx| add_tree_children(ctx, children)),
        Some("col") => builder.col_gap(gap_val, |ctx| add_tree_children(ctx, children)),
        Some(other) => {
            return Err(TomlError::InvalidValue {
                field: "root.type".into(),
                reason: format!("expected 'row' or 'col', got '{other}'").into(),
            });
        }
        None => {
            return Err(TomlError::InvalidValue {
                field: "root".into(),
                reason: "root node must have a 'type' field ('row' or 'col')".into(),
            });
        }
    }?;
    Ok(builder.build()?)
}

fn add_tree_children(ctx: &mut crate::ContainerCtx, children: Vec<TreeNodeDef>) {
    for child in children {
        add_tree_node(ctx, child);
    }
}

fn add_tree_node(ctx: &mut crate::ContainerCtx, node: TreeNodeDef) {
    match (node.kind.as_deref(), node.node_type.as_deref()) {
        (Some(_), Some(_)) => {
            ctx.set_error(crate::error::PaneError::InvalidTree(
                crate::error::TreeError::NodeKindAndType,
            ));
        }
        (Some(_), None) if node.grow.is_some() && node.fixed.is_some() => {
            ctx.set_error(crate::error::PaneError::InvalidConstraint(
                crate::error::ConstraintError::GrowFixedExclusive,
            ));
        }
        (Some(kind), None) => {
            let constraints = node_constraints(&node);
            ctx.panel_with(kind, constraints);
        }
        (None, Some("row")) => {
            ctx.row_gap(node.gap.unwrap_or(0.0), |inner| {
                add_tree_children(inner, node.children);
            });
        }
        (None, Some("col")) => {
            ctx.col_gap(node.gap.unwrap_or(0.0), |inner| {
                add_tree_children(inner, node.children);
            });
        }
        (None, Some(other)) => {
            ctx.set_error(crate::error::PaneError::InvalidTree(
                crate::error::TreeError::UnknownNodeType(other.into()),
            ));
        }
        (None, None) => {
            ctx.set_error(crate::error::PaneError::InvalidTree(
                crate::error::TreeError::NodeMissingKindOrType,
            ));
        }
    }
}

fn node_constraints(node: &TreeNodeDef) -> crate::panel::Constraints {
    let mut c = match (node.grow, node.fixed) {
        (Some(g), _) => crate::panel::grow(g),
        (_, Some(f)) => crate::panel::fixed(f),
        (None, None) => crate::panel::grow(1.0),
    };
    if let Some(lo) = node.min {
        c = c.min(lo);
    }
    if let Some(hi) = node.max {
        c = c.max(hi);
    }
    if let Some(v) = node.min_width {
        c = c.min_width(v);
    }
    if let Some(v) = node.max_width {
        c = c.max_width(v);
    }
    if let Some(v) = node.min_height {
        c = c.min_height(v);
    }
    if let Some(v) = node.max_height {
        c = c.max_height(v);
    }
    if let Some(a) = node.align {
        c = c.align(a);
    }
    if let Some(sm) = node.size_mode {
        c = c.size_mode(sm);
    }
    c
}

// -- Adaptive / runtime parsing --

/// Parse a TOML string into a `LayoutRuntime`.
///
/// Supports both single-strategy configs (builds a strategy runtime) and
/// adaptive configs with `[[layout.breakpoints]]` (builds an adaptive runtime).
pub(crate) fn parse_runtime(input: &str) -> Result<LayoutRuntime, TomlError> {
    let doc: TomlDocument = toml::from_str(input)?;
    let def = doc.layout;
    match def.breakpoints.as_ref() {
        Some(_) => build_adaptive(def),
        None => {
            let layout = build_from_def(def)?;
            Ok(LayoutRuntime::new(crate::tree::LayoutTree::from(layout)))
        }
    }
}

fn build_adaptive(def: LayoutDef) -> Result<LayoutRuntime, TomlError> {
    let panels = require_panels_strings(&def)?;
    let panel_arcs: Box<[Arc<str>]> = panels.iter().map(|s| Arc::from(s.as_ref())).collect();
    let bp_defs = def
        .breakpoints
        .ok_or(TomlError::MissingField("breakpoints".into()))?;
    let mut builder = Layout::adaptive(panel_arcs);
    for bp in bp_defs {
        let strategy = breakpoint_def_to_strategy(&bp)?;
        builder = builder.at(bp.min_width, strategy);
    }
    Ok(builder.into_runtime()?)
}

macro_rules! build_bp_strategy {
    ($ctor:expr, $bp:expr $(, $field:ident)*) => {{
        let mut s = $ctor;
        apply_opt!(s, $bp, $($field),*);
        Ok(s.build())
    }};
}

fn breakpoint_def_to_strategy(
    bp: &BreakpointDef,
) -> Result<crate::strategy::builder::Strategy, TomlError> {
    use crate::strategy::builder::Strategy;
    match bp.strategy.as_ref() {
        "master-stack" => build_bp_strategy!(Strategy::master_stack(), bp, master_ratio, gap),
        "centered-master" => build_bp_strategy!(Strategy::centered_master(), bp, master_ratio, gap),
        "deck" => build_bp_strategy!(Strategy::deck(), bp, master_ratio, gap),
        "monocle" => build_bp_strategy!(Strategy::monocle(), bp, bar_height),
        "tabbed" => build_bp_strategy!(Strategy::tabbed(), bp, bar_height),
        "stacked" => build_bp_strategy!(Strategy::stacked(), bp, bar_height),
        "scrollable" => build_bp_strategy!(Strategy::scrollable(), bp, size, gap),
        "dwindle" => build_bp_strategy!(Strategy::dwindle(), bp, ratio, gap),
        "spiral" => build_bp_strategy!(Strategy::spiral(), bp, ratio, gap),
        "columns" | "grid" | "dashboard" => build_bp_dashboard(bp),
        "split" => build_bp_strategy!(Strategy::split(), bp, ratio, gap),
        other => Err(TomlError::UnknownStrategy(other.into())),
    }
}

fn build_bp_dashboard(bp: &BreakpointDef) -> Result<crate::strategy::builder::Strategy, TomlError> {
    use crate::strategy::builder::Strategy;
    let base = Strategy::dashboard();
    let configured = match (bp.min_column_width, bp.column_mode.as_deref(), bp.columns) {
        (Some(w), Some("auto-fit"), _) => base.auto_fit(w),
        (Some(w), _, _) => base.auto_fill(w),
        (None, _, Some(c)) => base.columns(c),
        (None, _, None) => base,
    };
    build_bp_strategy!(configured, bp, gap)
}
