mod builder;
mod resolve;
mod types;

pub use builder::Overlay;
pub(crate) use resolve::resolve_overlay;
pub(crate) use types::OverlayIdGenerator;
pub use types::{
    AnchorFailure, ExtentValue, HAlign, OverlayAnchor, OverlayDef, OverlayEntry, OverlayExtent,
    OverlayId, SnapshotOverlay, VAlign,
};
