use std::sync::Arc;

use crate::error::{PaneError, TreeError};
use crate::node::{Node, NodeId};
use crate::overlay::{OverlayDef, SnapshotOverlay};
use crate::panel::Constraints;
use crate::strategy::{
    ActivePanelVariant, CardSpan, Direction, GridColumnMode, SlotDef, StrategyKind,
};
use crate::tree::LayoutTree;

/// Serializable snapshot of a [`LayoutRuntime`](crate::runtime::LayoutRuntime)
/// for session persistence.
///
/// Strategy runtimes serialize the recipe (strategy config + panel kinds).
/// Non-strategy runtimes serialize the tree topology.
///
/// # Example
///
/// ```rust
/// # use panes::Layout;
/// let mut rt = Layout::master_stack(["editor", "chat", "status"])
///     .master_ratio(0.6).gap(1.0).into_runtime().unwrap();
/// let snapshot = rt.snapshot().unwrap();
///
/// // Serialize with any serde format:
/// // let json = serde_json::to_string(&snapshot).unwrap();
///
/// // Restore later:
/// let mut rt2 = panes::runtime::LayoutRuntime::from_snapshot(snapshot).unwrap();
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LayoutSnapshot {
    source: SnapshotSource,
    focused: Option<Box<str>>,
    collapsed: Box<[Box<str>]>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "is_box_slice_empty")
    )]
    overlays: Box<[SnapshotOverlay]>,
}

#[cfg(feature = "serde")]
fn is_box_slice_empty<T>(s: &[T]) -> bool {
    s.is_empty()
}

impl LayoutSnapshot {
    pub fn source(&self) -> &SnapshotSource {
        &self.source
    }

    pub fn focused(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    pub fn collapsed(&self) -> &[Box<str>] {
        &self.collapsed
    }

    pub fn overlays(&self) -> &[SnapshotOverlay] {
        &self.overlays
    }

    pub fn into_overlays(self) -> Vec<SnapshotOverlay> {
        self.overlays.into_vec()
    }
}

/// What a snapshot restores from: a strategy recipe, a tree topology,
/// or an adaptive breakpoint set.

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SnapshotSource {
    /// Strategy-based runtime — rebuild from recipe.
    Strategy {
        strategy: StrategyConfig,
        panels: Box<[Box<str>]>,
    },
    /// Non-strategy runtime — rebuild from tree topology.
    Tree { root: SnapshotNode },
    /// Adaptive runtime — rebuild from breakpoints.
    Adaptive {
        breakpoints: Box<[SnapshotBreakpoint]>,
        panels: Box<[Box<str>]>,
        active_index: usize,
    },
}

/// Serializable breakpoint entry for adaptive layouts.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SnapshotBreakpoint {
    pub min_width: u32,
    pub strategy: StrategyConfig,
}

/// Serializable strategy configuration — mirrors [`StrategyKind`] with
/// owned collections instead of `Arc<[T]>`.

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StrategyConfig {
    Sequence {
        direction: Direction,
        gap: f32,
        #[cfg_attr(
            feature = "serde",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        ratio: Option<f32>,
    },
    MasterStack {
        master_ratio: f32,
        gap: f32,
    },
    Deck {
        master_ratio: f32,
        gap: f32,
    },
    CenteredMaster {
        master_ratio: f32,
        gap: f32,
    },
    BinarySplit {
        spiral: bool,
        ratio: f32,
        gap: f32,
    },
    Dashboard {
        columns: GridColumnMode,
        gap: f32,
        spans: Box<[CardSpan]>,
        #[cfg_attr(feature = "serde", serde(default))]
        auto_rows: bool,
    },
    ActivePanel {
        variant: ActivePanelVariant,
        bar_height: f32,
    },
    Window {
        size: usize,
        gap: f32,
    },
    Slotted {
        slots: Box<[SnapshotSlotDef]>,
        gap: f32,
        direction: Direction,
    },
}

/// Serializable slot definition for the Slotted strategy.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SnapshotSlotDef {
    pub kind: Box<str>,
    pub constraints: Constraints,
}

/// Recursive tree node for serializing non-strategy layouts.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SnapshotNode {
    Panel {
        kind: Box<str>,
        constraints: Constraints,
    },
    Row {
        gap: f32,
        children: Box<[SnapshotNode]>,
    },
    Col {
        gap: f32,
        children: Box<[SnapshotNode]>,
    },
}

// ---------------------------------------------------------------------------
// StrategyKind ↔ StrategyConfig conversions
// ---------------------------------------------------------------------------

