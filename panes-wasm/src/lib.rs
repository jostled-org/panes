//! Convert panes layouts into f64 rects for WASM/JavaScript consumption.

mod json_types;
mod layout;
mod runtime;

pub use layout::WasmLayout;
pub use runtime::WasmRuntime;

use panes::Rect;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "js", wasm_bindgen::prelude::wasm_bindgen)]
pub struct WasmRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
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
    fn offset_rect(self, r: Rect) -> Self {
        Self {
            x: f64::from(r.x) + self.x,
            y: f64::from(r.y) + self.y,
            w: f64::from(r.w),
            h: f64::from(r.h),
        }
    }
}

panes::impl_adapter! {
    rect: WasmRect,
    origin: WasmRect,
    convert_fn: |r: &Rect| WasmRect::from(*r),
    convert_at_fn: |r: &Rect, origin: WasmRect| origin.offset_rect(*r),
}
