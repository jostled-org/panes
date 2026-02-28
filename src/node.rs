/// Generates a newtype wrapper around `u32` with `from_raw`, `raw`, and `Display`.
macro_rules! id_newtype {
    ($(#[$meta:meta])* $vis:vis $Name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $vis struct $Name(u32);

        impl $Name {
            /// Construct from a raw integer.
            pub fn from_raw(raw: u32) -> Self {
                Self(raw)
            }

            /// Return the underlying integer.
            pub fn raw(self) -> u32 {
                self.0
            }
        }

        impl std::fmt::Display for $Name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

id_newtype!(
    /// Opaque unique identifier for a node in the layout tree.
    pub NodeId
);

id_newtype!(
    /// Opaque unique identifier for a panel.
    pub PanelId
);

use std::rc::Rc;
use std::sync::Arc;

use crate::panel::Constraints;

/// A node in the layout tree.
#[derive(Debug, Clone)]
pub enum Node {
    Row {
        gap: f32,
        children: Vec<NodeId>,
    },
    Col {
        gap: f32,
        children: Vec<NodeId>,
    },
    Panel {
        id: PanelId,
        kind: Arc<str>,
        constraints: Constraints,
    },
    TaffyPassthrough {
        style: Rc<taffy::Style>,
        children: Vec<NodeId>,
    },
}

impl Node {
    /// Child node ids for containers, empty slice for leaf nodes.
    pub fn children(&self) -> &[NodeId] {
        match self {
            Self::Row { children, .. }
            | Self::Col { children, .. }
            | Self::TaffyPassthrough { children, .. } => children,
            Self::Panel { .. } => &[],
        }
    }

    /// Mutable access to a container's children list.
    pub(crate) fn children_mut(&mut self) -> Option<&mut Vec<NodeId>> {
        match self {
            Self::Row { children, .. }
            | Self::Col { children, .. }
            | Self::TaffyPassthrough { children, .. } => Some(children),
            Self::Panel { .. } => None,
        }
    }
}
