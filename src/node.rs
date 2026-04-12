use crate::macros::id_newtype;

id_newtype!(
    /// Opaque unique identifier for a node in the layout tree.
    pub NodeId
);

id_newtype!(
    /// Opaque unique identifier for a panel.
    pub PanelId
);

id_newtype!(
    /// Stable panel identity that survives tree rebuilds.
    ///
    /// Assigned based on declaration order (sequence position).
    /// Panels sharing the same kind get distinct keys. Used for
    /// deterministic restore of focus and collapsed state when
    /// multiple panels share a kind.
    pub PanelKey
);

use std::sync::Arc;

use crate::panel::Constraints;
use crate::strategy::{CardSpan, GridColumnMode};

/// Node type in the layout tree arena.
///
/// Variants cover flex containers ([`Row`](Node::Row), [`Col`](Node::Col)),
/// CSS Grid ([`Grid`](Node::Grid), [`GridItemWrapper`](Node::GridItemWrapper)),
/// leaf panels ([`Panel`](Node::Panel)), and a raw Taffy escape hatch
/// ([`TaffyPassthrough`](Node::TaffyPassthrough)).
///
/// Construct trees via [`LayoutBuilder`](crate::LayoutBuilder) rather than
/// creating nodes directly.
#[derive(Debug, Clone)]
pub enum Node {
    /// Horizontal container laying children left-to-right.
    Row {
        /// Space between children.
        gap: f32,
        /// Optional constraints for this container as a child.
        constraints: Option<Constraints>,
        /// Ordered child node ids.
        children: Vec<NodeId>,
    },
    /// Vertical container laying children top-to-bottom.
    Col {
        /// Space between children.
        gap: f32,
        /// Optional constraints for this container as a child.
        constraints: Option<Constraints>,
        /// Ordered child node ids.
        children: Vec<NodeId>,
    },
    /// Leaf node representing a single panel.
    Panel {
        /// Unique panel identifier.
        id: PanelId,
        /// Application-defined panel kind (e.g. "editor", "chat").
        kind: Arc<str>,
        /// Size constraints for this panel.
        constraints: Constraints,
    },
    /// CSS Grid container with column mode, gap, and row sizing.
    Grid {
        /// Column layout mode (fixed count, auto-fill, or auto-fit).
        columns: GridColumnMode,
        /// Gap between grid items in pixels.
        gap: f32,
        /// When true, rows size to their tallest card instead of equal `1fr`.
        auto_rows: bool,
        /// Ordered child node ids.
        children: Vec<NodeId>,
    },
    /// Wrapper for a grid item that carries a column span.
    GridItemWrapper {
        /// Column span for this grid item.
        span: CardSpan,
        /// The single child node (typically a Panel).
        child: NodeId,
    },
    /// Raw Taffy node for escape-hatch styling.
    TaffyPassthrough {
        /// Custom Taffy style applied directly.
        style: Box<taffy::Style>,
        /// Ordered child node ids (immutable after construction).
        children: Box<[NodeId]>,
    },
}

impl Node {
    /// Child node ids for containers, empty slice for leaf nodes.
    pub fn children(&self) -> &[NodeId] {
        match self {
            Self::Row { children, .. }
            | Self::Col { children, .. }
            | Self::Grid { children, .. } => children,
            Self::GridItemWrapper { child, .. } => std::slice::from_ref(child),
            Self::TaffyPassthrough { children, .. } => children,
            Self::Panel { .. } => &[],
        }
    }

    /// Mutable access to a container's children list.
    pub(crate) fn children_mut(&mut self) -> Option<&mut Vec<NodeId>> {
        match self {
            Self::Row { children, .. }
            | Self::Col { children, .. }
            | Self::Grid { children, .. } => Some(children),
            Self::GridItemWrapper { .. } | Self::TaffyPassthrough { .. } | Self::Panel { .. } => {
                None
            }
        }
    }
}
