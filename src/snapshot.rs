use std::sync::Arc;

use crate::error::PaneError;
use crate::node::{Node, NodeId};
use crate::panel::Constraints;
use crate::strategy::{ActivePanelVariant, Direction, SlotDef, StrategyKind};
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
/// let snapshot = rt.snapshot();
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
    focused: Option<String>,
    collapsed: Vec<String>,
}

impl LayoutSnapshot {
    /// The snapshot source (strategy recipe or tree topology).
    pub fn source(&self) -> &SnapshotSource {
        &self.source
    }

    /// The kind of the focused panel at snapshot time, if any.
    pub fn focused(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    /// Kinds of collapsed panels at snapshot time.
    pub fn collapsed(&self) -> &[String] {
        &self.collapsed
    }
}

/// What a snapshot restores from: a strategy recipe or a tree topology.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SnapshotSource {
    /// Strategy-based runtime — rebuild from recipe.
    Strategy {
        /// The strategy configuration.
        strategy: StrategyConfig,
        /// Panel kinds in sequence order (no decorative panels).
        panels: Vec<String>,
    },
    /// Non-strategy runtime — rebuild from tree topology.
    Tree {
        /// The root node of the tree.
        root: SnapshotNode,
    },
}

/// Serializable strategy configuration — mirrors [`StrategyKind`] with
/// owned collections instead of `Arc<[T]>`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StrategyConfig {
    /// Flat sequence of panels in a row or column.
    Sequence {
        /// Layout direction.
        direction: Direction,
        /// Gap between panels.
        gap: f32,
    },
    /// One master panel with a stack of secondaries.
    MasterStack {
        /// Fraction of space for the master panel.
        master_ratio: f32,
        /// Gap between panels.
        gap: f32,
    },
    /// Master panel with a peek at adjacent panels.
    Deck {
        /// Fraction of space for the master panel.
        master_ratio: f32,
        /// Gap between panels.
        gap: f32,
    },
    /// Master panel centered with stacks on both sides.
    CenteredMaster {
        /// Fraction of space for the master panel.
        master_ratio: f32,
        /// Gap between panels.
        gap: f32,
    },
    /// Recursive binary splits (dwindle or spiral).
    BinarySplit {
        /// Whether to alternate split direction in a spiral.
        spiral: bool,
        /// Split ratio between parent and child.
        ratio: f32,
        /// Gap between panels.
        gap: f32,
    },
    /// Fixed number of equal columns with panels distributed.
    ColumnGrid {
        /// Number of columns.
        columns: usize,
        /// Gap between panels.
        gap: f32,
    },
    /// Grid with per-panel column spans.
    Dashboard {
        /// Number of columns.
        columns: usize,
        /// Gap between panels.
        gap: f32,
        /// Column span for each panel.
        spans: Vec<usize>,
    },
    /// Only one panel visible at a time (monocle, tabbed, stacked).
    ActivePanel {
        /// Which variant of active-panel display.
        variant: ActivePanelVariant,
        /// Height of the tab/title bar decoration.
        bar_height: f32,
    },
    /// Sliding window of visible panels.
    Window {
        /// How many panels are visible at once.
        size: usize,
        /// Gap between panels.
        gap: f32,
    },
    /// Fixed slots with predefined constraints.
    Slotted {
        /// Slot definitions (kind + constraints).
        slots: Vec<SnapshotSlotDef>,
        /// Gap between slots.
        gap: f32,
        /// Layout direction.
        direction: Direction,
    },
}

/// Serializable slot definition for the Slotted strategy.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SnapshotSlotDef {
    /// Panel kind for this slot.
    pub kind: String,
    /// Size constraints for this slot.
    pub constraints: Constraints,
}

