use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::node::PanelId;
use crate::overlay::{AnchorFailure, OverlayId};
use crate::rect::Rect;
use crate::resolver::ResolvedLayout;

const EPSILON: f32 = 1e-4;

/// A rect change for a single element between two frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectChange<Id> {
    /// The element that changed.
    pub id: Id,
    /// The rect in the previous frame.
    pub from: Rect,
    /// The rect in the current frame.
    pub to: Rect,
}

/// Panel rect change between frames.
pub type PanelRectChange = RectChange<PanelId>;

/// Overlay rect change between frames.
pub type OverlayRectChange = RectChange<OverlayId>;

/// Categorized differences between two frames.
///
/// Borrows its data from [`DiffScratch`] buffers owned by the runtime.
/// Valid until the next `resolve()` call.
#[derive(Debug)]
pub struct DiffResult<'a, Id> {
    /// Elements present in the new frame but not the old.
    pub added: &'a [Id],
    /// Elements present in the old frame but not the new.
    pub removed: &'a [Id],
    /// Elements whose position changed.
    pub moved: &'a [RectChange<Id>],
    /// Elements whose size changed.
    pub resized: &'a [RectChange<Id>],
    /// Elements whose rect is identical across frames.
    pub unchanged: &'a [Id],
}

/// Panel layout diff between frames.
pub type LayoutDiff<'a> = DiffResult<'a, PanelId>;

/// Overlay diff between frames with anchor failure tracking.
///
/// Unlike [`DiffResult`], this type carries an `anchor_failed` field for
/// overlays that resolved in the previous frame but whose anchor panel
/// is now missing or collapsed.
#[derive(Debug)]
pub struct OverlayDiff<'a> {
    /// Overlays present in the new frame but not the old.
    pub added: &'a [OverlayId],
    /// Overlays present in the old frame but not the new (hidden or removed).
    pub removed: &'a [OverlayId],
    /// Overlays whose position changed.
    pub moved: &'a [RectChange<OverlayId>],
    /// Overlays whose size changed.
    pub resized: &'a [RectChange<OverlayId>],
    /// Overlays whose rect is identical across frames.
    pub unchanged: &'a [OverlayId],
    /// Overlays that failed to anchor this frame but were resolved last frame.
    pub anchor_failed: &'a [OverlayId],
}

fn position_changed(a: &Rect, b: &Rect) -> bool {
    (a.x - b.x).abs() > EPSILON || (a.y - b.y).abs() > EPSILON
}

fn size_changed(a: &Rect, b: &Rect) -> bool {
    (a.w - b.w).abs() > EPSILON || (a.h - b.h).abs() > EPSILON
}

/// Classify a single element's rect change and push to the appropriate output Vec.
fn classify<Id: Copy>(
    id: Id,
    old_rect: &Rect,
    new_rect: &Rect,
    moved: &mut Vec<RectChange<Id>>,
    resized: &mut Vec<RectChange<Id>>,
    unchanged: &mut Vec<Id>,
) {
    let pos = position_changed(old_rect, new_rect);
    let size = size_changed(old_rect, new_rect);

    match (pos, size) {
        (false, false) => unchanged.push(id),
        (true, true) => {
            let change = RectChange {
                id,
                from: *old_rect,
                to: *new_rect,
            };
            moved.push(change);
            resized.push(change);
        }
        (true, false) => moved.push(RectChange {
            id,
            from: *old_rect,
            to: *new_rect,
        }),
        (false, true) => resized.push(RectChange {
            id,
            from: *old_rect,
            to: *new_rect,
        }),
    }
}

/// Reusable scratch buffers for frame-to-frame diffing.
///
/// Retains heap capacity across frames so repeated `resolve()` calls
/// avoid allocation. Call [`as_diff`](Self::as_diff) to borrow the
/// categorized results.
pub struct DiffScratch<Id> {
    pub(crate) added: Vec<Id>,
    pub(crate) removed: Vec<Id>,
    pub(crate) moved: Vec<RectChange<Id>>,
    pub(crate) resized: Vec<RectChange<Id>>,
    pub(crate) unchanged: Vec<Id>,
}

