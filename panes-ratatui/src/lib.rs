//! Convert panes layouts into `ratatui::layout::Rect` with pixel-perfect edge rounding.
//!
//! Primary API: [`resolve`] wraps a runtime resolve into a [`TerminalFrame`]
//! with quantized u16 rects. Existing free functions remain for manual use.

use panes::diff::LayoutDiff;
use panes::runtime::{Frame as PanesFrame, LayoutRuntime};
use panes::{AdapterFrame, Layout, OverlayEntry, PaneError, PanelEntry, PanelId, ResolvedLayout};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Clear;

// ---------------------------------------------------------------------------
// TerminalFrame
// ---------------------------------------------------------------------------

/// Resolved layout with rects quantized to terminal cells (u16).
///
/// Created via [`resolve`]. Provides [`diff`](Self::diff) and [`inner`](Self::inner)
/// for access to the underlying runtime state.
pub struct TerminalFrame<'a> {
    shell: AdapterFrame<'a>,
}

impl<'a> TerminalFrame<'a> {
    /// Quantized rect for a panel.
    pub fn get(&self, id: PanelId) -> Option<Rect> {
        self.shell.resolved().get(id).map(quantize)
    }

    /// All panels with quantized rects, in kind-grouped order.
    pub fn panels(&self) -> impl Iterator<Item = PanelEntry<'_, Rect>> {
        self.shell.resolved().panels().map(|e| e.map_rect(quantize))
    }

    /// Overlays with quantized rects.
    pub fn overlays(&self) -> impl Iterator<Item = OverlayEntry<'_, Rect>> {
        self.shell
            .resolved()
            .overlays()
            .map(|e| e.map_rect(quantize))
    }

    /// Panels with focus flag and quantized rects.
    ///
    /// A panel is focused when its id matches `focused`, or when it is a
    /// decoration panel whose content kind matches the focused panel's kind.
    pub fn focused_panels(
        &self,
        focused: Option<PanelId>,
    ) -> impl Iterator<Item = (PanelEntry<'_, Rect>, bool)> {
        focused_panels_impl(self.shell.resolved(), focused)
    }

    /// Clear underlying cells then render each overlay. Call after panels.
    pub fn render_overlays(
        &self,
        frame: &mut Frame,
        render: impl FnMut(&mut Frame, OverlayEntry<'_, Rect>),
    ) {
        render_overlays_impl(frame, self.overlays(), render);
    }

    /// Layout diff (panel IDs, not rects). `None` for stateless resolves.
    pub fn diff(&self) -> Option<LayoutDiff<'_>> {
        self.shell.diff()
    }

    /// Raw panes `Frame`. `None` for stateless resolves.
    pub fn inner(&self) -> Option<&PanesFrame> {
        self.shell.inner()
    }

    /// Overlays that failed to anchor during the most recent resolve.
    pub fn overlay_failures(
        &self,
    ) -> &[(panes::OverlayId, std::sync::Arc<str>, panes::AnchorFailure)] {
        self.shell.overlay_failures()
    }
}

/// Resolve a runtime and quantize in one step.
///
/// Calls [`LayoutRuntime::resolve`] then wraps the result in a [`TerminalFrame`]
/// with quantized u16 rects. The returned frame borrows the runtime for
/// [`diff`](TerminalFrame::diff) access.
pub fn resolve<'a>(rt: &'a mut LayoutRuntime, area: Rect) -> Result<TerminalFrame<'a>, PaneError> {
    let frame = rt.resolve(f32::from(area.width), f32::from(area.height))?;
    Ok(TerminalFrame {
        shell: AdapterFrame::from_runtime(frame, rt),
    })
}

