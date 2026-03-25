//! Convert panes layouts into `ratatui::layout::Rect` with pixel-perfect edge rounding.

use panes::{OverlayEntry, PanelEntry, PanelId, ResolvedLayout};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Clear;

panes::impl_adapter! {
    rect: Rect,
    origin: Rect,
    convert_fn: quantize,
    convert_at_fn: |r, origin: Rect| {
        let mut rect = quantize(r);
        rect.x += origin.x;
        rect.y += origin.y;
        rect
    },
}

pub fn convert_at(resolved: &ResolvedLayout, origin: Rect) -> panes::__FxHashMap<PanelId, Rect> {
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

/// Clear underlying cells before rendering each overlay. Call after panels.
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

/// Edge-rounding quantization: each edge is rounded independently,
/// so adjacent panels sharing a float edge produce the same integer.
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

fn clamp_edge(v: f32) -> u16 {
    match v {
        v if v <= 0.0 => 0,
        v if v >= f32::from(u16::MAX) => u16::MAX,
        _ => v as u16,
    }
}
