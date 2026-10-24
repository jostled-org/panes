use panes::{Axis, CardSpan, Constraints, GridColumnMode, LayoutTree, Node, NodeId};

use crate::selector::PaneSelector;

/// One element of the markup that [`emit`](crate::emit) styles.
///
/// [`skeleton`] returns these in document order. Build one element per entry,
/// nest each under `parent`, and give it `selector.attribute()`. The CSS from
/// `emit` then matches by construction.
#[derive(Debug, Clone, Copy)]
pub struct PaneElement<'a> {
    /// The tree node this element renders.
    pub node: NodeId,
    /// Index of the parent element in the skeleton. `None` for the root.
    pub parent: Option<usize>,
    /// The selector `emit` writes for this element.
    pub selector: PaneSelector<'a>,
    pub(crate) context: Context,
    pub(crate) shape: Shape<'a>,
}

/// How the parent lays out an element.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Context {
    Root,
    Flex(Axis),
    GridCard,
    Scroll(ScrollAxis),
}

impl Context {
    /// Main axis of the parent, for flex sizing properties.
    pub(crate) fn parent_axis(self) -> Axis {
        match self {
            Self::Flex(axis) => axis,
            Self::Scroll(ScrollAxis::Y) => Axis::Col,
            Self::Root | Self::GridCard | Self::Scroll(ScrollAxis::X) => Axis::Row,
        }
    }
}

/// The rule an element needs, with the node data that rule reads.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Shape<'a> {
    Panel(&'a Constraints),
    Flex {
        axis: Axis,
        gap: f32,
        constraints: Option<&'a Constraints>,
    },
    Grid {
        columns: GridColumnMode,
        gap: f32,
        auto_rows: bool,
    },
    GridCard(CardSpan),
    Scroll(ScrollAxis),
    Passthrough,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ScrollAxis {
    X,
    Y,
}

/// List the elements `emit` writes rules for, in document order.
///
/// Containers get `[data-pane-node="N"]` numbers in the order listed here,
/// which is the order `emit` uses. Accepts a `Layout` or a `LayoutTree`, such
/// as `runtime.tree()`.
pub fn skeleton<T: AsRef<LayoutTree> + ?Sized>(layout: &T) -> Box<[PaneElement<'_>]> {
    let tree = layout.as_ref();
    let mut walk = Walk {
        tree,
        elements: Vec::with_capacity(tree.node_count()),
        counter: 0,
    };
    if let Some(root) = tree.root() {
        walk.visit(root, None, Context::Root);
    }
    walk.elements.into_boxed_slice()
}

struct Walk<'a> {
    tree: &'a LayoutTree,
    elements: Vec<PaneElement<'a>>,
    counter: u32,
}

impl<'a> Walk<'a> {
    fn visit(&mut self, nid: NodeId, parent: Option<usize>, context: Context) {
        let Some(node) = self.tree.node(nid) else {
            return;
        };
        match node {
            Node::Panel {
                id,
                kind,
                constraints,
            } => {
                let selector = PaneSelector::Panel { id: *id, kind };
                self.push(nid, parent, context, selector, Shape::Panel(constraints));
            }
            Node::Row {
                gap,
                constraints,
                children,
            } => {
                let shape = flex_shape(Axis::Row, *gap, constraints.as_ref());
                let child_context = Context::Flex(Axis::Row);
                self.visit_container(nid, parent, context, shape, children, child_context);
            }
            Node::Col {
                gap,
                constraints,
                children,
            } => {
                let shape = flex_shape(Axis::Col, *gap, constraints.as_ref());
                let child_context = Context::Flex(Axis::Col);
                self.visit_container(nid, parent, context, shape, children, child_context);
            }
            Node::Grid {
                columns,
                gap,
                auto_rows,
                children,
            } => {
                let shape = Shape::Grid {
                    columns: *columns,
                    gap: *gap,
                    auto_rows: *auto_rows,
                };
                let index = self.push_container(nid, parent, context, shape);
                children
                    .iter()
                    .for_each(|&child| self.visit_grid_child(child, index));
            }
            Node::TaffyPassthrough { style, children } => {
                let (shape, child_context) =
                    passthrough_layout(style, self.tree, children, context);
                self.visit_container(nid, parent, context, shape, children, child_context);
            }
            // Only meaningful inside a grid; `visit_grid_child` places it.
            Node::GridItemWrapper { .. } => {}
        }
    }

    /// Push a container and visit its children under `child_context`.
    fn visit_container(
        &mut self,
        nid: NodeId,
        parent: Option<usize>,
        context: Context,
        shape: Shape<'a>,
        children: &[NodeId],
        child_context: Context,
    ) {
        let index = self.push_container(nid, parent, context, shape);
        children
            .iter()
            .for_each(|&child| self.visit(child, Some(index), child_context));
    }

    fn visit_grid_child(&mut self, nid: NodeId, grid: usize) {
        match self.tree.node(nid) {
            Some(Node::GridItemWrapper { span, child }) => {
                let index =
                    self.push_container(nid, Some(grid), Context::GridCard, Shape::GridCard(*span));
                self.visit(*child, Some(index), Context::Flex(Axis::Row));
            }
            _ => self.visit(nid, Some(grid), Context::Flex(Axis::Row)),
        }
    }

    /// Push a container, numbering it unless it is the root.
    fn push_container(
        &mut self,
        nid: NodeId,
        parent: Option<usize>,
        context: Context,
        shape: Shape<'a>,
    ) -> usize {
        let selector = match context {
            Context::Root => PaneSelector::Root,
            _ => {
                self.counter += 1;
                PaneSelector::Node(self.counter)
            }
        };
        self.push(nid, parent, context, selector, shape)
    }

    fn push(
        &mut self,
        node: NodeId,
        parent: Option<usize>,
        context: Context,
        selector: PaneSelector<'a>,
        shape: Shape<'a>,
    ) -> usize {
        self.elements.push(PaneElement {
            node,
            parent,
            selector,
            context,
            shape,
        });
        self.elements.len() - 1
    }
}

fn flex_shape(axis: Axis, gap: f32, constraints: Option<&Constraints>) -> Shape<'_> {
    Shape::Flex {
        axis,
        gap,
        constraints,
    }
}

/// Shape of a `TaffyPassthrough` and the context its children get.
fn passthrough_layout<'a>(
    style: &taffy::Style,
    tree: &LayoutTree,
    children: &[NodeId],
    context: Context,
) -> (Shape<'a>, Context) {
    match scrollable_axis(style, tree, children) {
        Some(axis) => (Shape::Scroll(axis), Context::Scroll(axis)),
        None => (Shape::Passthrough, Context::Flex(context.parent_axis())),
    }
}

/// A `TaffyPassthrough` scrolls when it is a nowrap flex row or column whose
/// children are all fixed-size panels.
fn scrollable_axis(
    style: &taffy::Style,
    tree: &LayoutTree,
    children: &[NodeId],
) -> Option<ScrollAxis> {
    let axis = match (style.display, style.flex_direction, style.flex_wrap) {
        (taffy::Display::Flex, taffy::FlexDirection::Row, taffy::FlexWrap::NoWrap) => ScrollAxis::X,
        (taffy::Display::Flex, taffy::FlexDirection::Column, taffy::FlexWrap::NoWrap) => {
            ScrollAxis::Y
        }
        _ => return None,
    };
    let all_fixed_panels = !children.is_empty()
        && children.iter().all(|&nid| {
            matches!(
                tree.node(nid),
                Some(Node::Panel { constraints, .. }) if constraints.fixed.is_some()
            )
        });
    all_fixed_panels.then_some(axis)
}
