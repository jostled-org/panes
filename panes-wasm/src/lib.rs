//! Convert panes layouts into f64 rects for WASM/JavaScript consumption.

mod json_types;
mod layout;
mod runtime;

pub use layout::WasmLayout;
pub use runtime::WasmRuntime;

use panes::{OverlayEntry, PanelEntry, PanelId, Rect, ResolvedLayout};
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

impl From<Rect> for WasmRect {
    fn from(r: Rect) -> Self {
        Self {
            x: f64::from(r.x),
            y: f64::from(r.y),
            w: f64::from(r.w),
            h: f64::from(r.h),
        }
    }
}

impl WasmRect {
    /// Convert a layout rect, offsetting by this rect's origin.
    fn offset_rect(self, r: Rect) -> Self {
        Self {
            x: f64::from(r.x) + self.x,
            y: f64::from(r.y) + self.y,
            w: f64::from(r.w),
            h: f64::from(r.h),
        }
    }
}

/// Convert a resolved panes layout into f64 wasm rects.
///
/// Casts each f32 field to f64 for JavaScript consumption.
pub fn convert(resolved: &ResolvedLayout) -> FxHashMap<PanelId, WasmRect> {
    resolved
        .iter()
        .map(|(pid, r)| (pid, WasmRect::from(*r)))
        .collect()
}

/// Iterate all panels in kind-grouped order, yielding identity and f64 rect.
///
/// No hashmap allocation — produces entries lazily from the resolved layout.
pub fn panels(resolved: &ResolvedLayout) -> impl Iterator<Item = PanelEntry<'_, WasmRect>> {
    resolved
        .panels()
        .map(|e| e.map_rect(|r| WasmRect::from(*r)))
}

/// Iterate all panels with f64 rects offset by an origin position.
///
/// Suitable for rendering a panes layout inside a container at an offset.
pub fn panels_at(
    resolved: &ResolvedLayout,
    origin: WasmRect,
) -> impl Iterator<Item = PanelEntry<'_, WasmRect>> {
    resolved
        .panels()
        .map(move |e| e.map_rect(|r| origin.offset_rect(*r)))
}

/// Iterate all resolved overlays, yielding identity and f64 rect.
pub fn overlays(resolved: &ResolvedLayout) -> impl Iterator<Item = OverlayEntry<'_, WasmRect>> {
    resolved
        .overlays()
        .map(|e| e.map_rect(|r| WasmRect::from(*r)))
}

/// Iterate all resolved overlays with f64 rects offset by an origin position.
///
/// Suitable for rendering overlays inside a container at an offset.
pub fn overlays_at(
    resolved: &ResolvedLayout,
    origin: WasmRect,
) -> impl Iterator<Item = OverlayEntry<'_, WasmRect>> {
    resolved
        .overlays()
        .map(move |e| e.map_rect(|r| origin.offset_rect(*r)))
}
