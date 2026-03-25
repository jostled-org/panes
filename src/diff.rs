use std::sync::Arc;

use rustc_hash::FxHashSet;

use crate::node::PanelId;
use crate::overlay::OverlayId;
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

/// Overlay diff between frames.
pub type OverlayDiff<'a> = DiffResult<'a, OverlayId>;

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

/// Reusable scratch buffers for diffing without per-frame allocation.
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

/// Overlay diff scratch buffers.
pub type OverlayDiffScratch = DiffScratch<OverlayId>;

/// Panel scratch with hash-set bookkeeping for add/remove detection.
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
pub(crate) fn diff_overlays<'a>(
    prev: &[(OverlayId, Rect)],
    curr: &[(OverlayId, Arc<str>, Rect)],
    scratch: &'a mut OverlayDiffScratch,
) -> OverlayDiff<'a> {
    scratch.clear();

    // Find removed and common
    for (old_id, old_rect) in prev {
        let found = curr.iter().find(|(id, _, _)| id == old_id);
        match found {
            None => scratch.removed.push(*old_id),
            Some((_, _, new_rect)) => classify(
                *old_id,
                old_rect,
                new_rect,
                &mut scratch.moved,
                &mut scratch.resized,
                &mut scratch.unchanged,
            ),
        }
    }

    // Find added
    for (new_id, _, _) in curr {
        let in_prev = prev.iter().any(|(id, _)| id == new_id);
        if !in_prev {
            scratch.added.push(*new_id);
        }
    }

    scratch.as_diff()
}

/// Produce an overlay diff for the first frame — all overlays are added.
pub(crate) fn first_frame_overlays<'a>(
    rects: &[(OverlayId, Arc<str>, Rect)],
    scratch: &'a mut OverlayDiffScratch,
) -> OverlayDiff<'a> {
    scratch.clear();
    scratch.added.extend(rects.iter().map(|(id, _, _)| *id));
    scratch.as_diff()
}
