use std::sync::Arc;

use super::resolve::validate_extent;
use super::types::{
    ExtentValue, HAlign, OverlayAnchor, OverlayDef, OverlayExtent, OverlayId, VAlign,
};
use crate::node::PanelKey;

/// Builder for constructing overlay definitions.
///
/// Created via static constructors like `Overlay::center()`, `Overlay::bottom()`, etc.
pub struct Overlay {
    anchor: OverlayAnchor,
    width: OverlayExtent,
    height: OverlayExtent,
}

/// Declares viewport-anchored and panel-anchored static constructors on `Overlay`.
///
/// Viewport arm: `(viewport $name:ident ($($params),*) => $doc, $h, $v, $mx, $my)`
/// Panel arm:    `(panel $name:ident => $doc, $h, $v)`
macro_rules! overlay_ctors {
    // Viewport-anchored: no margin parameters (defaults to 0.0, 0.0).
    (viewport $name:ident () => $doc:expr, $h:expr, $v:expr, $mx:expr, $my:expr) => {
        #[doc = $doc]
        pub fn $name() -> Self {
            Self::viewport($h, $v, $mx, $my)
        }
    };
    // Viewport-anchored: single margin parameter applied to margin_y.
    (viewport $name:ident (margin) => $doc:expr, $h:expr, $v:expr, $mx:expr) => {
        #[doc = $doc]
        pub fn $name(margin: f32) -> Self {
            Self::viewport($h, $v, $mx, margin)
        }
    };
    // Viewport-anchored: separate mx/my parameters.
    (viewport $name:ident (mx, my) => $doc:expr, $h:expr, $v:expr) => {
        #[doc = $doc]
        pub fn $name(mx: f32, my: f32) -> Self {
            Self::viewport($h, $v, mx, my)
        }
    };
    // Panel-anchored: takes a panel kind.
    (panel $name:ident => $doc:expr, $h:expr, $v:expr) => {
        #[doc = $doc]
        pub fn $name(kind: impl Into<Arc<str>>) -> Self {
            Self::panel(kind.into(), $h, $v)
        }
    };
}

impl Overlay {
    fn viewport(h: HAlign, v: VAlign, margin_x: f32, margin_y: f32) -> Self {
        Self {
            anchor: OverlayAnchor::Viewport {
                h,
                v,
                margin_x,
                margin_y,
            },
            width: OverlayExtent::default(),
            height: OverlayExtent::default(),
        }
    }

    fn panel(kind: Arc<str>, h: HAlign, v: VAlign) -> Self {
        Self {
            anchor: OverlayAnchor::Panel {
                kind,
                h,
                v,
                offset_x: 0.0,
                offset_y: 0.0,
            },
            width: OverlayExtent::default(),
            height: OverlayExtent::default(),
        }
    }

    fn panel_key(key: PanelKey, h: HAlign, v: VAlign) -> Self {
        Self {
            anchor: OverlayAnchor::PanelAnchor {
                key,
                h,
                v,
                offset_x: 0.0,
                offset_y: 0.0,
            },
            width: OverlayExtent::default(),
            height: OverlayExtent::default(),
        }
    }

    overlay_ctors!(viewport center ()       => "Centered in the viewport.", HAlign::Center, VAlign::Center, 0.0, 0.0);
    overlay_ctors!(viewport top (margin)    => "Top-center with vertical margin.", HAlign::Center, VAlign::Top, 0.0);
    overlay_ctors!(viewport bottom (margin) => "Bottom-center with vertical margin.", HAlign::Center, VAlign::Bottom, 0.0);
    overlay_ctors!(viewport top_left (mx, my)     => "Top-left corner with margins.", HAlign::Left, VAlign::Top);
    overlay_ctors!(viewport top_right (mx, my)    => "Top-right corner with margins.", HAlign::Right, VAlign::Top);
    overlay_ctors!(viewport bottom_left (mx, my)  => "Bottom-left corner with margins.", HAlign::Left, VAlign::Bottom);
    overlay_ctors!(viewport bottom_right (mx, my) => "Bottom-right corner with margins.", HAlign::Right, VAlign::Bottom);

    overlay_ctors!(panel above    => "Anchored above a panel (by kind). Rejects ambiguous kinds.", HAlign::Center, VAlign::Top);
    overlay_ctors!(panel below    => "Anchored below a panel (by kind). Rejects ambiguous kinds.", HAlign::Center, VAlign::Bottom);
    overlay_ctors!(panel left_of  => "Anchored to the left of a panel (by kind). Rejects ambiguous kinds.", HAlign::Left, VAlign::Center);
    overlay_ctors!(panel right_of => "Anchored to the right of a panel (by kind). Rejects ambiguous kinds.", HAlign::Right, VAlign::Center);

    /// Anchored above a panel identified by its stable [`PanelKey`].
    pub fn above_key(key: PanelKey) -> Self {
        Self::panel_key(key, HAlign::Center, VAlign::Top)
    }

