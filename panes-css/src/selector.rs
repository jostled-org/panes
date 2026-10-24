use std::borrow::Cow;
use std::fmt::{self, Display};

use panes::PanelId;

use crate::escape::write_attr_selector;

/// The attribute selector that `emit` targets for one element.
///
/// `Display` writes the CSS selector, e.g. `[data-pane-node="3"]`.
/// [`attribute`](Self::attribute) gives the matching markup attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneSelector<'a> {
    /// The layout root container: `[data-pane-root]`.
    Root,
    /// A non-root container, numbered in document order: `[data-pane-node="N"]`.
    Node(u32),
    /// A panel leaf: `[data-pane="kind"]`.
    Panel {
        /// The panel this element holds.
        id: PanelId,
        /// The panel kind used in the selector.
        kind: &'a str,
    },
}

impl<'a> PaneSelector<'a> {
    /// Attribute name and raw value to put on the element.
    ///
    /// The value is not escaped; escape it for the markup format you write.
    /// `Root` has an empty value (`data-pane-root` is a presence attribute).
    pub fn attribute(&self) -> (&'static str, Cow<'a, str>) {
        match *self {
            Self::Root => ("data-pane-root", Cow::Borrowed("")),
            Self::Node(n) => ("data-pane-node", Cow::Owned(n.to_string())),
            Self::Panel { kind, .. } => ("data-pane", Cow::Borrowed(kind)),
        }
    }
}

impl Display for PaneSelector<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Root => f.write_str("[data-pane-root]"),
            Self::Node(n) => write!(f, "[data-pane-node=\"{n}\"]"),
            Self::Panel { kind, .. } => write_attr_selector("data-pane", kind, f),
        }
    }
}
