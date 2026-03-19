//! Convert panes layouts into `egui::Rect` values.

use panes::{OverlayEntry, PanelEntry, PanelId, ResolvedLayout};
use rustc_hash::FxHashMap;

/// Convert a resolved panes layout into egui rects.
///
/// Direct f32 mapping — no rounding or quantization.
pub fn convert(resolved: &ResolvedLayout) -> FxHashMap<PanelId, egui::Rect> {
    resolved
        .iter()
        .map(|(pid, r)| {
            let rect = egui::Rect::from_min_size(egui::pos2(r.x, r.y), egui::vec2(r.w, r.h));
            (pid, rect)
        })
        .collect()
}

/// Iterate all panels in kind-grouped order, yielding identity and egui rect.
///
/// No hashmap allocation — produces entries lazily from the resolved layout.
pub fn panels(resolved: &ResolvedLayout) -> impl Iterator<Item = PanelEntry<'_, egui::Rect>> {
    resolved.panels().map(|e| {
        e.map_rect(|r| egui::Rect::from_min_size(egui::pos2(r.x, r.y), egui::vec2(r.w, r.h)))
    })
}

/// Iterate all resolved overlays, yielding identity and egui rect.
pub fn overlays(resolved: &ResolvedLayout) -> impl Iterator<Item = OverlayEntry<'_, egui::Rect>> {
    resolved.overlays().map(|e| {
        e.map_rect(|r| egui::Rect::from_min_size(egui::pos2(r.x, r.y), egui::vec2(r.w, r.h)))
    })
}

/// Iterate all panels with egui rects offset by an origin position.
///
/// Suitable for rendering a panes layout inside a sub-region of the UI.
pub fn panels_at(
    resolved: &ResolvedLayout,
    origin: egui::Pos2,
) -> impl Iterator<Item = PanelEntry<'_, egui::Rect>> {
    resolved.panels().map(move |e| {
        e.map_rect(|r| {
            egui::Rect::from_min_size(
                egui::pos2(r.x + origin.x, r.y + origin.y),
                egui::vec2(r.w, r.h),
            )
        })
    })
}

/// Iterate all resolved overlays with egui rects offset by an origin position.
///
/// Suitable for rendering overlays inside a sub-region of the UI.
pub fn overlays_at(
    resolved: &ResolvedLayout,
    origin: egui::Pos2,
) -> impl Iterator<Item = OverlayEntry<'_, egui::Rect>> {
    resolved.overlays().map(move |e| {
        e.map_rect(|r| {
            egui::Rect::from_min_size(
                egui::pos2(r.x + origin.x, r.y + origin.y),
                egui::vec2(r.w, r.h),
            )
        })
    })
}
