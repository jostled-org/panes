// panes-ratatui — ratatui adapter for panes layout engine

use panes::{PanelId, ResolvedLayout};
use ratatui::layout::Rect;
use rustc_hash::FxHashMap;

/// Convert a resolved panes layout into ratatui rects.
///
/// Uses edge-rounding quantization: each edge is rounded independently,
/// so adjacent panels sharing a float edge produce the same integer —
/// no gaps, no overlaps.
pub fn convert(resolved: &ResolvedLayout) -> FxHashMap<PanelId, Rect> {
    resolved.iter().map(|(pid, r)| (pid, quantize(r))).collect()
}

/// Round edges, not positions+sizes, to produce pixel-perfect u16 rects.
fn quantize(r: &panes::Rect) -> Rect {
    let left = clamp_edge(r.x.round());
    let top = clamp_edge(r.y.round());
    let right = clamp_edge((r.x + r.w).round());
    let bottom = clamp_edge((r.y + r.h).round());

    Rect {
        x: left,
        y: top,
        width: right.saturating_sub(left),
        height: bottom.saturating_sub(top),
    }
}

/// Clamp a rounded edge value to the u16 range.
fn clamp_edge(v: f32) -> u16 {
    match v {
        v if v <= 0.0 => 0,
        v if v >= f32::from(u16::MAX) => u16::MAX,
        _ => v as u16,
    }
}
