use std::sync::Arc;

use super::resolve::validate_extent;
use super::types::{
    ExtentValue, HAlign, OverlayAnchor, OverlayDef, OverlayExtent, OverlayId, VAlign,
};

/// Builder for constructing overlay definitions.
///
/// Created via static constructors like `Overlay::center()`, `Overlay::bottom()`, etc.
pub struct Overlay {
    anchor: OverlayAnchor,
    width: OverlayExtent,
    height: OverlayExtent,
}

impl Overlay {
    /// Centered in the viewport.
    pub fn center() -> Self {
        Self {
            anchor: OverlayAnchor::Viewport {
                h: HAlign::Center,
                v: VAlign::Center,
                margin_x: 0.0,
                margin_y: 0.0,
            },
            width: OverlayExtent::default(),
            height: OverlayExtent::default(),
        }
    }

    /// Top-center with vertical margin.
    pub fn top(margin: f32) -> Self {
        Self {
            anchor: OverlayAnchor::Viewport {
                h: HAlign::Center,
                v: VAlign::Top,
                margin_x: 0.0,
                margin_y: margin,
            },
            width: OverlayExtent::default(),
            height: OverlayExtent::default(),
        }
    }

    /// Bottom-center with vertical margin.
    pub fn bottom(margin: f32) -> Self {
        Self {
            anchor: OverlayAnchor::Viewport {
                h: HAlign::Center,
                v: VAlign::Bottom,
                margin_x: 0.0,
                margin_y: margin,
            },
            width: OverlayExtent::default(),
            height: OverlayExtent::default(),
        }
    }

    /// Top-left corner with margins.
    pub fn top_left(mx: f32, my: f32) -> Self {
        Self {
            anchor: OverlayAnchor::Viewport {
                h: HAlign::Left,
                v: VAlign::Top,
                margin_x: mx,
                margin_y: my,
            },
            width: OverlayExtent::default(),
            height: OverlayExtent::default(),
        }
    }

    /// Top-right corner with margins.
    pub fn top_right(mx: f32, my: f32) -> Self {
        Self {
            anchor: OverlayAnchor::Viewport {
                h: HAlign::Right,
                v: VAlign::Top,
                margin_x: mx,
                margin_y: my,
            },
            width: OverlayExtent::default(),
            height: OverlayExtent::default(),
        }
    }

    /// Bottom-left corner with margins.
    pub fn bottom_left(mx: f32, my: f32) -> Self {
        Self {
            anchor: OverlayAnchor::Viewport {
                h: HAlign::Left,
                v: VAlign::Bottom,
                margin_x: mx,
                margin_y: my,
            },
            width: OverlayExtent::default(),
            height: OverlayExtent::default(),
        }
    }

    /// Bottom-right corner with margins.
    pub fn bottom_right(mx: f32, my: f32) -> Self {
        Self {
            anchor: OverlayAnchor::Viewport {
                h: HAlign::Right,
                v: VAlign::Bottom,
                margin_x: mx,
                margin_y: my,
            },
            width: OverlayExtent::default(),
            height: OverlayExtent::default(),
        }
    }

    /// Anchored above a panel (by kind).
    pub fn above(kind: impl Into<Arc<str>>) -> Self {
        Self {
            anchor: OverlayAnchor::Panel {
                kind: kind.into(),
                h: HAlign::Center,
                v: VAlign::Top,
                offset_x: 0.0,
                offset_y: 0.0,
            },
            width: OverlayExtent::default(),
            height: OverlayExtent::default(),
        }
    }

    /// Anchored below a panel (by kind).
    pub fn below(kind: impl Into<Arc<str>>) -> Self {
        Self {
            anchor: OverlayAnchor::Panel {
                kind: kind.into(),
                h: HAlign::Center,
                v: VAlign::Bottom,
                offset_x: 0.0,
                offset_y: 0.0,
            },
            width: OverlayExtent::default(),
            height: OverlayExtent::default(),
        }
    }

    /// Anchored to the left of a panel (by kind).
    pub fn left_of(kind: impl Into<Arc<str>>) -> Self {
        Self {
            anchor: OverlayAnchor::Panel {
                kind: kind.into(),
                h: HAlign::Left,
                v: VAlign::Center,
                offset_x: 0.0,
                offset_y: 0.0,
            },
            width: OverlayExtent::default(),
            height: OverlayExtent::default(),
        }
    }

    /// Anchored to the right of a panel (by kind).
    pub fn right_of(kind: impl Into<Arc<str>>) -> Self {
        Self {
            anchor: OverlayAnchor::Panel {
                kind: kind.into(),
                h: HAlign::Right,
                v: VAlign::Center,
                offset_x: 0.0,
                offset_y: 0.0,
            },
            width: OverlayExtent::default(),
            height: OverlayExtent::default(),
        }
    }

    /// Set an offset (for panel-anchored overlays).
    pub fn offset(mut self, x: f32, y: f32) -> Self {
        match &mut self.anchor {
            OverlayAnchor::Panel {
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

    /// Percentage of viewport width (0.0–100.0).
    pub fn percent_width(mut self, pct: f32) -> Self {
        self.width.value = ExtentValue::Percent(pct);
        self
    }

    /// Percentage of viewport height (0.0–100.0).
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
