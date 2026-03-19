//! Convert panes layouts into `ratatui::layout::Rect` with pixel-perfect edge rounding.

use panes::{OverlayEntry, PanelEntry, PanelId, ResolvedLayout};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Clear;
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
    resolved.panels().map(|e| e.map_rect(quantize))
}

/// Iterate all panels with quantized rects offset by a parent rect's origin.
///
/// Suitable for rendering a panes layout inside another panes panel.
pub fn panels_at(
    resolved: &ResolvedLayout,
    origin: Rect,
) -> impl Iterator<Item = PanelEntry<'_, Rect>> {
    resolved.panels().map(move |e| {
        e.map_rect(|r| {
            let mut rect = quantize(r);
            rect.x += origin.x;
            rect.y += origin.y;
            rect
        })
    })
}

/// Iterate all panels with focus state derived from the given focused panel.
///
/// Each item pairs a quantized `PanelEntry` with a `bool` indicating focus.
/// A panel is focused when its id matches `focused`, or when its kind is a
/// decoration (`_tab` / `_title` suffix) of the focused panel's kind.
pub fn focused_panels<'a>(
    resolved: &'a ResolvedLayout,
    focused: Option<PanelId>,
) -> impl Iterator<Item = (PanelEntry<'a, Rect>, bool)> {
    let focused_kind = focused.and_then(|fid| {
        resolved
            .kinds()
            .find(|kind| resolved.by_kind(kind).contains(&fid))
    });

    resolved.panels().map(move |e| {
        let is_focused = match (focused, focused_kind) {
            (Some(fid), _) if e.id == fid => true,
            (_, Some(fk)) => e
                .kind
                .strip_suffix("_tab")
                .or_else(|| e.kind.strip_suffix("_title"))
                .is_some_and(|base| base == fk),
            _ => false,
        };
        (e.map_rect(quantize), is_focused)
    })
}

/// Iterate all panels with focus state, offset by a parent rect's origin.
///
/// Combines the focus logic of [`focused_panels`] with the origin offset of
/// [`panels_at`].
pub fn focused_panels_at<'a>(
    resolved: &'a ResolvedLayout,
    focused: Option<PanelId>,
    origin: Rect,
) -> impl Iterator<Item = (PanelEntry<'a, Rect>, bool)> {
    let focused_kind = focused.and_then(|fid| {
        resolved
            .kinds()
            .find(|kind| resolved.by_kind(kind).contains(&fid))
    });

    resolved.panels().map(move |e| {
        let is_focused = match (focused, focused_kind) {
            (Some(fid), _) if e.id == fid => true,
            (_, Some(fk)) => e
                .kind
                .strip_suffix("_tab")
                .or_else(|| e.kind.strip_suffix("_title"))
                .is_some_and(|base| base == fk),
            _ => false,
        };
        let entry = e.map_rect(|r| {
            let mut rect = quantize(r);
            rect.x += origin.x;
            rect.y += origin.y;
            rect
        });
        (entry, is_focused)
    })
}

/// Iterate all resolved overlays, yielding identity and quantized ratatui rect.
pub fn overlays(resolved: &ResolvedLayout) -> impl Iterator<Item = OverlayEntry<'_, Rect>> {
    resolved.overlays().map(|e| e.map_rect(quantize))
}

/// Iterate all resolved overlays with quantized rects offset by a parent rect's origin.
///
/// Suitable for rendering overlays inside a nested panes layout.
pub fn overlays_at(
    resolved: &ResolvedLayout,
    origin: Rect,
) -> impl Iterator<Item = OverlayEntry<'_, Rect>> {
    resolved.overlays().map(move |e| {
        e.map_rect(|r| {
            let mut rect = quantize(r);
            rect.x += origin.x;
            rect.y += origin.y;
            rect
        })
    })
}

/// Clear underlying cells and render each overlay via the callback.
///
/// Overlays sit above panels. Without clearing, panel borders and content
/// bleed through. This function handles the clear automatically — call it
/// after rendering panels.
pub fn render_overlays(
    frame: &mut Frame,
    resolved: &ResolvedLayout,
    mut render: impl FnMut(&mut Frame, OverlayEntry<'_, Rect>),
) {
    for entry in overlays(resolved) {
        frame.render_widget(Clear, entry.rect);
        render(frame, entry);
    }
}

/// Clear underlying cells and render each overlay, offset by a parent origin.
///
/// Combines [`overlays_at`] origin translation with automatic cell clearing.
pub fn render_overlays_at(
    frame: &mut Frame,
    resolved: &ResolvedLayout,
    origin: Rect,
    mut render: impl FnMut(&mut Frame, OverlayEntry<'_, Rect>),
) {
    for entry in overlays_at(resolved, origin) {
        frame.render_widget(Clear, entry.rect);
        render(frame, entry);
    }
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