impl<Id> Default for DiffScratch<Id> {
    fn default() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            moved: Vec::new(),
            resized: Vec::new(),
            unchanged: Vec::new(),
        }
    }
}

impl<Id> DiffScratch<Id> {
    /// Clear all output buffers, retaining allocated capacity.
    fn clear(&mut self) {
        self.added.clear();
        self.removed.clear();
        self.moved.clear();
        self.resized.clear();
        self.unchanged.clear();
    }

    /// Borrow the diff result from this scratch buffer.
    pub fn as_diff(&self) -> DiffResult<'_, Id> {
        DiffResult {
            added: &self.added,
            removed: &self.removed,
            moved: &self.moved,
            resized: &self.resized,
            unchanged: &self.unchanged,
        }
    }
}

/// Panel diff scratch buffers.
pub type PanelDiffScratch = DiffScratch<PanelId>;

/// Overlay diff scratch buffers with anchor failure tracking.
///
/// Extends [`DiffScratch`] with hash-set bookkeeping for overlay identity
/// and an `anchor_failed` buffer for overlays whose anchor disappeared.
#[derive(Default)]
pub struct OverlayDiffScratch {
    pub(crate) inner: DiffScratch<OverlayId>,
    pub(crate) anchor_failed: Vec<OverlayId>,
    curr_rect_indices: FxHashMap<OverlayId, usize>,
    curr_failed_ids: FxHashSet<OverlayId>,
    prev_rect_ids: FxHashSet<OverlayId>,
}

impl OverlayDiffScratch {
    fn clear(&mut self) {
        self.inner.clear();
        self.anchor_failed.clear();
        self.curr_rect_indices.clear();
        self.curr_failed_ids.clear();
        self.prev_rect_ids.clear();
    }

    /// Borrow the overlay diff result from this scratch buffer.
    pub fn as_diff(&self) -> OverlayDiff<'_> {
        OverlayDiff {
            added: &self.inner.added,
            removed: &self.inner.removed,
            moved: &self.inner.moved,
            resized: &self.inner.resized,
            unchanged: &self.inner.unchanged,
            anchor_failed: &self.anchor_failed,
        }
    }
}

/// Panel diff scratch with hash-set bookkeeping for add/remove detection.
///
/// Tracks the old and new panel ID sets so added/removed panels can be
/// computed via set difference. Call [`as_diff`](Self::as_diff) to borrow
/// the categorized results.
#[derive(Default)]
pub struct PanelScratch {
    pub(crate) old_ids: FxHashSet<PanelId>,
    pub(crate) new_ids: FxHashSet<PanelId>,
    pub(crate) inner: PanelDiffScratch,
}

impl PanelScratch {
    /// Borrow the diff result from this scratch buffer.
    pub fn as_diff(&self) -> LayoutDiff<'_> {
        self.inner.as_diff()
    }
}

/// Compare two resolved layouts and categorize every panel.
///
/// Standalone version that allocates its own scratch. For per-frame use,
/// prefer the runtime's `last_diff()` which reuses buffers.
pub fn diff(old: &ResolvedLayout, new: &ResolvedLayout) -> PanelScratch {
    let mut scratch = PanelScratch::default();
    diff_reuse(old, new, &mut scratch);
    scratch
}

/// Compare two resolved layouts, reusing scratch buffers across frames.
pub(crate) fn diff_reuse<'a>(
    old: &ResolvedLayout,
    new: &ResolvedLayout,
    scratch: &'a mut PanelScratch,
) -> LayoutDiff<'a> {
    scratch.old_ids.clear();
    scratch.old_ids.extend(old.panel_ids());
    scratch.new_ids.clear();
    scratch.new_ids.extend(new.panel_ids());

    scratch.inner.clear();
    scratch
        .inner
        .removed
        .extend(scratch.old_ids.difference(&scratch.new_ids).copied());
    scratch
        .inner
        .added
        .extend(scratch.new_ids.difference(&scratch.old_ids).copied());

    for &pid in scratch.old_ids.intersection(&scratch.new_ids) {
        let (Some(old_rect), Some(new_rect)) = (old.get(pid), new.get(pid)) else {
            continue;
        };
        classify(
            pid,
            old_rect,
            new_rect,
            &mut scratch.inner.moved,
            &mut scratch.inner.resized,
            &mut scratch.inner.unchanged,
        );
    }

    scratch.inner.as_diff()
}

