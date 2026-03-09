use serde::Deserialize;

use crate::layout::Layout;

macro_rules! apply_opt {
    ($preset:expr, $def:expr, $($field:ident),+ $(,)?) => {
        $(if let Some(v) = $def.$field {
            $preset = $preset.$field(v);
        })+
    };
}

/// Errors arising from TOML configuration parsing.
#[non_exhaustive]
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
    tab_height: Option<f32>,
    title_height: Option<f32>,
    sidebar_width: Option<f32>,
    header_height: Option<f32>,
    footer_height: Option<f32>,
    // Custom tree
    root: Option<TreeNodeDef>,
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
    span: usize,
}

fn default_span() -> usize {
    1
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
    gap: Option<f32>,
    #[serde(default)]
    children: Vec<TreeNodeDef>,
}

/// Parse a TOML string into a `Layout`.
pub(crate) fn parse(input: &str) -> Result<Layout, TomlError> {
    let doc: TomlDocument = toml::from_str(input)?;
    build_from_def(doc.layout)
}

fn into_toml_error(err: crate::error::PaneError) -> TomlError {
    TomlError::InvalidValue {
        field: "layout".into(),
        reason: err.to_string().into(),
    }
}

macro_rules! build_preset {
    ($def:expr, $ctor:expr $(, $field:ident)*) => {{
        let panels = require_panels_strings(&$def)?;
        let mut preset = $ctor(panels.iter().map(Box::as_ref));
        apply_opt!(preset, $def, $($field),*);
        preset.build().map_err(into_toml_error)
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
        "tabbed" => build_preset!(def, Layout::tabbed, active, tab_height, gap),
        "stacked" => build_preset!(def, Layout::stacked, active, title_height, gap),
        "columns" => build_columns(def),
        "grid" => build_grid(def),
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

fn build_columns(def: LayoutDef) -> Result<Layout, TomlError> {
    let cols = def
        .columns
        .ok_or(TomlError::MissingField("columns".into()))?;
    let panels = require_panels_strings(&def)?;
    let mut preset = Layout::columns(cols, panels.iter().map(Box::as_ref));
    apply_opt!(preset, def, gap);
    preset.build().map_err(into_toml_error)
}

fn build_grid(def: LayoutDef) -> Result<Layout, TomlError> {
    let cols = def
        .columns
        .ok_or(TomlError::MissingField("columns".into()))?;
    let panels = require_panels_strings(&def)?;
    let mut preset = Layout::grid(cols, panels.iter().map(Box::as_ref));
    apply_opt!(preset, def, gap);
    preset.build().map_err(into_toml_error)
}

fn build_sidebar(def: LayoutDef) -> Result<Layout, TomlError> {
    let sidebar = require_field(&def.sidebar, "sidebar")?;
    let content = require_field(&def.content, "content")?;
    let mut preset = Layout::sidebar(sidebar, content);
    apply_opt!(preset, def, sidebar_width, gap);
    preset.build().map_err(into_toml_error)
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
    preset.build().map_err(into_toml_error)
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
    preset.build().map_err(into_toml_error)
}

fn build_dashboard(def: LayoutDef) -> Result<Layout, TomlError> {
    let cards = match &def.panels {
        Some(PanelsList::Cards(cards)) => cards
            .iter()
            .map(|c| (c.kind.clone(), c.span))
            .collect::<Vec<_>>(),
        Some(PanelsList::Strings(strings)) => strings
            .iter()
            .map(|s| (s.clone(), 1usize))
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
    let mut preset = Layout::dashboard(cards);
    apply_opt!(preset, def, columns, gap);
    preset.build().map_err(into_toml_error)
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
    }
    .map_err(into_toml_error)?;
    builder.build().map_err(into_toml_error)
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
                crate::error::TreeError::Dynamic(
                    "node has both 'kind' and 'type'; use one or the other".into(),
                ),
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
                crate::error::TreeError::Dynamic(
                    format!("unknown node type '{other}'; expected 'row' or 'col'").into(),
                ),
            ));
        }
        (None, None) => {
            ctx.set_error(crate::error::PaneError::InvalidTree(
                crate::error::TreeError::Dynamic(
                    "node must have either 'kind' (panel) or 'type' (container)".into(),
                ),
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
    c
}
