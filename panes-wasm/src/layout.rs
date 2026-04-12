use std::cell::OnceCell;
use std::sync::Arc;

use panes::{BoundaryAxis, ResolvedLayout};
#[cfg(feature = "js")]
use wasm_bindgen::prelude::*;

use crate::json_types::{BoundaryJson, OverlayFailureJson, PanelJson, RectJson};

fn boundary_axis_code(axis: BoundaryAxis) -> f64 {
    match axis {
        BoundaryAxis::Vertical => 0.0,
        BoundaryAxis::Horizontal => 1.0,
    }
}

fn boundary_axis_str(axis: BoundaryAxis) -> &'static str {
    match axis {
        BoundaryAxis::Vertical => "vertical",
        BoundaryAxis::Horizontal => "horizontal",
    }
}

/// Resolved layout snapshot for JavaScript consumers.
///
/// Exposes two API tiers:
///
/// **Primary (fast-path):** [`panels_buf`](Self::panels_buf),
/// [`boundary_at_point_buf`](Self::boundary_at_point_buf), [`panel_at_point`](Self::panel_at_point),
/// [`overlay_at_point`](Self::overlay_at_point), [`panel_count`](Self::panel_count),
/// [`kind_table`](Self::kind_table).
/// These return typed data (flat f64 buffers, u32 IDs, or cached strings) with no per-call
/// serialization overhead. Use them in frame loops and hot paths.
///
/// **Convenience (JSON):** [`panels`](Self::panels),
/// [`boundary_at_point`](Self::boundary_at_point).
/// These serialize to JSON strings for debugging, logging, or one-shot queries.
/// They delegate to the same core `ResolvedLayout` queries as the fast paths.
#[cfg_attr(feature = "js", wasm_bindgen)]
pub struct WasmLayout {
    inner: Arc<ResolvedLayout>,
    buf: Vec<f64>,
    boundary_buf: Vec<f64>,
    cached_kind_table: OnceCell<Result<String, String>>,
    cached_panel_count: OnceCell<u32>,
}

impl WasmLayout {
    pub(crate) fn new(inner: Arc<ResolvedLayout>) -> Self {
        Self {
            inner,
            buf: Vec::new(),
            boundary_buf: Vec::new(),
            cached_kind_table: OnceCell::new(),
            cached_panel_count: OnceCell::new(),
        }
    }

    // ── Primary fast-path API ───────────────────────────────────────────

    /// Populate and return a flat f64 buffer with 6 values per panel.
    ///
    /// Layout: `[id, x, y, w, h, kindIndex, id, x, y, w, h, kindIndex, ...]`
    /// in kind-grouped order (see [`kind_table`](Self::kind_table)).
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

    /// Return the resize boundary closest to the point as a flat f64 buffer.
    ///
    /// Returns 4 values `[axis, side1_id, side2_id, position]` on hit,
    /// or an empty slice if no boundary is within tolerance.
    /// Axis encoding: `0.0` = vertical, `1.0` = horizontal.
    pub fn boundary_at_point_buf(
        &mut self,
        pointer_x: f64,
        pointer_y: f64,
        tolerance: f64,
    ) -> &[f64] {
        self.boundary_buf.clear();
        let hit = self.boundary_hit(pointer_x, pointer_y, tolerance);
        self.boundary_buf_from_hit(hit);
        &self.boundary_buf
    }

    /// Return the panel whose rect contains the given point, or `None`.
    ///
    /// Delegates to [`ResolvedLayout::panel_at_point`]; coordinates are
    /// f64 at the JS boundary, converted to f32 internally.
    pub fn panel_at_point(&self, pointer_x: f64, pointer_y: f64) -> Option<u32> {
        let (pointer_x, pointer_y) = query_point(pointer_x, pointer_y)?;
        self.inner
            .panel_at_point(pointer_x, pointer_y)
            .map(|pid| pid.raw())
    }

    /// Return the overlay whose rect contains the given point, or `None`.
    ///
    /// Delegates to [`ResolvedLayout::overlay_at_point`]; coordinates are
    /// f64 at the JS boundary, converted to f32 internally.
    pub fn overlay_at_point(&self, pointer_x: f64, pointer_y: f64) -> Option<u32> {
        let (pointer_x, pointer_y) = query_point(pointer_x, pointer_y)?;
        self.inner
            .overlay_at_point(pointer_x, pointer_y)
            .map(|oid| oid.raw())
    }

