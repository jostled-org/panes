// panes-wasm — WASM/canvas adapter for panes layout engine

use panes::{PanelId, ResolvedLayout};
use rustc_hash::FxHashMap;

/// Rectangle with f64 fields for JavaScript interop.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "js", wasm_bindgen::prelude::wasm_bindgen)]
pub struct WasmRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Convert a resolved panes layout into f64 wasm rects.
///
/// Casts each f32 field to f64 for JavaScript consumption.
pub fn convert(resolved: &ResolvedLayout) -> FxHashMap<PanelId, WasmRect> {
    resolved
        .iter()
        .map(|(pid, r)| {
            let rect = WasmRect {
                x: f64::from(r.x),
                y: f64::from(r.y),
                w: f64::from(r.w),
                h: f64::from(r.h),
            };
            (pid, rect)
        })
        .collect()
}
