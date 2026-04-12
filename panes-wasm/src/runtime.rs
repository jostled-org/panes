use std::sync::Arc;

use panes::runtime::LayoutRuntime;
use panes::{Layout, PanelId, PresetInfo};
#[cfg(feature = "js")]
use wasm_bindgen::prelude::*;

use crate::json_types::{DiffJson, OverlayDiffJson, RectChangeJson, RectJson};
use crate::layout::WasmLayout;

/// Stateful layout runtime for JavaScript consumers.
///
/// Wraps [`LayoutRuntime`] with f64 coordinate boundaries.
#[cfg_attr(feature = "js", wasm_bindgen)]
pub struct WasmRuntime {
    inner: LayoutRuntime,
}

fn js_number_to_f32(value: f64, name: &str) -> Result<f32, String> {
    if !value.is_finite() {
        return Err(format!("{name} must be finite"));
    }

    if value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        return Err(format!("{name} is out of range for f32"));
    }

    Ok(value as f32)
}

fn err_string(e: panes::PaneError) -> String {
    e.to_string()
}

fn diff_to_json<I: Copy, C>(
    ids_added: &[I],
    ids_removed: &[I],
    moved: &[C],
    resized: &[C],
    ids_unchanged: &[I],
    raw_id: fn(I) -> u32,
    change_json: fn(&C) -> RectChangeJson,
) -> Result<String, String> {
    let diff = DiffJson {
        added: ids_added.iter().map(|id| raw_id(*id)).collect(),
        removed: ids_removed.iter().map(|id| raw_id(*id)).collect(),
        moved: moved.iter().map(change_json).collect(),
        resized: resized.iter().map(change_json).collect(),
        unchanged: ids_unchanged.iter().map(|id| raw_id(*id)).collect(),
    };
    serde_json::to_string(&diff).map_err(|e| e.to_string())
}

fn rect_change(id: u32, from: panes::Rect, to: panes::Rect) -> RectChangeJson {
    RectChangeJson {
        id,
        from: RectJson::from(from),
        to: RectJson::from(to),
    }
}

impl WasmRuntime {
    /// Return metadata for all available presets, sourced from the core catalog.
    pub fn available_presets() -> &'static [PresetInfo] {
        Layout::presets()
    }

    /// Construct from a preset name and panel kind strings.
    ///
    /// Supports all presets in the core catalog. `DynamicList` presets accept
    /// any number of panels. `FixedSlots` presets require the exact panel count
    /// for that layout (e.g. 2 for sidebar/split, 5 for holy-grail).
    pub fn from_preset(preset: &str, panels: &[&str]) -> Result<Self, String> {
        let rt = build_from_preset(preset, panels)?;
        Ok(Self { inner: rt })
    }

    pub fn resolve(&mut self, width: f64, height: f64) -> Result<WasmLayout, String> {
        let width = js_number_to_f32(width, "width")?;
        let height = js_number_to_f32(height, "height")?;
        let frame = self.inner.resolve(width, height).map_err(err_string)?;
        Ok(WasmLayout::new(frame.arc()))
    }

    pub fn add_panel(&mut self, kind: &str) -> Result<u32, String> {
        let pid = self.inner.add_panel(Arc::from(kind)).map_err(err_string)?;
        Ok(pid.raw())
    }

    pub fn remove_panel(&mut self, pid: u32) -> Result<(), String> {
        self.inner
            .remove_panel(PanelId::from_raw(pid))
            .map_err(err_string)?;
        Ok(())
    }

    pub fn focused(&self) -> Option<u32> {
        self.inner.focused().map(PanelId::raw)
    }

    pub fn focus_next(&mut self) {
        self.inner.focus_next();
    }

    pub fn focus_prev(&mut self) {
        self.inner.focus_prev();
    }

    pub fn set_panel_size(&mut self, pid: u32, width: f64, height: f64) -> Result<(), String> {
        let width = js_number_to_f32(width, "width")?;
        let height = js_number_to_f32(height, "height")?;
        self.inner
            .set_panel_size(PanelId::from_raw(pid), width, height)
            .map_err(err_string)
    }

    pub fn clear_panel_size(&mut self, pid: u32) -> Result<(), String> {
        self.inner
            .clear_panel_size(PanelId::from_raw(pid))
            .map_err(err_string)
    }

    pub fn scroll_offset(&self) -> f64 {
        f64::from(self.inner.viewport().scroll_offset)
    }

    pub fn set_scroll_offset(&mut self, offset: f64) -> Result<(), String> {
        let offset = js_number_to_f32(offset, "offset")?;
        self.inner.scroll_to(offset).map_err(err_string)
    }

    pub fn scroll_by(&mut self, delta: f64) -> Result<(), String> {
        let delta = js_number_to_f32(delta, "delta")?;
        self.inner.scroll_by(delta).map_err(err_string)
    }

    /// Serialize the last layout diff to JSON (convenience path).
    ///
    /// Returns `{"added":[...],"removed":[...],"moved":[...],"resized":[...],"unchanged":[...]}`.
    /// Panel IDs are u32, rects have f64 fields.
    pub fn layout_diff(&self) -> Result<String, String> {
        let diff = self.inner.last_diff();
        diff_to_json(
            diff.added,
            diff.removed,
            diff.moved,
            diff.resized,
            diff.unchanged,
            PanelId::raw,
            |c| rect_change(c.id.raw(), c.from, c.to),
        )
    }

    /// Serialize the last overlay diff to JSON (convenience path).
    ///
    /// Includes an `anchorFailed` array alongside the standard diff fields.
    pub fn overlay_diff(&self) -> Result<String, String> {
        let diff = self.inner.last_overlay_diff();
        let json = OverlayDiffJson {
            added: diff.added.iter().map(|id| id.raw()).collect(),
            removed: diff.removed.iter().map(|id| id.raw()).collect(),
            moved: diff
                .moved
                .iter()
                .map(|c| rect_change(c.id.raw(), c.from, c.to))
                .collect(),
            resized: diff
                .resized
                .iter()
                .map(|c| rect_change(c.id.raw(), c.from, c.to))
                .collect(),
            unchanged: diff.unchanged.iter().map(|id| id.raw()).collect(),
            anchor_failed: diff.anchor_failed.iter().map(|id| id.raw()).collect(),
        };
        serde_json::to_string(&json).map_err(|e| e.to_string())
    }
}

fn build_from_preset(preset: &str, kinds: &[&str]) -> Result<LayoutRuntime, String> {
    Layout::runtime_from_preset(preset, kinds)
}
