use super::types::{
    ExtentValue, HAlign, OverlayAnchor, OverlayDef, OverlayExtent, OverlayId, VAlign,
};
use crate::rect::Rect;
use crate::resolver::ResolvedLayout;

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

/// Validate an `OverlayExtent`'s value and min/max bounds.
pub(crate) fn validate_extent(
    extent: &OverlayExtent,
    name: &'static str,
    check: fn(f32) -> Result<(), crate::validate::FloatInvalid>,
    map_err: fn(&'static str, crate::validate::FloatInvalid) -> crate::error::ConstraintError,
) -> Result<(), crate::error::PaneError> {
    use crate::error::{ConstraintError, PaneError};

    match extent.value {
        ExtentValue::Fixed(v) | ExtentValue::Percent(v) => {
            check(v).map_err(|e| PaneError::InvalidConstraint(map_err(name, e)))?;
        }
        ExtentValue::Full => {}
    }

    if let Some(lo) = extent.min {
        check(lo).map_err(|e| PaneError::InvalidConstraint(map_err(name, e)))?;
    }
    if let Some(hi) = extent.max {
        check(hi).map_err(|e| PaneError::InvalidConstraint(map_err(name, e)))?;
    }
    match (extent.min, extent.max) {
        (Some(lo), Some(hi)) if lo > hi => {
            return Err(PaneError::InvalidConstraint(ConstraintError::MinExceedsMax));
        }
        _ => {}
    }

    Ok(())
}

/// Resolve an extent value against a viewport dimension.
fn resolve_extent(extent: &OverlayExtent, viewport: f32) -> f32 {
    let raw = match extent.value {
        ExtentValue::Fixed(v) => v,
        ExtentValue::Percent(pct) => viewport * pct / 100.0,
        ExtentValue::Full => viewport,
    };
    match (extent.min, extent.max) {
        (Some(lo), Some(hi)) => raw.clamp(lo, hi),
        (Some(lo), None) => raw.max(lo),
        (None, Some(hi)) => raw.min(hi),
        (None, None) => raw,
    }
}

/// Position along the horizontal axis.
fn align_h(align: HAlign, origin: f32, container: f32, size: f32, margin: f32) -> f32 {
    match align {
        HAlign::Left => origin + margin,
        HAlign::Center => origin + (container - size) / 2.0 + margin,
        HAlign::Right => origin + container - size - margin,
    }
}

/// Position along the vertical axis.
fn align_v(align: VAlign, origin: f32, container: f32, size: f32, margin: f32) -> f32 {
    match align {
        VAlign::Top => origin + margin,
        VAlign::Center => origin + (container - size) / 2.0 + margin,
        VAlign::Bottom => origin + container - size - margin,
    }
}

/// Resolve a single overlay to a rect, given viewport dimensions and the base layout.
pub(crate) fn resolve_overlay(
    def: &OverlayDef,
    vp_w: f32,
    vp_h: f32,
    base: &ResolvedLayout,
) -> Option<Rect> {
    let w = resolve_extent(&def.width, vp_w);
    let h = resolve_extent(&def.height, vp_h);

    let (x, y) = match &def.anchor {
        OverlayAnchor::Viewport {
            h: ha,
            v: va,
            margin_x,
            margin_y,
        } => {
            let x = align_h(*ha, 0.0, vp_w, w, *margin_x);
            let y = align_v(*va, 0.0, vp_h, h, *margin_y);
            (x, y)
        }
        OverlayAnchor::Panel {
            kind,
            h: ha,
            v: va,
            offset_x,
            offset_y,
        } => {
            let panel_id = base.by_kind(kind).first()?;
            let panel_rect = base.get(*panel_id)?;

            let x = align_h(*ha, panel_rect.x, panel_rect.w, w, *offset_x);
            let y = align_v(*va, panel_rect.y, panel_rect.h, h, *offset_y);
            (x, y)
        }
    };

    Some(Rect { x, y, w, h })
}
