// panes-egui — egui adapter for panes layout engine

use panes::{PanelId, ResolvedLayout};
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
