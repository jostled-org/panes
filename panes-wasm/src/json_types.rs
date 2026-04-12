use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct PanelJson<'a> {
    pub id: u32,
    pub kind: &'a str,
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

#[derive(Serialize)]
pub(crate) struct OverlayDiffJson {
    pub added: Box<[u32]>,
    pub removed: Box<[u32]>,
    pub moved: Box<[RectChangeJson]>,
    pub resized: Box<[RectChangeJson]>,
    pub unchanged: Box<[u32]>,
    #[serde(rename = "anchorFailed")]
    pub anchor_failed: Box<[u32]>,
}

#[derive(Serialize)]
pub(crate) struct BoundaryJson {
    pub axis: &'static str,
    pub sides: [u32; 2],
    pub position: f64,
}

#[derive(Serialize)]
pub(crate) struct OverlayFailureJson<'a> {
    pub id: u32,
    pub kind: &'a str,
    pub reason: &'static str,
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
