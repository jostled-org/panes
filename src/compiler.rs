use crate::error::{PaneError, TreeError, ViewportError};
use crate::node::{Node, NodeId, PanelId};
use crate::panel::{Align, Constraints};
use crate::strategy::Direction;
use crate::tree::LayoutTree;
use crate::validate::{FloatInvalid, check_f32_non_negative};

/// Result of compiling a `LayoutTree` into a Taffy tree.
pub struct CompileResult {
    /// The Taffy tree backing layout computation.
    pub taffy_tree: taffy::TaffyTree,
    /// Mapping from panes `NodeId` to Taffy `NodeId`, indexed by `NodeId::raw()`.
    pub node_map: Vec<Option<taffy::NodeId>>,
    /// The Taffy root node.
    pub root: taffy::NodeId,
}

/// Mutable state threaded through recursive compilation.
struct CompileCtx<'a> {
    taffy_tree: taffy::TaffyTree,
    node_map: Vec<Option<taffy::NodeId>>,
    panel_sizes: &'a [Option<(f32, f32)>],
}

/// Merge an axis-relative dimension with an absolute cross-axis override.
/// The absolute constraint takes precedence when set.
fn merge_dim(axis_dim: taffy::Dimension, absolute_dim: taffy::Dimension) -> taffy::Dimension {
    match absolute_dim == taffy::Dimension::auto() {
        true => axis_dim,
        false => absolute_dim,
    }
}

/// Map panes `Constraints` to a Taffy `Style` for a child along the given axis.
pub fn constraints_to_style(constraints: &Constraints, axis: Direction) -> taffy::Style {
    let (flex_grow, flex_basis, flex_shrink) = match (constraints.grow, constraints.fixed) {
        (Some(g), _) => (g, taffy::Dimension::length(0.0), 1.0),
        (_, Some(f)) => (0.0, taffy::Dimension::length(f), 0.0),
        (None, None) => (1.0, taffy::Dimension::length(0.0), 1.0),
    };

    let min_dim = constraints
        .min
        .map_or(taffy::Dimension::auto(), taffy::Dimension::length);
    let max_dim = constraints
        .max
        .map_or(taffy::Dimension::auto(), taffy::Dimension::length);

    let cross_min_w = constraints
        .min_width
        .map_or(taffy::Dimension::auto(), taffy::Dimension::length);
    let cross_max_w = constraints
        .max_width
        .map_or(taffy::Dimension::auto(), taffy::Dimension::length);
    let cross_min_h = constraints
        .min_height
        .map_or(taffy::Dimension::auto(), taffy::Dimension::length);
    let cross_max_h = constraints
        .max_height
        .map_or(taffy::Dimension::auto(), taffy::Dimension::length);

    let (min_size, max_size) = match axis {
        Direction::Horizontal => (
            taffy::Size {
                width: merge_dim(min_dim, cross_min_w),
                height: cross_min_h,
            },
            taffy::Size {
                width: merge_dim(max_dim, cross_max_w),
                height: cross_max_h,
            },
        ),
        Direction::Vertical => (
            taffy::Size {
                width: cross_min_w,
                height: merge_dim(min_dim, cross_min_h),
            },
            taffy::Size {
                width: cross_max_w,
                height: merge_dim(max_dim, cross_max_h),
            },
        ),
    };

    let align_self = match constraints.align {
        Some(Align::Start) => Some(taffy::AlignSelf::Start),
        Some(Align::Center) => Some(taffy::AlignSelf::Center),
        Some(Align::End) => Some(taffy::AlignSelf::End),
        Some(Align::Stretch) | None => None,
    };

    taffy::Style {
        flex_grow,
        flex_basis,
        flex_shrink,
        min_size,
        max_size,
        align_self,
        ..Default::default()
    }
}

