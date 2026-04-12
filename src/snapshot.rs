use std::sync::Arc;

use crate::error::{PaneError, TreeError};
use crate::node::{Node, NodeId, PanelKey};
use crate::overlay::{OverlayDef, SnapshotOverlay};
use crate::panel::Axis;
use crate::panel::Constraints;
use crate::strategy::{ActivePanelVariant, CardSpan, GridColumnMode, SlotDef, StrategyKind};
use crate::tree::LayoutTree;
use crate::validate::{check_f32_non_negative, float_invalid_to_constraint};

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
    /// Sequence index of the focused panel for deterministic restore
    /// with repeated kinds. Preferred over `focused` when present.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    focused_key: Option<PanelKey>,
    /// Sequence indices of collapsed panels for deterministic restore
    /// with repeated kinds. Preferred over `collapsed` when present.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "is_box_slice_empty")
    )]
    collapsed_keys: Box<[PanelKey]>,
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
    /// Returns the snapshot source: a strategy config, tree topology, or adaptive breakpoint set.
    pub fn source(&self) -> &SnapshotSource {
        &self.source
    }

    /// Returns the kind of the focused panel at capture time, if any.
    pub fn focused(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    /// Returns the kinds of all collapsed panels at capture time.
    pub fn collapsed(&self) -> &[Box<str>] {
        &self.collapsed
    }

    /// Sequence index of the focused panel for deterministic restore.
    pub fn focused_key(&self) -> Option<PanelKey> {
        self.focused_key
    }

    /// Sequence indices of collapsed panels for deterministic restore.
    pub fn collapsed_keys(&self) -> &[PanelKey] {
        &self.collapsed_keys
    }

    /// Returns the overlay definitions captured at snapshot time.
    pub fn overlays(&self) -> &[SnapshotOverlay] {
        &self.overlays
    }

    /// Consumes the snapshot and returns the overlay definitions as an owned `Vec`.
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

/// Serializable strategy recipe for snapshot restore.
///
/// Mirrors [`StrategyKind`] with owned collections (`Box<[T]>`) instead of
/// `Arc<[T]>`, making it safe for serde round-trips without shared-ownership
/// bookkeeping.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StrategyConfig {
    Sequence {
        axis: Axis,
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
        panel_count: usize,
        gap: f32,
    },
    Slotted {
        slots: Box<[SnapshotSlotDef]>,
        gap: f32,
        axis: Axis,
    },
}

/// Serializable slot definition for the Slotted strategy.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SnapshotSlotDef {
    pub kind: Box<str>,
    pub constraints: Constraints,
}

/// A grid item inside a [`SnapshotNode::Grid`] container.
///
/// Wraps a child node with an optional column span. Items without a span
/// use the default single-column placement.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SnapshotGridItem {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub span: Option<CardSpan>,
    pub node: SnapshotNode,
}

/// Recursive tree topology node for non-strategy snapshots.
///
/// Variants mirror the runtime node types (`Panel`, `Row`, `Col`, `Grid`).
/// The tree is walked depth-first during capture and rebuilt on restore.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SnapshotNode {
    Panel {
        kind: Box<str>,
        constraints: Constraints,
    },
    Row {
        gap: f32,
        #[cfg_attr(
            feature = "serde",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        constraints: Option<Constraints>,
        children: Box<[SnapshotNode]>,
    },
    Col {
        gap: f32,
        #[cfg_attr(
            feature = "serde",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        constraints: Option<Constraints>,
        children: Box<[SnapshotNode]>,
    },
    Grid {
        columns: GridColumnMode,
        gap: f32,
        #[cfg_attr(feature = "serde", serde(default))]
        auto_rows: bool,
        children: Box<[SnapshotGridItem]>,
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
        Sequence { axis, gap, ratio },
        MasterStack { master_ratio, gap },
        Deck { master_ratio, gap },
        CenteredMaster { master_ratio, gap },
        BinarySplit { spiral, ratio, gap },
        ActivePanel { variant, bar_height },
        Window { panel_count, gap },
    ],
    custom_to_config: [
        StrategyKind::Dashboard { columns, gap, spans, auto_rows } => StrategyConfig::Dashboard {
            columns: *columns, gap: *gap, spans: Box::from(&**spans), auto_rows: *auto_rows,
        },
        StrategyKind::Slotted { slots, gap, axis } => StrategyConfig::Slotted {
            slots: slots.iter().map(|s| SnapshotSlotDef {
                kind: Box::from(&*s.kind), constraints: s.constraints,
            }).collect::<Box<[_]>>(),
            gap: *gap, axis: *axis,
        },
    ],
    custom_to_kind: [
        StrategyConfig::Dashboard { columns, gap, spans, auto_rows } => StrategyKind::Dashboard {
            columns: *columns, gap: *gap, spans: Arc::from(&**spans), auto_rows: *auto_rows,
        },
        StrategyConfig::Slotted { slots, gap, axis } => StrategyKind::Slotted {
            slots: slots.iter().map(|s| SlotDef {
                kind: Arc::from(&*s.kind), constraints: s.constraints,
            }).collect::<Arc<[_]>>(),
            gap: *gap, axis: *axis,
        },
    ],
}