/// Generate bidirectional `From` impls between `StrategyKind` and `StrategyConfig`.
///
/// Copy-only variants list fields for automatic `*field` copying.
/// Custom variants provide explicit conversion bodies for each direction.
macro_rules! strategy_convert {
    (
        // Copy-only variants: fields are all Copy, just dereference.
        copy: [ $( $variant:ident { $($field:ident),* } ),* $(,)? ],
        // Custom variant: StrategyKind → StrategyConfig body, then reverse.
        custom_to_config: [ $($to_config_arm:tt)* ],
        custom_to_kind:   [ $($to_kind_arm:tt)* ],
    ) => {
        impl From<&StrategyKind> for StrategyConfig {
            fn from(sk: &StrategyKind) -> Self {
                match sk {
                    $(
                        StrategyKind::$variant { $($field),* } =>
                            StrategyConfig::$variant { $($field: *$field),* },
                    )*
                    $($to_config_arm)*
                }
            }
        }

        impl From<&StrategyConfig> for StrategyKind {
            fn from(sc: &StrategyConfig) -> Self {
                match sc {
                    $(
                        StrategyConfig::$variant { $($field),* } =>
                            StrategyKind::$variant { $($field: *$field),* },
                    )*
                    $($to_kind_arm)*
                }
            }
        }
    };
}

strategy_convert! {
    copy: [
        Sequence { direction, gap, ratio },
        MasterStack { master_ratio, gap },
        Deck { master_ratio, gap },
        CenteredMaster { master_ratio, gap },
        BinarySplit { spiral, ratio, gap },
        ActivePanel { variant, bar_height },
        Window { size, gap },
    ],
    custom_to_config: [
        StrategyKind::Dashboard { columns, gap, spans, auto_rows } => StrategyConfig::Dashboard {
            columns: *columns, gap: *gap, spans: Box::from(&**spans), auto_rows: *auto_rows,
        },
        StrategyKind::Slotted { slots, gap, direction } => StrategyConfig::Slotted {
            slots: slots.iter().map(|s| SnapshotSlotDef {
                kind: Box::from(&*s.kind), constraints: s.constraints,
            }).collect::<Vec<_>>().into_boxed_slice(),
            gap: *gap, direction: *direction,
        },
    ],
    custom_to_kind: [
        StrategyConfig::Dashboard { columns, gap, spans, auto_rows } => StrategyKind::Dashboard {
            columns: *columns, gap: *gap, spans: Arc::from(&**spans), auto_rows: *auto_rows,
        },
        StrategyConfig::Slotted { slots, gap, direction } => StrategyKind::Slotted {
            slots: slots.iter().map(|s| SlotDef {
                kind: Arc::from(&*s.kind), constraints: s.constraints,
            }).collect::<Vec<_>>().into(),
            gap: *gap, direction: *direction,
        },
    ],
}

// ---------------------------------------------------------------------------
// Tree → SnapshotNode (walk the arena)
// ---------------------------------------------------------------------------

/// Maximum recursion depth for snapshot tree operations.
const MAX_SNAPSHOT_DEPTH: usize = 64;

/// Walk the tree from `root` and build a recursive `SnapshotNode`.
/// Returns `None` if root is missing or contains unsupported node types.
pub(crate) fn tree_to_snapshot(tree: &LayoutTree) -> Option<SnapshotNode> {
    let root = tree.root()?;
    node_to_snapshot(tree, root, 0)
}

fn container_snapshot(is_row: bool, gap: f32, children: Box<[SnapshotNode]>) -> SnapshotNode {
    match is_row {
        true => SnapshotNode::Row { gap, children },
        false => SnapshotNode::Col { gap, children },
    }
}

fn node_to_snapshot(tree: &LayoutTree, nid: NodeId, depth: usize) -> Option<SnapshotNode> {
    let node = tree.node(nid)?;
    match (depth > MAX_SNAPSHOT_DEPTH, node) {
        (true, _) | (_, Node::TaffyPassthrough { .. }) => None,
        (
            _,
            Node::Panel {
                kind, constraints, ..
            },
        ) => Some(SnapshotNode::Panel {
            kind: Box::from(&**kind),
            constraints: *constraints,
        }),
        (_, Node::Row { gap, children } | Node::Col { gap, children }) => {
            let is_row = matches!(node, Node::Row { .. });
            let kids: Box<[SnapshotNode]> = children
                .iter()
                .filter_map(|&c| node_to_snapshot(tree, c, depth + 1))
                .collect();
            (!kids.is_empty()).then(|| container_snapshot(is_row, *gap, kids))
        }
    }
}

// ---------------------------------------------------------------------------
// SnapshotNode → LayoutTree (rebuild via builder)
// ---------------------------------------------------------------------------