    /// Anchored below a panel identified by its stable [`PanelKey`].
    pub fn below_key(key: PanelKey) -> Self {
        Self::panel_key(key, HAlign::Center, VAlign::Bottom)
    }

    /// Anchored to the left of a panel identified by its stable [`PanelKey`].
    pub fn left_of_key(key: PanelKey) -> Self {
        Self::panel_key(key, HAlign::Left, VAlign::Center)
    }

    /// Anchored to the right of a panel identified by its stable [`PanelKey`].
    pub fn right_of_key(key: PanelKey) -> Self {
        Self::panel_key(key, HAlign::Right, VAlign::Center)
    }

    /// Set the anchor displacement.
    ///
    /// For panel-anchored overlays, this sets the panel-relative offset.
    /// For viewport-anchored overlays, this rewrites the viewport margins.
    pub fn offset(mut self, x: f32, y: f32) -> Self {
        match &mut self.anchor {
            OverlayAnchor::Panel {
                offset_x, offset_y, ..
            }
            | OverlayAnchor::PanelAnchor {
                offset_x, offset_y, ..
            } => {
                *offset_x = x;
                *offset_y = y;
            }
            OverlayAnchor::Viewport {
                margin_x, margin_y, ..
            } => {
                *margin_x = x;
                *margin_y = y;
            }
        }
        self
    }

    /// Fixed size on both axes.
    pub fn fixed(mut self, w: f32, h: f32) -> Self {
        self.width.value = ExtentValue::Fixed(w);
        self.height.value = ExtentValue::Fixed(h);
        self
    }

    /// Fixed width.
    pub fn width(mut self, w: f32) -> Self {
        self.width.value = ExtentValue::Fixed(w);
        self
    }

    /// Fixed height.
    pub fn height(mut self, h: f32) -> Self {
        self.height.value = ExtentValue::Fixed(h);
        self
    }

    /// Full viewport width.
    pub fn full_width(mut self) -> Self {
        self.width.value = ExtentValue::Full;
        self
    }

    /// Full viewport height.
    pub fn full_height(mut self) -> Self {
        self.height.value = ExtentValue::Full;
        self
    }

    /// Percentage of viewport width.
    ///
    /// Values above `100.0` are allowed and produce widths larger than the
    /// viewport before any optional clamp is applied.
    pub fn percent_width(mut self, pct: f32) -> Self {
        self.width.value = ExtentValue::Percent(pct);
        self
    }

    /// Percentage of viewport height.
    ///
    /// Values above `100.0` are allowed and produce heights larger than the
    /// viewport before any optional clamp is applied.
    pub fn percent_height(mut self, pct: f32) -> Self {
        self.height.value = ExtentValue::Percent(pct);
        self
    }

    /// Clamp width to a range (in pixels).
    pub fn clamp_width(mut self, min: f32, max: f32) -> Self {
        self.width.min = Some(min);
        self.width.max = Some(max);
        self
    }

    /// Clamp height to a range (in pixels).
    pub fn clamp_height(mut self, min: f32, max: f32) -> Self {
        self.height.min = Some(min);
        self.height.max = Some(max);
        self
    }

    /// Validate all float fields in the overlay builder.
    pub(crate) fn validate(&self) -> Result<(), crate::error::PaneError> {
        use crate::error::PaneError;
        use crate::validate::{
            check_f32_finite, check_f32_non_negative, float_invalid_to_constraint,
        };

        let map = |name: &'static str, e| {
            PaneError::InvalidConstraint(float_invalid_to_constraint(name, e))
        };

        match &self.anchor {
            OverlayAnchor::Viewport {
                margin_x, margin_y, ..
            } => {
                check_f32_finite(*margin_x).map_err(|e| map("overlay_margin_x", e))?;
                check_f32_finite(*margin_y).map_err(|e| map("overlay_margin_y", e))?;
            }
            OverlayAnchor::Panel {
                offset_x, offset_y, ..
            }
            | OverlayAnchor::PanelAnchor {
                offset_x, offset_y, ..
            } => {
                check_f32_finite(*offset_x).map_err(|e| map("overlay_offset_x", e))?;
                check_f32_finite(*offset_y).map_err(|e| map("overlay_offset_y", e))?;
            }
        }

        validate_extent(
            &self.width,
            "overlay_width",
            check_f32_non_negative,
            float_invalid_to_constraint,
        )?;
        validate_extent(
            &self.height,
            "overlay_height",
            check_f32_non_negative,
            float_invalid_to_constraint,
        )?;

        Ok(())
    }

    /// Consume the builder and produce an `OverlayDef` with the given id and kind.
    pub(crate) fn into_def(self, id: OverlayId, kind: Arc<str>) -> OverlayDef {
        OverlayDef {
            id,
            kind,
            anchor: self.anchor,
            width: self.width,
            height: self.height,
            visible: true,
        }
    }
}