/// Diff when the panel set is identical, reusing scratch Vecs across frames.
pub(crate) fn diff_same_panels_reuse<'a>(
    old: &ResolvedLayout,
    new: &ResolvedLayout,
    scratch: &'a mut PanelScratch,
) -> LayoutDiff<'a> {
    scratch.inner.clear();

    for (pid, new_rect) in new.iter() {
        let Some(old_rect) = old.get(pid) else {
            continue;
        };
        classify(
            pid,
            old_rect,
            new_rect,
            &mut scratch.inner.moved,
            &mut scratch.inner.resized,
            &mut scratch.inner.unchanged,
        );
    }

    scratch.inner.as_diff()
}

/// Produce a diff representing the first frame — all panels are added.
pub(crate) fn first_frame<'a>(
    layout: &ResolvedLayout,
    scratch: &'a mut PanelScratch,
) -> LayoutDiff<'a> {
    scratch.inner.clear();
    scratch.inner.added.extend(layout.panel_ids());
    scratch.inner.as_diff()
}

/// Diff overlay rects between frames, reusing scratch buffers.
///
/// Overlays that were resolved last frame but now fail to anchor appear in `anchor_failed`.
/// Overlays that were hidden or removed appear in `removed`.
pub(crate) fn diff_overlays<'a>(
    prev_rects: &[(OverlayId, Arc<str>, Rect)],
    curr_rects: &[(OverlayId, Arc<str>, Rect)],
    curr_failures: &[(OverlayId, Arc<str>, AnchorFailure)],
    scratch: &'a mut OverlayDiffScratch,
) -> OverlayDiff<'a> {
    scratch.clear();
    scratch.curr_rect_indices.extend(
        curr_rects
            .iter()
            .enumerate()
            .map(|(index, (id, _, _))| (*id, index)),
    );
    scratch
        .curr_failed_ids
        .extend(curr_failures.iter().map(|(id, _, _)| *id));
    scratch
        .prev_rect_ids
        .extend(prev_rects.iter().map(|(id, _, _)| *id));

    // Find removed/failed and common among previously-resolved overlays
    for (old_id, _, old_rect) in prev_rects {
        let in_curr = scratch
            .curr_rect_indices
            .get(old_id)
            .map(|index| &curr_rects[*index].2);
        let now_failed = scratch.curr_failed_ids.contains(old_id);
        match (in_curr, now_failed) {
            (Some(new_rect), _) => classify(
                *old_id,
                old_rect,
                new_rect,
                &mut scratch.inner.moved,
                &mut scratch.inner.resized,
                &mut scratch.inner.unchanged,
            ),
            (None, true) => scratch.anchor_failed.push(*old_id),
            (None, false) => scratch.inner.removed.push(*old_id),
        }
    }

    // Find newly added (in curr_rects but not in prev_rects)
    for (new_id, _, _) in curr_rects {
        if !scratch.prev_rect_ids.contains(new_id) {
            scratch.inner.added.push(*new_id);
        }
    }

    // Overlays that failed in both frames are not reported in any diff category.
    // Overlays that were failed before but now resolved appear as added (handled above).

    scratch.as_diff()
}

/// Produce an overlay diff for the first frame — all resolved overlays are added,
/// all failed overlays are in anchor_failed.
pub(crate) fn first_frame_overlays<'a>(
    rects: &[(OverlayId, Arc<str>, Rect)],
    failures: &[(OverlayId, Arc<str>, AnchorFailure)],
    scratch: &'a mut OverlayDiffScratch,
) -> OverlayDiff<'a> {
    scratch.clear();
    scratch
        .inner
        .added
        .extend(rects.iter().map(|(id, _, _)| *id));
    scratch
        .anchor_failed
        .extend(failures.iter().map(|(id, _, _)| *id));
    scratch.as_diff()
}
