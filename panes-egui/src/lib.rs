//! Convert panes layouts into `egui::Rect` values.

use panes::{PanelEntry, PanelId, ResolvedLayout};
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
