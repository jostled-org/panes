mod builder;
mod resolve;
mod types;

pub use builder::Overlay;
pub(crate) use resolve::{OverlayIdGenerator, resolve_overlay};
pub use types::{
    ExtentValue, HAlign, OverlayAnchor, OverlayDef, OverlayEntry, OverlayExtent, OverlayId,
    SnapshotOverlay, VAlign,
};
