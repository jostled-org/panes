//! Convert panes layouts into `egui::Rect` values.
//!
//! Primary API: [`resolve`] wraps a runtime resolve into an [`EguiFrame`]
//! with `egui::Rect` panels. Existing free functions remain for manual use.

use panes::diff::LayoutDiff;
use panes::runtime::{Frame as PanesFrame, LayoutRuntime};
use panes::{AdapterFrame, Layout, OverlayEntry, PaneError, PanelEntry, PanelId};

// ---------------------------------------------------------------------------
// EguiFrame
// ---------------------------------------------------------------------------

/// Resolved layout with `egui::Rect` panels.
///
/// Created via [`resolve`] or [`resolve_layout`]. Provides
/// [`diff`](Self::diff) and [`inner`](Self::inner) for runtime state access.
pub struct EguiFrame<'a> {
    shell: AdapterFrame<'a>,
}

impl<'a> EguiFrame<'a> {
    /// `egui::Rect` for a panel.
    pub fn get(&self, id: PanelId) -> Option<egui::Rect> {
        self.shell.resolved().get(id).map(to_egui_rect)
    }

    /// All panels with `egui::Rect` values, in kind-grouped order.
    pub fn panels(&self) -> impl Iterator<Item = PanelEntry<'_, egui::Rect>> {
        self.shell
            .resolved()
            .panels()
            .map(|e| e.map_rect(to_egui_rect))
    }

    /// Overlays with `egui::Rect` values.
    pub fn overlays(&self) -> impl Iterator<Item = OverlayEntry<'_, egui::Rect>> {
        self.shell
            .resolved()
            .overlays()
            .map(|e| e.map_rect(to_egui_rect))
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

/// Resolve a runtime and convert to `egui::Rect` in one step.
///
/// Calls [`LayoutRuntime::resolve`] then wraps the result in an [`EguiFrame`].
/// The returned frame borrows the runtime for [`diff`](EguiFrame::diff) access.
pub fn resolve<'a>(
    rt: &'a mut LayoutRuntime,
    area: egui::Rect,
) -> Result<EguiFrame<'a>, PaneError> {
    let frame = rt.resolve(area.width(), area.height())?;
    Ok(EguiFrame {
        shell: AdapterFrame::from_runtime(frame, rt),
    })
}

/// Resolve a layout without runtime state.
///
/// Returns an [`EguiFrame`] with `egui::Rect` values.
/// [`diff`](EguiFrame::diff) and [`inner`](EguiFrame::inner) return `None`.
/// The `'static` lifetime reflects that the `Stateless` variant owns all data.
pub fn resolve_layout(layout: &Layout, area: egui::Rect) -> Result<EguiFrame<'static>, PaneError> {
    let resolved = layout.resolve(area.width(), area.height())?;
    Ok(EguiFrame {
        shell: AdapterFrame::from_stateless(resolved),
    })
}

// ---------------------------------------------------------------------------
// Shared conversion
// ---------------------------------------------------------------------------

fn to_egui_rect(r: &panes::Rect) -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(r.x, r.y), egui::vec2(r.w, r.h))
}

// ---------------------------------------------------------------------------
// Existing free functions (backward compatible)
// ---------------------------------------------------------------------------

panes::impl_adapter! {
    rect: egui::Rect,
    origin: egui::Pos2,
    convert_fn: to_egui_rect,
    convert_at_fn: |r: &panes::Rect, origin: egui::Pos2| {
        to_egui_rect(r).translate(egui::vec2(origin.x, origin.y))
    },
}