/// Recursive tree node for serializing non-strategy layouts.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SnapshotNode {
    /// A leaf panel.
    Panel {
        /// Application-defined panel kind.
        kind: String,
        /// Size constraints.
        constraints: Constraints,
    },
    /// Horizontal container (children laid out left-to-right).
    Row {
        /// Gap between children.
        gap: f32,
        /// Child nodes.
        children: Vec<SnapshotNode>,
    },
    /// Vertical container (children laid out top-to-bottom).
    Col {
        /// Gap between children.
        gap: f32,
        /// Child nodes.
        children: Vec<SnapshotNode>,
    },
}

// ---------------------------------------------------------------------------
// StrategyKind ↔ StrategyConfig conversions
// ---------------------------------------------------------------------------

impl From<&StrategyKind> for StrategyConfig {
    fn from(sk: &StrategyKind) -> Self {
        match sk {
            StrategyKind::Sequence { direction, gap } => StrategyConfig::Sequence {
                direction: *direction,
                gap: *gap,
            },
            StrategyKind::MasterStack { master_ratio, gap } => StrategyConfig::MasterStack {
                master_ratio: *master_ratio,
                gap: *gap,
            },
            StrategyKind::Deck { master_ratio, gap } => StrategyConfig::Deck {
                master_ratio: *master_ratio,
                gap: *gap,
            },
            StrategyKind::CenteredMaster { master_ratio, gap } => StrategyConfig::CenteredMaster {
                master_ratio: *master_ratio,
                gap: *gap,
            },
            StrategyKind::BinarySplit { spiral, ratio, gap } => StrategyConfig::BinarySplit {
                spiral: *spiral,
                ratio: *ratio,
                gap: *gap,
            },
            StrategyKind::ColumnGrid { columns, gap } => StrategyConfig::ColumnGrid {
                columns: *columns,
                gap: *gap,
            },
            StrategyKind::Dashboard {
                columns,
                gap,
                spans,
            } => StrategyConfig::Dashboard {
                columns: *columns,
                gap: *gap,
                spans: spans.to_vec(),
            },
            StrategyKind::ActivePanel {
                variant,
                bar_height,
            } => StrategyConfig::ActivePanel {
                variant: *variant,
                bar_height: *bar_height,
            },
            StrategyKind::Window { size, gap } => StrategyConfig::Window {
                size: *size,
                gap: *gap,
            },
            StrategyKind::Slotted {
                slots,
                gap,
                direction,
            } => StrategyConfig::Slotted {
                slots: slots
                    .iter()
                    .map(|s| SnapshotSlotDef {
                        kind: s.kind.to_string(),
                        constraints: s.constraints,
                    })
                    .collect(),
                gap: *gap,
                direction: *direction,
            },
        }
    }
}

