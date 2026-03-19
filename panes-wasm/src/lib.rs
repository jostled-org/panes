//! Convert panes layouts into f64 rects for WASM/JavaScript consumption.

use panes::{OverlayEntry, PanelEntry, PanelId, ResolvedLayout};
use rustc_hash::FxHashMap;

/// Rectangle with f64 fields for JavaScript interop.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "js", wasm_bindgen::prelude::wasm_bindgen)]
pub struct WasmRect {
    /// Horizontal origin.
    pub x: f64,
    /// Vertical origin.
    pub y: f64,
    /// Width.
    pub w: f64,
    /// Height.
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

/// Iterate all panels in kind-grouped order, yielding identity and f64 rect.
///
/// No hashmap allocation — produces entries lazily from the resolved layout.
pub fn panels(resolved: &ResolvedLayout) -> impl Iterator<Item = PanelEntry<'_, WasmRect>> {
    resolved.panels().map(|e| {
        e.map_rect(|r| WasmRect {
            x: f64::from(r.x),
            y: f64::from(r.y),
            w: f64::from(r.w),
            h: f64::from(r.h),
        })
    })
}

/// Iterate all panels with f64 rects offset by an origin position.
///
/// Suitable for rendering a panes layout inside a container at an offset.
pub fn panels_at(
    resolved: &ResolvedLayout,
    origin: WasmRect,
) -> impl Iterator<Item = PanelEntry<'_, WasmRect>> {
    resolved.panels().map(move |e| {
        e.map_rect(|r| WasmRect {
            x: f64::from(r.x) + origin.x,
            y: f64::from(r.y) + origin.y,
            w: f64::from(r.w),
            h: f64::from(r.h),
        })
    })
}

/// Iterate all resolved overlays, yielding identity and f64 rect.
pub fn overlays(resolved: &ResolvedLayout) -> impl Iterator<Item = OverlayEntry<'_, WasmRect>> {
    resolved.overlays().map(|e| {
        e.map_rect(|r| WasmRect {
            x: f64::from(r.x),
            y: f64::from(r.y),
            w: f64::from(r.w),
            h: f64::from(r.h),
        })
    })
}

/// Iterate all resolved overlays with f64 rects offset by an origin position.
///
/// Suitable for rendering overlays inside a container at an offset.
pub fn overlays_at(
    resolved: &ResolvedLayout,
    origin: WasmRect,
) -> impl Iterator<Item = OverlayEntry<'_, WasmRect>> {
    resolved.overlays().map(move |e| {
        e.map_rect(|r| WasmRect {
            x: f64::from(r.x) + origin.x,
            y: f64::from(r.y) + origin.y,
            w: f64::from(r.w),
            h: f64::from(r.h),
        })
    })
}
