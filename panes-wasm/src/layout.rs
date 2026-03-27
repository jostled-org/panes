use std::cell::OnceCell;
use std::sync::Arc;

use panes::{BoundaryAxis, ResolvedLayout};
#[cfg(feature = "js")]
use wasm_bindgen::prelude::*;

use crate::json_types::{PanelJson, RectJson};

/// Resolved layout snapshot for JavaScript consumers.
///
/// Wraps `Arc<ResolvedLayout>` and exposes panels as JSON with f64 rects.
#[cfg_attr(feature = "js", wasm_bindgen)]
pub struct WasmLayout {
    inner: Arc<ResolvedLayout>,
    buf: Vec<f64>,
    cached_kind_table: OnceCell<Result<String, String>>,
    cached_panel_count: OnceCell<u32>,
}

impl WasmLayout {
    pub(crate) fn new(inner: Arc<ResolvedLayout>) -> Self {
        Self {
            inner,
            buf: Vec::new(),
            cached_kind_table: OnceCell::new(),
            cached_panel_count: OnceCell::new(),
        }
    }

    /// Serialize all panels to a JSON array string.
    ///
    /// Each entry has `id`, `kind`, `rect` (with f64 fields), and `kindIndex`.
    pub fn panels(&self) -> Result<String, String> {
        let entries: Vec<PanelJson<'_>> = self
            .inner
            .panels()
            .map(|e| PanelJson {
                id: e.id.raw(),
                kind: e.kind,
                rect: RectJson::from(*e.rect),
                kind_index: e.kind_index,
            })
            .collect();
        serde_json::to_string(&entries).map_err(|e| e.to_string())
    }

    /// Populate and return a flat f64 buffer with 6 values per panel.
    ///
    /// Layout: `[id, x, y, w, h, kindIndex, id, x, y, w, h, kindIndex, ...]`
    /// in the same kind-grouped order as [`panels()`](Self::panels).
    /// The buffer is reused across calls — no allocation after first use
    /// if panel count stays the same or shrinks.
    pub fn panels_buf(&mut self) -> &[f64] {
        self.buf.clear();
        let mut count: u32 = 0;
        for entry in self.inner.panels() {
            self.buf.extend_from_slice(&[
                f64::from(entry.id.raw()),
                f64::from(entry.rect.x),
                f64::from(entry.rect.y),
                f64::from(entry.rect.w),
                f64::from(entry.rect.h),
                f64::from(entry.kind_index as u32),
            ]);
            count += 1;
        }
        let _ = self.cached_panel_count.set(count);
        &self.buf
    }

    /// Return the kind strings as a JSON array in `kind_index` order.
    ///
    /// `kind_table()[kindIndex]` gives the kind string for any panel
    /// in the [`panels_buf()`](Self::panels_buf) output.
    pub fn kind_table(&self) -> Result<String, String> {
        self.cached_kind_table
            .get_or_init(|| {
                let keys: Vec<&str> = self
                    .inner
                    .sorted_kind_keys()
                    .iter()
                    .map(AsRef::as_ref)
                    .collect();
                serde_json::to_string(&keys).map_err(|e| e.to_string())
            })
            .clone()
    }

    /// Number of panels in the resolved layout.
    pub fn panel_count(&self) -> u32 {
        *self
            .cached_panel_count
            .get_or_init(|| self.inner.panels().count() as u32)
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