/// Build a Taffy `Style` for a container node (Row, Col, or TaffyPassthrough).
///
/// Only called from the container arm of `compile_node`; the Panel arm is
/// unreachable by construction but kept for exhaustiveness.
fn container_style(node: &Node, is_root: bool) -> taffy::Style {
    let (size, flex_grow, flex_basis, flex_shrink) = match is_root {
        true => (
            taffy::Size {
                width: taffy::Dimension::percent(1.0),
                height: taffy::Dimension::percent(1.0),
            },
            0.0,
            taffy::Dimension::auto(),
            0.0,
        ),
        false => (taffy::Size::auto(), 1.0, taffy::Dimension::length(0.0), 1.0),
    };

    let (direction, gap_value) = match node {
        Node::Row { gap, .. } => (taffy::FlexDirection::Row, *gap),
        Node::Col { gap, .. } => (taffy::FlexDirection::Column, *gap),
        // Deep-clones the taffy::Style; known cost accepted for correctness.
        Node::TaffyPassthrough { style, .. } => return style.as_ref().clone(),
        Node::Panel { .. } => return taffy::Style::default(),
    };
    let gap_size = match direction {
        taffy::FlexDirection::Row | taffy::FlexDirection::RowReverse => taffy::Size {
            width: taffy::LengthPercentage::length(gap_value),
            height: taffy::LengthPercentage::length(0.0),
        },
        _ => taffy::Size {
            width: taffy::LengthPercentage::length(0.0),
            height: taffy::LengthPercentage::length(gap_value),
        },
    };
    taffy::Style {
        flex_direction: direction,
        size,
        flex_grow,
        flex_basis,
        flex_shrink,
        gap: gap_size,
        ..Default::default()
    }
}

/// Derive the layout direction from a parent node.
pub(crate) fn direction_of(node: &Node) -> Direction {
    match node {
        Node::Col { .. } => Direction::Vertical,
        Node::TaffyPassthrough { style, .. }
            if matches!(
                style.flex_direction,
                taffy::FlexDirection::Column | taffy::FlexDirection::ColumnReverse
            ) =>
        {
            Direction::Vertical
        }
        _ => Direction::Horizontal,
    }
}

/// Compile a `LayoutTree` into a Taffy tree ready for layout computation.
///
/// Validates the tree, then recursively walks from root, mapping each panes
/// node to a corresponding Taffy node.
pub fn compile(tree: &LayoutTree) -> Result<CompileResult, PaneError> {
    compile_with(tree, None)
}

/// Compile a `LayoutTree`, optionally reusing buffers from a previous compile.
pub fn compile_with(
    tree: &LayoutTree,
    reuse: Option<CompileResult>,
) -> Result<CompileResult, PaneError> {
    compile_with_sizes(tree, reuse, &[])
}

/// Compile a `LayoutTree` with intrinsic panel sizes, optionally reusing buffers.
pub fn compile_with_sizes(
    tree: &LayoutTree,
    reuse: Option<CompileResult>,
    panel_sizes: &[Option<(f32, f32)>],
) -> Result<CompileResult, PaneError> {
    #[cfg(debug_assertions)]
    tree.validate()?;

    let root_id = tree
        .root()
        .ok_or(PaneError::InvalidTree(TreeError::RootNotSet))?;

    let arena_len = tree.arena_len();
    let (taffy_tree, node_map) = match reuse {
        Some(prev) => {
            let mut t = prev.taffy_tree;
            t.clear();
            let mut nm = prev.node_map;
            nm.iter_mut().for_each(|slot| *slot = None);
            nm.resize(arena_len, None);
            (t, nm)
        }
        None => (taffy::TaffyTree::new(), vec![None; arena_len]),
    };

    let mut ctx = CompileCtx {
        taffy_tree,
        node_map,
        panel_sizes,
    };

    let root_node = tree.node(root_id).ok_or(PaneError::NodeNotFound(root_id))?;
    let root_axis = direction_of(root_node);
    let taffy_root = compile_node(tree, root_id, root_axis, true, &mut ctx)?;

    Ok(CompileResult {
        taffy_tree: ctx.taffy_tree,
        node_map: ctx.node_map,
        root: taffy_root,
    })
}