/// Resolve a layout without runtime state.
///
/// Returns a [`TerminalFrame`] with quantized rects. [`diff`](TerminalFrame::diff)
/// and [`inner`](TerminalFrame::inner) return `None`. The `'static` lifetime
/// reflects that the `Stateless` variant owns all its data.
pub fn resolve_layout(layout: &Layout, area: Rect) -> Result<TerminalFrame<'static>, PaneError> {
    let resolved = layout.resolve(f32::from(area.width), f32::from(area.height))?;
    Ok(TerminalFrame {
        shell: AdapterFrame::from_stateless(resolved),
    })
}

// ---------------------------------------------------------------------------
// Existing free functions (backward compatible)
// ---------------------------------------------------------------------------

panes::impl_adapter! {
    rect: Rect,
    origin: Rect,
    convert_fn: quantize,
    convert_at_fn: |r, origin: Rect| offset_rect(quantize(r), origin),
}

pub fn convert_at(resolved: &ResolvedLayout, origin: Rect) -> panes::__FxHashMap<PanelId, Rect> {
    resolved
        .iter()
        .map(|(pid, r)| (pid, offset_rect(quantize(r), origin)))
        .collect()
}

/// A panel is focused when its id matches `focused`, or when it is a
/// decoration panel whose content kind matches the focused panel's kind.
pub fn focused_panels<'a>(
    resolved: &'a ResolvedLayout,
    focused: Option<PanelId>,
) -> impl Iterator<Item = (PanelEntry<'a, Rect>, bool)> {
    focused_panels_impl(resolved, focused)
}

pub fn focused_panels_at<'a>(
    resolved: &'a ResolvedLayout,
    focused: Option<PanelId>,
    origin: Rect,
) -> impl Iterator<Item = (PanelEntry<'a, Rect>, bool)> {
    focused_panels_impl(resolved, focused)
        .map(move |(e, is_focused)| (e.map_rect(|r| offset_rect(r, origin)), is_focused))
}

fn focused_panels_impl<'a>(
    resolved: &'a ResolvedLayout,
    focused: Option<PanelId>,
) -> impl Iterator<Item = (PanelEntry<'a, Rect>, bool)> {
    let focused_kind = focused.and_then(|pid| resolved.kind_of(pid));

    let content = resolved.panels().map(move |entry| {
        let is_focused = matches!(focused, Some(fid) if entry.id == fid);
        (entry.map_rect(quantize), is_focused)
    });

    let decorations = resolved.decoration_panels().iter().filter_map(move |d| {
        let rect = resolved.get(d.id)?;
        let is_focused = focused_kind.is_some_and(|fk| fk == d.content_kind.as_ref());
        let kind_index = resolved.kind_index_of(d.content_kind.as_ref())?;
        Some((
            PanelEntry {
                id: d.id,
                kind: &d.content_kind,
                rect: quantize(rect),
                kind_index,
            },
            is_focused,
        ))
    });

    content.chain(decorations)
}

/// Clear underlying cells before rendering each overlay. Call after panels.
pub fn render_overlays(
    frame: &mut Frame,
    resolved: &ResolvedLayout,
    render: impl FnMut(&mut Frame, OverlayEntry<'_, Rect>),
) {
    render_overlays_impl(frame, overlays(resolved), render);
}

pub fn render_overlays_at(
    frame: &mut Frame,
    resolved: &ResolvedLayout,
    origin: Rect,
    render: impl FnMut(&mut Frame, OverlayEntry<'_, Rect>),
) {
    render_overlays_impl(frame, overlays_at(resolved, origin), render);
}

fn render_overlays_impl<'a>(
    frame: &mut Frame,
    overlays: impl Iterator<Item = OverlayEntry<'a, Rect>>,
    mut render: impl FnMut(&mut Frame, OverlayEntry<'a, Rect>),
) {
    for entry in overlays {
        frame.render_widget(Clear, entry.rect);
        render(frame, entry);
    }
}

fn offset_rect(r: Rect, origin: Rect) -> Rect {
    Rect {
        x: r.x + origin.x,
        y: r.y + origin.y,
        ..r
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
