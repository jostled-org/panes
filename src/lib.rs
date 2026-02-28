// panes — renderer-agnostic layout engine for Rust

mod builder;
pub mod compiler;
pub mod diff;
mod error;
mod layout;
mod macros;
mod node;
mod panel;
mod preset;
mod rect;
pub mod resolver;
pub mod runtime;
#[cfg(feature = "toml")]
mod toml_parse;
mod tree;
mod viewport;

pub use builder::{ContainerCtx, Gap, LayoutBuilder, gap};
pub use error::PaneError;
pub use layout::Layout;
pub use node::{Node, NodeId, PanelId};
pub use panel::{Constraints, PanelIdGenerator, fixed, grow};
pub use preset::{
    CenteredMaster, Columns, Dashboard, Deck, Dwindle, Grid, HolyGrail, MasterStack, Monocle,
    Scrollable, Sidebar, Spiral, Split, Stacked, Tabbed,
};
pub use rect::Rect;
pub use resolver::ResolvedLayout;
#[cfg(feature = "toml")]
pub use toml_parse::TomlError;
pub use tree::{LayoutTree, Position};
pub use viewport::ViewportState;
