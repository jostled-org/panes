//! Convert panes layouts into `ratatui::layout::Rect` with pixel-perfect edge rounding.

use panes::{PanelEntry, PanelId, ResolvedLayout};
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

/// Convert a resolved layout into ratatui rects offset by a parent rect's origin.
///
/// Panels are positioned relative to `origin.x` and `origin.y`, making this
/// suitable for rendering a panes layout inside another panes panel.
pub fn convert_at(resolved: &ResolvedLayout, origin: Rect) -> FxHashMap<PanelId, Rect> {
    resolved
        .iter()
        .map(|(pid, r)| {
            let mut rect = quantize(r);
            rect.x += origin.x;
            rect.y += origin.y;
            (pid, rect)
        })
        .collect()
}

/// Iterate all panels in kind-grouped order, yielding identity and quantized rect.
///
/// No hashmap allocation — produces entries lazily from the resolved layout.
pub fn panels(resolved: &ResolvedLayout) -> impl Iterator<Item = PanelEntry<'_, Rect>> {
    resolved.panels().map(|e| PanelEntry {
        id: e.id,
        kind: e.kind,
        rect: quantize(e.rect),
    })
}

/// Iterate all panels with quantized rects offset by a parent rect's origin.
///
/// Suitable for rendering a panes layout inside another panes panel.
pub fn panels_at(
    resolved: &ResolvedLayout,
    origin: Rect,
) -> impl Iterator<Item = PanelEntry<'_, Rect>> {
    resolved.panels().map(move |e| {
        let mut rect = quantize(e.rect);
        rect.x += origin.x;
        rect.y += origin.y;
        PanelEntry {
            id: e.id,
            kind: e.kind,
            rect,
        }
    })
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
