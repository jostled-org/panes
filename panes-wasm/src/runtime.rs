use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{Layout, PanelId, Rect};
#[cfg(feature = "js")]
use wasm_bindgen::prelude::*;

use crate::layout::WasmLayout;

/// Stateful layout runtime for JavaScript consumers.
///
/// Wraps [`LayoutRuntime`] with f64 coordinate boundaries.
#[cfg_attr(feature = "js", wasm_bindgen)]
pub struct WasmRuntime {
    inner: LayoutRuntime,
}

/// Error conversion: PaneError → String for non-wasm, JsError for wasm.
fn err_string(e: panes::PaneError) -> String {
    e.to_string()
}

/// Serialize a rect change (id + from/to rects) to a JSON value with f64 fields.
fn rect_change_json(id: u32, from: Rect, to: Rect) -> serde_json::Value {
    let f = crate::WasmRect::from(from);
    let t = crate::WasmRect::from(to);
    serde_json::json!({
        "id": id,
        "from": { "x": f.x, "y": f.y, "w": f.w, "h": f.h },
        "to": { "x": t.x, "y": t.y, "w": t.w, "h": t.h },
    })
}

/// Serialize a diff (layout or overlay) to a JSON string.
///
/// Generic over the ID type (PanelId or OverlayId) and rect-change type.
fn diff_to_json<I: Copy, C>(
    ids_added: &[I],
    ids_removed: &[I],
    moved: &[C],
    resized: &[C],
    ids_unchanged: &[I],
    raw_id: fn(I) -> u32,
    change_json: fn(&C) -> serde_json::Value,
) -> String {
    let added: Vec<u32> = ids_added.iter().map(|id| raw_id(*id)).collect();
    let removed: Vec<u32> = ids_removed.iter().map(|id| raw_id(*id)).collect();
    let moved: Vec<serde_json::Value> = moved.iter().map(change_json).collect();
    let resized: Vec<serde_json::Value> = resized.iter().map(change_json).collect();
    let unchanged: Vec<u32> = ids_unchanged.iter().map(|id| raw_id(*id)).collect();
    serde_json::json!({
        "added": added,
        "removed": removed,
        "moved": moved,
        "resized": resized,
        "unchanged": unchanged,
    })
    .to_string()
}

impl WasmRuntime {
    /// Construct from a preset name and panel kind strings.
    ///
    /// Supports all `DynamicList` presets (master-stack, monocle, dwindle, etc.).
    pub fn from_preset(preset: &str, panels: &[&str]) -> Result<Self, String> {
        let kinds: Vec<Arc<str>> = panels.iter().map(|s| Arc::from(*s)).collect();
        let rt = build_from_preset(preset, &kinds)?;
        Ok(Self { inner: rt })
    }

    /// Resolve the layout at the given viewport dimensions.
    pub fn resolve(&mut self, width: f64, height: f64) -> Result<WasmLayout, String> {
        let frame = self
            .inner
            .resolve(width as f32, height as f32)
            .map_err(err_string)?;
        Ok(WasmLayout::new(frame.arc()))
    }

    /// Add a panel by kind string, returning its raw id.
    pub fn add_panel(&mut self, kind: &str) -> Result<u32, String> {
        let pid = self.inner.add_panel(Arc::from(kind)).map_err(err_string)?;
        Ok(pid.raw())
    }

    /// Remove a panel by raw id.
    pub fn remove_panel(&mut self, pid: u32) -> Result<(), String> {
        self.inner
            .remove_panel(PanelId::from_raw(pid))
            .map_err(err_string)?;
        Ok(())
    }

    /// The currently focused panel's raw id, if any.
    pub fn focused(&self) -> Option<u32> {
        self.inner.focused().map(PanelId::raw)
    }

    /// Move focus to the next panel in the sequence.
    pub fn focus_next(&mut self) {
        self.inner.focus_next();
    }

    /// Move focus to the previous panel in the sequence.
    pub fn focus_prev(&mut self) {
        self.inner.focus_prev();
    }

    /// Set a panel's intrinsic size (f64 → f32 at boundary).
    pub fn set_panel_size(&mut self, pid: u32, width: f64, height: f64) -> Result<(), String> {
        self.inner
            .set_panel_size(PanelId::from_raw(pid), width as f32, height as f32)
            .map_err(err_string)
    }

    /// Clear a panel's intrinsic size, reverting to constraint-based layout.
    pub fn clear_panel_size(&mut self, pid: u32) -> Result<(), String> {
        self.inner
            .clear_panel_size(PanelId::from_raw(pid))
            .map_err(err_string)
    }

    /// Current scroll offset as f64.
    pub fn scroll_offset(&self) -> f64 {
        f64::from(self.inner.viewport().scroll_offset)
    }

    /// Set the scroll offset to an absolute value (f64 → f32 at boundary).
    pub fn set_scroll_offset(&mut self, offset: f64) -> Result<(), String> {
        self.inner.scroll_to(offset as f32).map_err(err_string)
    }

    /// Shift the scroll offset by a delta (f64 → f32 at boundary).
    pub fn scroll_by(&mut self, delta: f64) -> Result<(), String> {
        self.inner.scroll_by(delta as f32).map_err(err_string)
    }

    /// Serialize the last layout diff to JSON.
    ///
    /// Returns `{"added":[...],"removed":[...],"moved":[...],"resized":[...],"unchanged":[...]}`.
    /// Panel IDs are u32, rects have f64 fields.
    pub fn layout_diff(&self) -> String {
        let diff = self.inner.last_diff();
        diff_to_json(
            diff.added,
            diff.removed,
            diff.moved,
            diff.resized,
            diff.unchanged,
            PanelId::raw,
            |c| rect_change_json(c.id.raw(), c.from, c.to),
        )
    }

    /// Serialize the last overlay diff to JSON.
    ///
    /// Same shape as `layout_diff()` but for overlay IDs.
    pub fn overlay_diff(&self) -> String {
        let diff = self.inner.last_overlay_diff();
        diff_to_json(
            diff.added,
            diff.removed,
            diff.moved,
            diff.resized,
            diff.unchanged,
            panes::OverlayId::raw,
            |c| rect_change_json(c.id.raw(), c.from, c.to),
        )
    }
}

/// Build a `LayoutRuntime` from a preset name and panel kinds.
fn build_from_preset(preset: &str, kinds: &[Arc<str>]) -> Result<LayoutRuntime, String> {
    let iter = || kinds.iter().cloned();
    match preset {
        "master-stack" => Layout::master_stack(iter()).into_runtime(),
        "centered-master" => Layout::centered_master(iter()).into_runtime(),
        "monocle" => Layout::monocle(iter()).into_runtime(),
        "scrollable" => Layout::scrollable(iter()).into_runtime(),
        "dwindle" => Layout::dwindle(iter()).into_runtime(),
        "spiral" => Layout::spiral(iter()).into_runtime(),
        "deck" => Layout::deck(iter()).into_runtime(),
        "tabbed" => Layout::tabbed(iter()).into_runtime(),
        "stacked" => Layout::stacked(iter()).into_runtime(),
        _ => return Err(format!("unknown preset: {preset}")),
    }
    .map_err(err_string)
}
