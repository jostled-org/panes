use std::sync::Arc;

use crate::macros::id_newtype;
use crate::node::PanelKey;

#[cfg(feature = "serde")]
fn serialize_arc_str<S: serde::Serializer>(v: &Arc<str>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(v)
}

#[cfg(feature = "serde")]
fn deserialize_arc_str<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Arc<str>, D::Error> {
    let s: Box<str> = serde::Deserialize::deserialize(d)?;
    Ok(Arc::from(s))
}

id_newtype!(
    /// Opaque overlay identifier.
    pub OverlayId
);

/// Generates sequential, unique `OverlayId` values.
#[derive(Default)]
pub(crate) struct OverlayIdGenerator {
    counter: u32,
}

impl OverlayIdGenerator {
    pub(crate) fn next_id(&mut self) -> Result<OverlayId, crate::error::PaneError> {
        let id = OverlayId::from_raw(self.counter);
        self.counter = self
            .counter
            .checked_add(1)
            .ok_or(crate::error::PaneError::InvalidTree(
                crate::error::TreeError::OverlayIdExhausted,
            ))?;
        Ok(id)
    }
}

/// Horizontal alignment for overlay anchoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HAlign {
    Left,
    Center,
    Right,
}

/// Vertical alignment for overlay anchoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VAlign {
    Top,
    Center,
    Bottom,
}

/// Where an overlay is positioned.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OverlayAnchor {
    /// Relative to viewport edges.
    Viewport {
        h: HAlign,
        v: VAlign,
        margin_x: f32,
        margin_y: f32,
    },
    /// Relative to a base panel's rect (looked up by kind).
    ///
    /// Rejects ambiguity: if multiple panels share the kind, the overlay
    /// is silently omitted during resolve. Use `PanelAnchor` for
    /// identity-based anchoring when repeated kinds are expected.
    Panel {
        #[cfg_attr(
            feature = "serde",
            serde(
                serialize_with = "serialize_arc_str",
                deserialize_with = "deserialize_arc_str"
            )
        )]
        kind: Arc<str>,
        h: HAlign,
        v: VAlign,
        offset_x: f32,
        offset_y: f32,
    },
    /// Relative to a panel identified by its stable [`PanelKey`].
    ///
    /// Preferred over `Panel` when multiple panels may share a kind.
    /// The key is resolved via the runtime's sequence to a `PanelId`,
    /// then looked up in the resolved layout.
    PanelAnchor {
        key: PanelKey,
        h: HAlign,
        v: VAlign,
        offset_x: f32,
        offset_y: f32,
    },
}

/// How an overlay's width or height is determined.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OverlayExtent {
    pub value: ExtentValue,
    pub min: Option<f32>,
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
    Fixed(f32),
    /// Percentage of the viewport size.
    ///
    /// Values above `100.0` are allowed and produce overlays larger than the
    /// viewport before any optional min/max clamp is applied.
    Percent(f32),
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
    pub fn id(&self) -> OverlayId {
        self.id
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn anchor(&self) -> &OverlayAnchor {
        &self.anchor
    }

    pub fn width(&self) -> &OverlayExtent {
        &self.width
    }

    pub fn height(&self) -> &OverlayExtent {
        &self.height
    }
}

/// A resolved overlay for adapter consumption.
pub struct OverlayEntry<'a, R> {
    pub id: OverlayId,
    pub kind: &'a str,
    pub rect: R,
}

impl<'a, R> OverlayEntry<'a, R> {
    pub fn map_rect<R2>(self, f: impl FnOnce(R) -> R2) -> OverlayEntry<'a, R2> {
        OverlayEntry {
            id: self.id,
            kind: self.kind,
            rect: f(self.rect),
        }
    }
}

/// Why an overlay anchor failed to resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorFailure {
    /// Kind-based anchor matched zero panels.
    KindNotFound,
    /// Kind-based anchor matched multiple panels (ambiguous).
    KindAmbiguous,
    /// PanelKey no longer maps to a live panel in the sequence/layout.
    KeyStale,
}

/// Serializable overlay for snapshot persistence.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SnapshotOverlay {
    pub kind: Box<str>,
    pub anchor: OverlayAnchor,
    pub width: OverlayExtent,
    pub height: OverlayExtent,
    pub visible: bool,
}