    /// Number of panels in the resolved layout.
    pub fn panel_count(&self) -> u32 {
        *self
            .cached_panel_count
            .get_or_init(|| self.inner.panel_count() as u32)
    }

    /// Return the kind strings as a JSON array in `kind_index` order.
    ///
    /// `kind_table()[kindIndex]` gives the kind string for any panel
    /// in the [`panels_buf`](Self::panels_buf) output. Cached after first call.
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

    // ── Convenience JSON API ────────────────────────────────────────────

    /// Serialize all panels to a JSON array string.
    ///
    /// Each entry has `id`, `kind`, `rect` (with f64 fields), and `kindIndex`.
    /// Convenience wrapper over the same `ResolvedLayout::panels()` iterator
    /// that [`panels_buf`](Self::panels_buf) uses — prefer the buffer path
    /// for performance-critical frame loops.
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

    /// Return the resize boundary closest to the point within tolerance.
    ///
    /// Returns a JSON string with `axis`, `sides`, and `position` fields,
    /// or `None` if no boundary is within tolerance.
    /// Convenience wrapper over the same `ResolvedLayout::boundary_at_point()`
    /// query that [`boundary_at_point_buf`](Self::boundary_at_point_buf) uses —
    /// prefer the buffer path for performance-critical frame loops.
    pub fn boundary_at_point(
        &self,
        pointer_x: f64,
        pointer_y: f64,
        tolerance: f64,
    ) -> Option<String> {
        let hit = self.boundary_hit(pointer_x, pointer_y, tolerance)?;
        let boundary = BoundaryJson {
            axis: boundary_axis_str(hit.axis),
            sides: [hit.sides.0.raw(), hit.sides.1.raw()],
            position: f64::from(hit.position),
        };
        serde_json::to_string(&boundary).ok()
    }

    // ── Overlay failures ────────────────────────────────────────────────

    /// Serialize overlay anchor failures to a JSON string.
    ///
    /// Returns `[{"id": <u32>, "kind": "<str>", "reason": "<str>"}, ...]`.
    pub fn overlay_failures(&self) -> Result<String, String> {
        let failures: Vec<OverlayFailureJson<'_>> = self
            .inner
            .overlay_failures()
            .iter()
            .map(|(id, kind, reason)| OverlayFailureJson {
                id: id.raw(),
                kind,
                reason: anchor_failure_str(*reason),
            })
            .collect();
        serde_json::to_string(&failures).map_err(|e| e.to_string())
    }

    // ── Internal helpers ────────────────────────────────────────────────

    fn boundary_hit(
        &self,
        pointer_x: f64,
        pointer_y: f64,
        tolerance: f64,
    ) -> Option<panes::BoundaryHit> {
        let (pointer_x, pointer_y) = query_point(pointer_x, pointer_y)?;
        let tolerance = js_number_to_f32(tolerance)?;
        self.inner
            .boundary_at_point(pointer_x, pointer_y, tolerance)
    }

    fn boundary_buf_from_hit(&mut self, hit: Option<panes::BoundaryHit>) {
        let Some(h) = hit else { return };
        let axis = boundary_axis_code(h.axis);
        self.boundary_buf.extend_from_slice(&[
            axis,
            f64::from(h.sides.0.raw()),
            f64::from(h.sides.1.raw()),
            f64::from(h.position),
        ]);
    }
}

impl From<Arc<ResolvedLayout>> for WasmLayout {
    fn from(inner: Arc<ResolvedLayout>) -> Self {
        Self::new(inner)
    }
}

fn anchor_failure_str(reason: panes::AnchorFailure) -> &'static str {
    match reason {
        panes::AnchorFailure::KindNotFound => "KindNotFound",
        panes::AnchorFailure::KindAmbiguous => "KindAmbiguous",
        panes::AnchorFailure::KeyStale => "KeyStale",
    }
}

fn js_number_to_f32(value: f64) -> Option<f32> {
    if !value.is_finite() {
        return None;
    }

    if value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        return None;
    }

    Some(value as f32)
}

fn query_point(pointer_x: f64, pointer_y: f64) -> Option<(f32, f32)> {
    Some((js_number_to_f32(pointer_x)?, js_number_to_f32(pointer_y)?))
}
