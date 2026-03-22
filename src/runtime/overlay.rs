use std::sync::Arc;

use super::types::LayoutRuntime;
use crate::error::PaneError;
use crate::overlay::{self, Overlay, OverlayDef, OverlayId};
use crate::validate::{check_f32_non_negative, float_invalid_to_constraint};

impl LayoutRuntime {
    /// Add an overlay. Returns the existing id if the kind already exists.
    pub fn add_overlay(
        &mut self,
        kind: impl Into<Arc<str>>,
        builder: Overlay,
    ) -> Result<OverlayId, PaneError> {
        crate::runtime_overlay::add_overlay_impl(
            &mut self.overlays,
            &mut self.overlay_index,
            &mut self.overlay_gen,
            kind.into(),
            builder,
        )
    }

    /// Remove an overlay by kind. No-op if the kind is not found.
    pub fn remove_overlay(&mut self, kind: &str) {
        crate::runtime_overlay::remove_overlay_impl(
            &mut self.overlays,
            &mut self.overlay_index,
            kind,
        );
    }

    /// Show or hide an overlay without removing it.
    pub fn set_overlay_visible(&mut self, kind: &str, visible: bool) {
        if let Some(&idx) = self.overlay_index.get(kind) {
            self.overlays[idx].visible = visible;
        }
    }

    /// Update an overlay's height (fixed value).
    pub fn set_overlay_height(&mut self, kind: &str, h: f32) -> Result<(), PaneError> {
        validate_overlay_dimension("overlay_height", h)?;
        if let Some(&idx) = self.overlay_index.get(kind) {
            self.overlays[idx].height.value = overlay::ExtentValue::Fixed(h);
        }
        Ok(())
    }

    /// Update an overlay's width (fixed value).
    pub fn set_overlay_width(&mut self, kind: &str, w: f32) -> Result<(), PaneError> {
        validate_overlay_dimension("overlay_width", w)?;
        if let Some(&idx) = self.overlay_index.get(kind) {
            self.overlays[idx].width.value = overlay::ExtentValue::Fixed(w);
        }
        Ok(())
    }

    /// Look up an overlay definition by kind.
    pub fn overlay(&self, kind: &str) -> Option<&OverlayDef> {
        self.overlay_index.get(kind).map(|&idx| &self.overlays[idx])
    }

    /// All overlay definitions.
    pub fn overlays(&self) -> &[OverlayDef] {
        &self.overlays
    }
}

fn validate_overlay_dimension(name: &'static str, value: f32) -> Result<(), PaneError> {
    check_f32_non_negative(value)
        .map_err(|e| PaneError::InvalidConstraint(float_invalid_to_constraint(name, e)))
}
