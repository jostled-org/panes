use serde::Serialize;

use crate::WasmRect;

#[derive(Serialize)]
pub(crate) struct PanelJson {
    pub id: u32,
    pub kind: Box<str>,
    pub rect: RectJson,
    #[serde(rename = "kindIndex")]
    pub kind_index: usize,
}

#[derive(Serialize)]
pub(crate) struct RectJson {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Serialize)]
pub(crate) struct RectChangeJson {
    pub id: u32,
    pub from: RectJson,
    pub to: RectJson,
}

#[derive(Serialize)]
pub(crate) struct DiffJson {
    pub added: Box<[u32]>,
    pub removed: Box<[u32]>,
    pub moved: Box<[RectChangeJson]>,
    pub resized: Box<[RectChangeJson]>,
    pub unchanged: Box<[u32]>,
}

impl From<WasmRect> for RectJson {
    fn from(r: WasmRect) -> Self {
        Self {
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
        }
    }
}

impl From<panes::Rect> for RectJson {
    fn from(r: panes::Rect) -> Self {
        Self {
            x: f64::from(r.x),
            y: f64::from(r.y),
            w: f64::from(r.w),
            h: f64::from(r.h),
        }
    }
}