impl From<&StrategyConfig> for StrategyKind {
    fn from(sc: &StrategyConfig) -> Self {
        match sc {
            StrategyConfig::Sequence { direction, gap } => StrategyKind::Sequence {
                direction: *direction,
                gap: *gap,
            },
            StrategyConfig::MasterStack { master_ratio, gap } => StrategyKind::MasterStack {
                master_ratio: *master_ratio,
                gap: *gap,
            },
            StrategyConfig::Deck { master_ratio, gap } => StrategyKind::Deck {
                master_ratio: *master_ratio,
                gap: *gap,
            },
            StrategyConfig::CenteredMaster { master_ratio, gap } => StrategyKind::CenteredMaster {
                master_ratio: *master_ratio,
                gap: *gap,
            },
            StrategyConfig::BinarySplit { spiral, ratio, gap } => StrategyKind::BinarySplit {
                spiral: *spiral,
                ratio: *ratio,
                gap: *gap,
            },
            StrategyConfig::ColumnGrid { columns, gap } => StrategyKind::ColumnGrid {
                columns: *columns,
                gap: *gap,
            },
            StrategyConfig::Dashboard {
                columns,
                gap,
                spans,
            } => StrategyKind::Dashboard {
                columns: *columns,
                gap: *gap,
                spans: spans.as_slice().into(),
            },
            StrategyConfig::ActivePanel {
                variant,
                bar_height,
            } => StrategyKind::ActivePanel {
                variant: *variant,
                bar_height: *bar_height,
            },
            StrategyConfig::Window { size, gap } => StrategyKind::Window {
                size: *size,
                gap: *gap,
            },
            StrategyConfig::Slotted {
                slots,
                gap,
                direction,
            } => StrategyKind::Slotted {
                slots: slots
                    .iter()
                    .map(|s| SlotDef {
                        kind: Arc::from(s.kind.as_str()),
                        constraints: s.constraints,
                    })
                    .collect::<Vec<_>>()
                    .into(),
                gap: *gap,
                direction: *direction,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Tree → SnapshotNode (walk the arena)
// ---------------------------------------------------------------------------

/// Walk the tree from `root` and build a recursive `SnapshotNode`.
/// Returns `None` if root is missing or contains unsupported node types.
pub(crate) fn tree_to_snapshot(tree: &LayoutTree) -> Option<SnapshotNode> {
    let root = tree.root()?;
    node_to_snapshot(tree, root)
}

fn node_to_snapshot(tree: &LayoutTree, nid: NodeId) -> Option<SnapshotNode> {
    let node = tree.node(nid)?;
    match node {
        Node::Panel {
            kind, constraints, ..
        } => Some(SnapshotNode::Panel {
            kind: kind.to_string(),
            constraints: *constraints,
        }),
        Node::Row { gap, children } => {
            let kids: Vec<SnapshotNode> = children
                .iter()
                .filter_map(|&c| node_to_snapshot(tree, c))
                .collect();
            Some(SnapshotNode::Row {
                gap: *gap,
                children: kids,
            })
        }
        Node::Col { gap, children } => {
            let kids: Vec<SnapshotNode> = children
                .iter()
                .filter_map(|&c| node_to_snapshot(tree, c))
                .collect();
            Some(SnapshotNode::Col {
                gap: *gap,
                children: kids,
            })
        }
        Node::TaffyPassthrough { .. } => None,
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
            builder.row_gap(*gap, |ctx| {
                for child in children {
                    add_snapshot_node(ctx, child);
                }
            })?;
        }
        SnapshotNode::Col { gap, children } => {
            builder.col_gap(*gap, |ctx| {
                for child in children {
                    add_snapshot_node(ctx, child);
                }
            })?;
        }
        SnapshotNode::Panel { kind, constraints } => {
            builder.row(|ctx| {
                ctx.panel_with(kind.as_str(), *constraints);
            })?;
        }
    }
    Ok(LayoutTree::from(builder.build()?))
}

fn add_snapshot_node(ctx: &mut crate::ContainerCtx, node: &SnapshotNode) {
    match node {
        SnapshotNode::Panel { kind, constraints } => {
            ctx.panel_with(kind.as_str(), *constraints);
        }
        SnapshotNode::Row { gap, children } => {
            ctx.row_gap(*gap, |inner| {
                for child in children {
                    add_snapshot_node(inner, child);
                }
            });
        }
        SnapshotNode::Col { gap, children } => {
            ctx.col_gap(*gap, |inner| {
                for child in children {
                    add_snapshot_node(inner, child);
                }
            });
        }
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
) -> LayoutSnapshot {
    let focused = viewport
        .focus
        .and_then(|pid| tree.panel_kind(pid).ok())
        .map(str::to_string);

    let collapsed: Vec<String> = viewport
        .collapsed
        .iter()
        .filter_map(|&pid| tree.panel_kind(pid).ok().map(str::to_string))
        .collect();

    let source = match strategy {
        Some(sk) => {
            let panels: Vec<String> = sequence
                .iter()
                .filter_map(|pid| tree.panel_kind(pid).ok().map(str::to_string))
                .collect();
            SnapshotSource::Strategy {
                strategy: StrategyConfig::from(sk),
                panels,
            }
        }
        None => {
            let root = tree_to_snapshot(tree).unwrap_or(SnapshotNode::Row {
                gap: 0.0,
                children: Vec::new(),
            });
            SnapshotSource::Tree { root }
        }
    };

    LayoutSnapshot {
        source,
        focused,
        collapsed,
    }
}