/// Rebuild a `LayoutTree` from a `SnapshotNode`.
pub(crate) fn snapshot_to_tree(root: &SnapshotNode) -> Result<LayoutTree, PaneError> {
    let mut builder = crate::builder::LayoutBuilder::new();
    match root {
        SnapshotNode::Row { gap, children } => {
            build_root_container(&mut builder, true, *gap, children)?;
        }
        SnapshotNode::Col { gap, children } => {
            build_root_container(&mut builder, false, *gap, children)?;
        }
        SnapshotNode::Panel { kind, constraints } => {
            builder.row(|ctx| {
                ctx.panel_with(&**kind, *constraints);
            })?;
        }
    }
    Ok(LayoutTree::from(builder.build()?))
}

fn build_root_container(
    builder: &mut crate::builder::LayoutBuilder,
    is_row: bool,
    gap: f32,
    children: &[SnapshotNode],
) -> Result<(), PaneError> {
    let populate = |ctx: &mut crate::ContainerCtx| {
        for child in children {
            add_snapshot_node(ctx, child, 1);
        }
    };
    match is_row {
        true => builder.row_gap(gap, populate),
        false => builder.col_gap(gap, populate),
    }
}

fn add_snapshot_node(ctx: &mut crate::ContainerCtx, node: &SnapshotNode, depth: usize) {
    if depth > MAX_SNAPSHOT_DEPTH {
        ctx.set_error(PaneError::InvalidTree(TreeError::SnapshotTooDeep(
            MAX_SNAPSHOT_DEPTH,
        )));
        return;
    }
    match node {
        SnapshotNode::Panel { kind, constraints } => {
            ctx.panel_with(&**kind, *constraints);
        }
        SnapshotNode::Row { gap, children } => {
            ctx.row_gap(*gap, |inner| add_snapshot_children(inner, children, depth));
        }
        SnapshotNode::Col { gap, children } => {
            ctx.col_gap(*gap, |inner| add_snapshot_children(inner, children, depth));
        }
    }
}

fn add_snapshot_children(ctx: &mut crate::ContainerCtx, children: &[SnapshotNode], depth: usize) {
    for child in children {
        add_snapshot_node(ctx, child, depth + 1);
    }
}

// ---------------------------------------------------------------------------
// Capture helper (called by LayoutRuntime)
// ---------------------------------------------------------------------------

/// Capture a snapshot from the current runtime state.
pub(crate) fn capture(
    tree: &LayoutTree,
    strategy: Option<&StrategyKind>,
    sequence: &crate::sequence::PanelSequence,
    viewport: &crate::viewport::ViewportState,
    overlay_defs: &[OverlayDef],
    breakpoints: Option<(&[crate::breakpoint::BreakpointEntry], usize)>,
) -> Result<LayoutSnapshot, PaneError> {
    let focused = viewport
        .focus
        .and_then(|pid| tree.panel_kind(pid).ok())
        .map(Box::from);

    let collapsed: Box<[Box<str>]> = viewport
        .collapsed
        .iter()
        .filter_map(|&pid| tree.panel_kind(pid).ok().map(Box::from))
        .collect();

    let panels_box = || -> Box<[Box<str>]> {
        sequence
            .iter()
            .filter_map(|pid| tree.panel_kind(pid).ok().map(Box::from))
            .collect()
    };

    let source = match (breakpoints, strategy) {
        (Some((bps, active_index)), _) => {
            let snap_bps: Box<[SnapshotBreakpoint]> = bps
                .iter()
                .map(|bp| SnapshotBreakpoint {
                    min_width: bp.min_width,
                    strategy: StrategyConfig::from(&bp.strategy),
                })
                .collect();
            SnapshotSource::Adaptive {
                breakpoints: snap_bps,
                panels: panels_box(),
                active_index,
            }
        }
        (None, Some(sk)) => SnapshotSource::Strategy {
            strategy: StrategyConfig::from(sk),
            panels: panels_box(),
        },
        (None, None) => {
            let root =
                tree_to_snapshot(tree).ok_or(PaneError::InvalidTree(TreeError::SnapshotNoRoot))?;
            SnapshotSource::Tree { root }
        }
    };

    let overlays: Box<[SnapshotOverlay]> = overlay_defs
        .iter()
        .map(|def| SnapshotOverlay {
            kind: Box::from(&*def.kind),
            anchor: def.anchor.clone(),
            width: def.width,
            height: def.height,
            visible: def.visible,
        })
        .collect();

    Ok(LayoutSnapshot {
        source,
        focused,
        collapsed,
        overlays,
    })
}