// ---------------------------------------------------------------------------
// Tree → SnapshotNode (walk the arena)
// ---------------------------------------------------------------------------

/// Maximum recursion depth for snapshot tree operations.
const MAX_SNAPSHOT_DEPTH: usize = 64;

/// Walk the tree from `root` and build a recursive `SnapshotNode`.
/// Returns `None` if the tree has no root.
pub(crate) fn tree_to_snapshot(tree: &LayoutTree) -> Result<Option<SnapshotNode>, PaneError> {
    let Some(root) = tree.root() else {
        return Ok(None);
    };
    node_to_snapshot(tree, root, 0).map(Some)
}

fn container_snapshot(
    is_row: bool,
    gap: f32,
    constraints: Option<Constraints>,
    children: Box<[SnapshotNode]>,
) -> SnapshotNode {
    match is_row {
        true => SnapshotNode::Row {
            gap,
            constraints,
            children,
        },
        false => SnapshotNode::Col {
            gap,
            constraints,
            children,
        },
    }
}

fn node_to_snapshot(
    tree: &LayoutTree,
    nid: NodeId,
    depth: usize,
) -> Result<SnapshotNode, PaneError> {
    if depth > MAX_SNAPSHOT_DEPTH {
        return Err(PaneError::InvalidTree(TreeError::SnapshotTooDeep(
            MAX_SNAPSHOT_DEPTH,
        )));
    }
    let Some(node) = tree.node(nid) else {
        return Err(PaneError::NodeNotFound(nid));
    };
    match node {
        Node::Grid {
            columns,
            gap,
            auto_rows,
            children,
        } => {
            let snap_children = children
                .iter()
                .map(|&child_nid| grid_child_to_snapshot(tree, child_nid, depth + 1))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice();
            Ok(SnapshotNode::Grid {
                columns: *columns,
                gap: *gap,
                auto_rows: *auto_rows,
                children: snap_children,
            })
        }
        Node::TaffyPassthrough { .. } | Node::GridItemWrapper { .. } => Err(
            PaneError::InvalidTree(TreeError::UnsupportedSnapshotNode(nid)),
        ),
        Node::Panel {
            kind, constraints, ..
        } => Ok(SnapshotNode::Panel {
            kind: Box::from(&**kind),
            constraints: *constraints,
        }),
        Node::Row {
            gap,
            constraints,
            children,
        }
        | Node::Col {
            gap,
            constraints,
            children,
        } => {
            let is_row = matches!(node, Node::Row { .. });
            let snap_children = children
                .iter()
                .map(|&child_id| node_to_snapshot(tree, child_id, depth + 1))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice();
            Ok(container_snapshot(
                is_row,
                *gap,
                *constraints,
                snap_children,
            ))
        }
    }
}

