use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::error::PaneError;
use crate::overlay::{self, AnchorFailure, Overlay, OverlayDef, OverlayId, OverlayIdGenerator};
use crate::resolver::ResolvedLayout;
use crate::sequence::PanelSequence;

/// Add an overlay. Returns the existing id if the kind already exists.
pub(crate) fn add_overlay_impl(
    overlays: &mut Vec<OverlayDef>,
    overlay_index: &mut FxHashMap<Arc<str>, usize>,
    overlay_gen: &mut OverlayIdGenerator,
    kind: Arc<str>,
    builder: Overlay,
) -> Result<OverlayId, PaneError> {
    if let Some(&idx) = overlay_index.get(&kind) {
        return Ok(overlays[idx].id);
    }
    builder.validate()?;
    let id = overlay_gen.next_id()?;
    let def = builder.into_def(id, Arc::clone(&kind));
    let idx = overlays.len();
    overlays.push(def);
    overlay_index.insert(kind, idx);
    Ok(id)
}

/// Remove an overlay by kind. No-op if the kind is not found.
pub(crate) fn remove_overlay_impl(
    overlays: &mut Vec<OverlayDef>,
    overlay_index: &mut FxHashMap<Arc<str>, usize>,
    kind: &str,
) {
    let Some(idx) = overlay_index.remove(kind) else {
        return;
    };
    overlays.remove(idx);
    for v in overlay_index.values_mut().filter(|v| **v > idx) {
        *v -= 1;
    }
}

/// Resolve visible overlay rects and store them in the layout.
pub(crate) fn resolve_overlays_impl(
    overlays: &[OverlayDef],
    overlay_rects_buf: &mut Vec<(OverlayId, Arc<str>, crate::rect::Rect)>,
    overlay_failures_buf: &mut Vec<(OverlayId, Arc<str>, AnchorFailure)>,
    sequence: &PanelSequence,
    layout: &mut ResolvedLayout,
    width: f32,
    height: f32,
) {
    overlay_rects_buf.clear();
    overlay_failures_buf.clear();
    for def in overlays.iter().filter(|d| d.visible) {
        match overlay::resolve_overlay(def, width, height, layout, sequence) {
            Ok(rect) => overlay_rects_buf.push((def.id, Arc::clone(&def.kind), rect)),
            Err(failure) => {
                overlay_failures_buf.push((def.id, Arc::clone(&def.kind), failure));
            }
        }
    }
    layout.swap_overlay_rects(overlay_rects_buf);
    layout.swap_overlay_failures(overlay_failures_buf);
}
