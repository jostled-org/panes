use std::sync::Arc;

use panes::{BoundaryAxis, ResolvedLayout};
#[cfg(feature = "js")]
use wasm_bindgen::prelude::*;

use crate::WasmRect;
use crate::json_types::{PanelJson, RectJson};

/// Resolved layout snapshot for JavaScript consumers.
///
/// Wraps `Arc<ResolvedLayout>` and exposes panels as JSON with f64 rects.
#[cfg_attr(feature = "js", wasm_bindgen)]
pub struct WasmLayout {
    inner: Arc<ResolvedLayout>,
}

impl WasmLayout {
    pub(crate) fn new(inner: Arc<ResolvedLayout>) -> Self {
        Self { inner }
    }

    /// Serialize all panels to a JSON array string.
    ///
    /// Each entry has `id`, `kind`, `rect` (with f64 fields), and `kindIndex`.
    pub fn panels(&self) -> Result<String, String> {
        let entries: Vec<PanelJson> = self
            .inner
            .panels()
            .map(|e| {
                let wr = WasmRect::from(*e.rect);
                PanelJson {
                    id: e.id.raw(),
                    kind: Box::from(e.kind),
                    rect: RectJson::from(wr),
                    kind_index: e.kind_index,
                }
            })
            .collect();
        serde_json::to_string(&entries).map_err(|e| e.to_string())
    }

    /// Return the panel whose rect contains the given point, or `None`.
    ///
    /// Coordinates are f64 at the JS boundary, converted to f32 internally.
    pub fn panel_at_point(&self, pointer_x: f64, pointer_y: f64) -> Option<u32> {
        self.inner
            .panel_at_point(pointer_x as f32, pointer_y as f32)
            .map(|pid| pid.raw())
    }

    /// Return the overlay whose rect contains the given point, or `None`.
    ///
    /// Coordinates are f64 at the JS boundary, converted to f32 internally.
    pub fn overlay_at_point(&self, pointer_x: f64, pointer_y: f64) -> Option<u32> {
        self.inner
            .overlay_at_point(pointer_x as f32, pointer_y as f32)
            .map(|oid| oid.raw())
    }

    /// Return the resize boundary closest to the point within tolerance.
    ///
    /// Returns a JSON string with `axis`, `sides`, and `position` fields,
    /// or `None` if no boundary is within tolerance.
    pub fn boundary_at_point(
        &self,
        pointer_x: f64,
        pointer_y: f64,
        tolerance: f64,
    ) -> Option<String> {
        let hit =
            self.inner
                .boundary_at_point(pointer_x as f32, pointer_y as f32, tolerance as f32)?;
        let axis = match hit.axis {
            BoundaryAxis::Vertical => "vertical",
            BoundaryAxis::Horizontal => "horizontal",
        };
        // Inline serialization — BoundaryJson is structurally disconnected
        // from the panel/diff types and used only here.
        use std::fmt::Write;
        let mut buf = String::with_capacity(80);
        let _ = write!(
            buf,
            r#"{{"axis":"{}","sides":[{},{}],"position":{}}}"#,
            axis,
            hit.sides.0.raw(),
            hit.sides.1.raw(),
            f64::from(hit.position),
        );
        Some(buf)
    }
}