/// Convert a single grid child to a snapshot grid item.
///
/// If the child is a `GridItemWrapper`, unwrap it and attach the span
/// to the inner node. Otherwise treat as a regular child.
fn grid_child_to_snapshot(
    tree: &LayoutTree,
    nid: NodeId,
    depth: usize,
) -> Result<SnapshotGridItem, PaneError> {
    let Some(node) = tree.node(nid) else {
        return Err(PaneError::NodeNotFound(nid));
    };
    match node {
        Node::GridItemWrapper { span, child } => {
            let inner = node_to_snapshot(tree, *child, depth)?;
            Ok(SnapshotGridItem {
                span: Some(*span),
                node: inner,
            })
        }
        _ => {
            let inner = node_to_snapshot(tree, nid, depth)?;
            Ok(SnapshotGridItem {
                span: None,
                node: inner,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// SnapshotNode → LayoutTree (rebuild via builder)
// ---------------------------------------------------------------------------

/// Rebuild a `LayoutTree` from a `SnapshotNode`.
pub(crate) fn snapshot_to_tree(root: &SnapshotNode) -> Result<LayoutTree, PaneError> {
    let mut tree = LayoutTree::new();
    let root_id = snapshot_node_to_tree(&mut tree, root, 0)?;
    tree.set_root(root_id);
    tree.validate()?;
    Ok(tree)
}

fn snapshot_node_to_tree(
    tree: &mut LayoutTree,
    snapshot_node: &SnapshotNode,
    depth: usize,
) -> Result<NodeId, PaneError> {
    if depth > MAX_SNAPSHOT_DEPTH {
        return Err(PaneError::InvalidTree(TreeError::SnapshotTooDeep(
            MAX_SNAPSHOT_DEPTH,
        )));
    }

    match snapshot_node {
        SnapshotNode::Panel { kind, constraints } => {
            let (_, node_id) = tree.add_panel(&**kind, *constraints)?;
            Ok(node_id)
        }
        SnapshotNode::Row {
            gap,
            constraints,
            children,
        } => {
            validate_snapshot_gap(*gap)?;
            let child_ids = snapshot_children_to_tree(tree, children, depth)?;
            tree.add_row_constrained(*gap, *constraints, child_ids)
        }
        SnapshotNode::Col {
            gap,
            constraints,
            children,
        } => {
            validate_snapshot_gap(*gap)?;
            let child_ids = snapshot_children_to_tree(tree, children, depth)?;
            tree.add_col_constrained(*gap, *constraints, child_ids)
        }
        SnapshotNode::Grid {
            columns,
            gap,
            auto_rows,
            children,
        } => {
            crate::preset::validate_grid_columns(*columns)?;
            validate_snapshot_gap(*gap)?;
            let child_ids = grid_children_to_tree(tree, children, depth)?;
            tree.add_grid(*columns, *gap, *auto_rows, child_ids)
        }
    }
}

fn snapshot_children_to_tree(
    tree: &mut LayoutTree,
    children: &[SnapshotNode],
    depth: usize,
) -> Result<Vec<NodeId>, PaneError> {
    children
        .iter()
        .map(|child| snapshot_node_to_tree(tree, child, depth + 1))
        .collect()
}

fn grid_children_to_tree(
    tree: &mut LayoutTree,
    children: &[SnapshotGridItem],
    depth: usize,
) -> Result<Vec<NodeId>, PaneError> {
    children
        .iter()
        .map(|grid_item| grid_item_to_tree(tree, grid_item, depth + 1))
        .collect()
}

fn grid_item_to_tree(
    tree: &mut LayoutTree,
    grid_item: &SnapshotGridItem,
    depth: usize,
) -> Result<NodeId, PaneError> {
    if depth > MAX_SNAPSHOT_DEPTH {
        return Err(PaneError::InvalidTree(TreeError::SnapshotTooDeep(
            MAX_SNAPSHOT_DEPTH,
        )));
    }

    match (&grid_item.span, &grid_item.node) {
        (Some(span), SnapshotNode::Panel { kind, constraints }) => {
            crate::preset::validate_grid_span(*span)?;
            let (_, panel_id) = tree.add_panel(&**kind, *constraints)?;
            tree.add_grid_item(*span, panel_id)
        }
        (Some(_), snapshot_node) => Err(PaneError::InvalidTree(
            TreeError::SnapshotSpanRequiresPanel(snapshot_node_kind(snapshot_node)),
        )),
        (None, snapshot_node) => snapshot_node_to_tree(tree, snapshot_node, depth),
    }
}

fn validate_snapshot_gap(gap: f32) -> Result<(), PaneError> {
    check_f32_non_negative(gap)
        .map_err(|error| PaneError::InvalidConstraint(float_invalid_to_constraint("gap", error)))
}

fn snapshot_node_kind(snapshot_node: &SnapshotNode) -> &'static str {
    match snapshot_node {
        SnapshotNode::Panel { .. } => "panel",
        SnapshotNode::Row { .. } => "row",
        SnapshotNode::Col { .. } => "col",
        SnapshotNode::Grid { .. } => "grid",
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
        .map(|pid| tree.panel_kind(pid).map(Box::from))
        .transpose()?;

    let focused_key = match sequence.is_empty() {
        true => None,
        false => viewport
            .focus
            .map(|pid| {
                sequence
                    .index_of(pid)
                    .map(|idx| PanelKey::from_raw(idx as u32))
                    .ok_or(PaneError::InvalidTree(
                        TreeError::SnapshotFocusedMissingFromSequence(pid),
                    ))
            })
            .transpose()?,
    };

    let collapsed: Box<[Box<str>]> = viewport
        .collapsed
        .iter()
        .map(|&pid| tree.panel_kind(pid).map(Box::from))
        .collect::<Result<Vec<_>, _>>()?
        .into_boxed_slice();

    let collapsed_keys: Box<[PanelKey]> = match sequence.is_empty() {
        true => Box::default(),
        false => viewport
            .collapsed
            .iter()
            .map(|&pid| {
                sequence
                    .index_of(pid)
                    .map(|idx| PanelKey::from_raw(idx as u32))
                    .ok_or(PaneError::InvalidTree(
                        TreeError::SnapshotCollapsedMissingFromSequence(pid),
                    ))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_boxed_slice(),
    };

    let panels_box = || -> Result<Box<[Box<str>]>, PaneError> {
        Ok(sequence
            .iter()
            .map(|pid| tree.panel_kind(pid).map(Box::from))
            .collect::<Result<Vec<_>, _>>()?
            .into_boxed_slice())
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
                panels: panels_box()?,
                active_index,
            }
        }
        (None, Some(sk)) => SnapshotSource::Strategy {
            strategy: StrategyConfig::from(sk),
            panels: panels_box()?,
        },
        (None, None) => {
            let root =
                tree_to_snapshot(tree)?.ok_or(PaneError::InvalidTree(TreeError::SnapshotNoRoot))?;
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
        focused_key,
        collapsed_keys,
        overlays,
    })
}