/// Build a taffy style for a panel, applying intrinsic size override when present.
fn panel_leaf_style(
    constraints: &Constraints,
    parent_axis: Direction,
    intrinsic: Option<(f32, f32)>,
) -> taffy::Style {
    let mut style = constraints_to_style(constraints, parent_axis);
    match intrinsic {
        Some((w, h)) => {
            style.size = taffy::Size {
                width: taffy::Dimension::length(w),
                height: taffy::Dimension::length(h),
            };
            style.flex_grow = 0.0;
            style.flex_shrink = 0.0;
            style.flex_basis = taffy::Dimension::auto();
        }
        None => {}
    }
    style
}

/// Recursively compile a single panes node into the Taffy tree.
fn compile_node(
    tree: &LayoutTree,
    nid: NodeId,
    parent_axis: Direction,
    is_root: bool,
    ctx: &mut CompileCtx<'_>,
) -> Result<taffy::NodeId, PaneError> {
    let node = tree.node(nid).ok_or(PaneError::NodeNotFound(nid))?;

    let taffy_id = match node {
        Node::Panel {
            constraints, id, ..
        } => {
            let intrinsic = ctx.panel_sizes.get(id.raw() as usize).copied().flatten();
            let style = panel_leaf_style(constraints, parent_axis, intrinsic);
            ctx.taffy_tree
                .new_leaf(style)
                .map_err(|e| PaneError::InvalidTree(TreeError::TaffyError(e.to_string().into())))?
        }
        Node::Row { .. } | Node::Col { .. } | Node::TaffyPassthrough { .. } => {
            let taffy_children = compile_children(tree, node, ctx)?;
            let style = container_style(node, is_root);
            ctx.taffy_tree
                .new_with_children(style, &taffy_children)
                .map_err(|e| PaneError::InvalidTree(TreeError::TaffyError(e.to_string().into())))?
        }
    };

    *ctx.node_map
        .get_mut(nid.raw() as usize)
        .ok_or(PaneError::NodeNotFound(nid))? = Some(taffy_id);
    Ok(taffy_id)
}

/// Compile all children of a container node, returning their Taffy node ids.
fn compile_children(
    tree: &LayoutTree,
    node: &Node,
    ctx: &mut CompileCtx<'_>,
) -> Result<Vec<taffy::NodeId>, PaneError> {
    let child_axis = direction_of(node);
    node.children()
        .iter()
        .map(|&child_nid| compile_node(tree, child_nid, child_axis, false, ctx))
        .collect()
}

/// Reject NaN, negative, or infinite viewport dimensions.
fn validate_viewport(width: f32, height: f32) -> Result<(), PaneError> {
    validate_dimension(width)?;
    validate_dimension(height)
}

fn validate_dimension(d: f32) -> Result<(), PaneError> {
    check_f32_non_negative(d).map_err(|e| {
        PaneError::InvalidViewport(match e {
            FloatInvalid::Nan => ViewportError::IsNan,
            FloatInvalid::Negative => ViewportError::IsNegative,
            FloatInvalid::Infinite => ViewportError::IsInfinite,
        })
    })
}

/// Run Taffy layout computation on a compiled tree at the given dimensions.
pub fn compute_layout(
    result: &mut CompileResult,
    width: f32,
    height: f32,
) -> Result<(), PaneError> {
    validate_viewport(width, height)?;
    let available = taffy::Size {
        width: taffy::AvailableSpace::Definite(width),
        height: taffy::AvailableSpace::Definite(height),
    };
    result
        .taffy_tree
        .compute_layout(result.root, available)
        .map_err(|e| PaneError::InvalidTree(TreeError::TaffyError(e.to_string().into())))
}

/// Look up the computed layout for a panel by its `PanelId`.
///
/// Resolves PanelId → NodeId → taffy::NodeId → taffy Layout.
pub fn panel_layout<'a>(
    result: &'a CompileResult,
    tree: &LayoutTree,
    pid: PanelId,
) -> Option<&'a taffy::Layout> {
    let nid = tree.node_for_panel(pid)?;
    let taffy_id = result.node_map.get(nid.raw() as usize)?.as_ref()?;
    result.taffy_tree.layout(*taffy_id).ok()
}
