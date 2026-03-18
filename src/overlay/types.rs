use std::sync::Arc;

use crate::macros::id_newtype;

#[cfg(feature = "serde")]
fn serialize_arc_str<S: serde::Serializer>(v: &Arc<str>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(v)
}

#[cfg(feature = "serde")]
fn deserialize_arc_str<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Arc<str>, D::Error> {
    let s: String = serde::Deserialize::deserialize(d)?;
    Ok(Arc::from(s.as_str()))
}

id_newtype!(
    /// Opaque overlay identifier.
    pub OverlayId
);

/// Horizontal alignment for overlay anchoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HAlign {
    /// Left edge.
    Left,
    /// Horizontal center.
    Center,
    /// Right edge.
    Right,
}

/// Vertical alignment for overlay anchoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VAlign {
    /// Top edge.
    Top,
    /// Vertical center.
    Center,
    /// Bottom edge.
    Bottom,
}

/// Where an overlay is positioned.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OverlayAnchor {
    /// Relative to viewport edges.
    Viewport {
        /// Horizontal alignment.
        h: HAlign,
        /// Vertical alignment.
        v: VAlign,
        /// Horizontal margin from edge.
        margin_x: f32,
        /// Vertical margin from edge.
        margin_y: f32,
    },
    /// Relative to a base panel's rect (looked up by kind).
    Panel {
        /// Kind of the anchor panel.
        #[cfg_attr(
            feature = "serde",
            serde(
                serialize_with = "serialize_arc_str",
                deserialize_with = "deserialize_arc_str"
            )
        )]
        kind: Arc<str>,
        /// Horizontal alignment relative to panel.
        h: HAlign,
        /// Vertical alignment relative to panel.
        v: VAlign,
        /// Horizontal offset from anchor.
        offset_x: f32,
        /// Vertical offset from anchor.
        offset_y: f32,
    },
}

/// How an overlay's width or height is determined.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OverlayExtent {
    /// The base size value.
    pub value: ExtentValue,
    /// Minimum size.
    pub min: Option<f32>,
    /// Maximum size.
    pub max: Option<f32>,
}

impl Default for OverlayExtent {
    fn default() -> Self {
        Self {
            value: ExtentValue::Fixed(100.0),
            min: None,
            max: None,
        }
    }
}

/// Base size value for an overlay extent.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExtentValue {
    /// Fixed pixel size.
    Fixed(f32),
    /// Percentage of viewport (0.0–100.0).
    Percent(f32),
    /// 100% of viewport on this axis.
    Full,
}

/// Complete overlay definition stored by the runtime.
#[derive(Debug, Clone)]
pub struct OverlayDef {
    pub(crate) id: OverlayId,
    pub(crate) kind: Arc<str>,
    pub(crate) anchor: OverlayAnchor,
    pub(crate) width: OverlayExtent,
    pub(crate) height: OverlayExtent,
    pub(crate) visible: bool,
}

impl OverlayDef {
    /// The overlay's unique identifier.
    pub fn id(&self) -> OverlayId {
        self.id
    }

    /// The overlay's kind string.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Whether the overlay is visible.
    pub fn visible(&self) -> bool {
        self.visible
    }
}

/// A resolved overlay for adapter consumption.
pub struct OverlayEntry<'a, R> {
    /// Overlay identifier.
    pub id: OverlayId,
    /// Overlay kind string.
    pub kind: &'a str,
    /// Computed rectangle.
    pub rect: R,
}

impl<'a, R> OverlayEntry<'a, R> {
    /// Transform the rect while preserving identity fields.
    pub fn map_rect<R2>(self, f: impl FnOnce(R) -> R2) -> OverlayEntry<'a, R2> {
        OverlayEntry {
            id: self.id,
            kind: self.kind,
            rect: f(self.rect),
        }
    }
}

/// Serializable overlay for snapshot persistence.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SnapshotOverlay {
    /// Overlay kind string.
    pub kind: Box<str>,
    /// Overlay anchor.
    pub anchor: OverlayAnchor,
    /// Width extent.
    pub width: OverlayExtent,
    /// Height extent.
    pub height: OverlayExtent,
    /// Visibility state.
    pub visible: bool,
}
