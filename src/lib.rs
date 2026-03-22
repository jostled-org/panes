//! Renderer-agnostic spatial layout engine.
//!
//! Describe panels in rows, columns, and presets. panes solves the geometry
//! via Taffy's flexbox engine and hands back a map of `PanelId → Rect`.

mod breakpoint;
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
/// Overlay types for floating UI elements rendered above the base layout.
pub mod overlay;
mod panel;
mod preset;
mod rect;
mod resize;
/// Resolves compiled Taffy output into [`ResolvedLayout`].
pub mod resolver;
/// Stateful runtime with viewport tracking, caching, and frame diffing.
pub mod runtime;
mod runtime_overlay;
/// Ordered panel sequence for focus navigation.
mod sequence;
/// Serializable snapshots for session persistence.
mod snapshot;
/// Layout mutation strategies mapped to presets.
mod strategy;
#[cfg(feature = "toml")]
mod toml_parse;
mod tree;
mod validate;
mod viewport;

pub use breakpoint::{AdaptiveBuilder, BreakpointEntry};
pub use builder::{ContainerCtx, LayoutBuilder};
pub use diff::{DiffScratch, LayoutDiff, OverlayDiff, OverlayRectChange, RectChange};
pub use error::{ConstraintError, MutationError, PaneError, TreeError, ViewportError};
pub use focus::FocusDirection;
pub use layout::Layout;
pub use node::{Node, NodeId, PanelId};
pub use overlay::{
    ExtentValue, HAlign, Overlay, OverlayAnchor, OverlayDef, OverlayEntry, OverlayExtent,
    OverlayId, SnapshotOverlay, VAlign,
};
pub use panel::{Align, Constraints, PanelIdGenerator, SizeMode, fixed, grow};
pub use preset::{
    CenteredMaster, Dashboard, Deck, Dwindle, HolyGrail, MasterStack, Monocle, PanelInputKind,
    PresetInfo, Scrollable, Sidebar, Spiral, Split, Stacked, Tabbed,
};
pub use rect::Rect;
pub use resolver::{BoundaryAxis, BoundaryHit, PanelEntry, ResolvedLayout};
pub use runtime::Placement;
pub use sequence::PanelSequence;
pub use snapshot::{
    LayoutSnapshot, SnapshotBreakpoint, SnapshotNode, SnapshotSlotDef, SnapshotSource,
    StrategyConfig,
};
pub use strategy::{
    ActivePanelStrategy, ActivePanelVariant, BinarySplitStrategy, BoundStrategy, CardSpan,
    CenteredMasterStrategy, DashboardStrategy, DeckStrategy, Direction, GridColumnMode,
    HolyGrailStrategy, MasterStackStrategy, SidebarStrategy, SlotDef, SplitStrategy, Strategy,
    StrategyKind, WindowStrategy,
};
#[cfg(feature = "toml")]
pub use toml_parse::TomlError;
pub use tree::{LayoutTree, Position};
pub use viewport::ViewportState;
