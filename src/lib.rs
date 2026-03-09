//! Renderer-agnostic spatial layout engine.
//!
//! Describe panels in rows, columns, and presets. panes solves the geometry
//! via Taffy's flexbox engine and hands back a map of `PanelId → Rect`.

mod builder;
/// Compiles a [`LayoutTree`] into a Taffy tree for layout computation.
pub mod compiler;
/// Frame-to-frame diffing of resolved layouts.
pub mod diff;
mod error;
mod focus;
mod layout;
mod macros;
mod node;
mod panel;
mod preset;
mod rect;
mod resize;
/// Resolves compiled Taffy output into [`ResolvedLayout`].
pub mod resolver;
/// Stateful runtime with viewport tracking, caching, and frame diffing.
pub mod runtime;
/// Ordered panel sequence for focus navigation.
mod sequence;
/// Layout mutation strategies mapped to presets.
mod strategy;
#[cfg(feature = "toml")]
mod toml_parse;
mod tree;
mod validate;
mod viewport;

pub use builder::{ContainerCtx, LayoutBuilder};
pub use error::{ConstraintError, MutationError, PaneError, TreeError, ViewportError};
pub use focus::FocusDirection;
pub use layout::Layout;
pub use node::{Node, NodeId, PanelId};
pub use panel::{Constraints, PanelIdGenerator, fixed, grow};
pub use preset::{
    CenteredMaster, Columns, Dashboard, Deck, Dwindle, Grid, HolyGrail, MasterStack, Monocle,
    PanelInputKind, PresetInfo, Scrollable, Sidebar, Spiral, Split, Stacked, Tabbed,
};
pub use rect::Rect;
pub use resolver::{PanelEntry, ResolvedLayout};
pub use runtime::Placement;
pub use sequence::PanelSequence;
pub use strategy::{ActivePanelVariant, Direction, SlotDef, StrategyKind};
#[cfg(feature = "toml")]
pub use toml_parse::TomlError;
pub use tree::{LayoutTree, Position};
pub use viewport::ViewportState;
