use std::sync::Arc;

use rustc_hash::FxHashSet;

use crate::node::PanelId;
use crate::overlay::OverlayId;
use crate::rect::Rect;
use crate::resolver::ResolvedLayout;

const EPSILON: f32 = 1e-4;

/// A panel whose rect changed between two frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectChange {
    /// The panel that changed.
    pub id: PanelId,
    /// The rect in the previous frame.
    pub from: Rect,
    /// The rect in the current frame.
    pub to: Rect,
}

/// Categorized differences between two resolved layouts.
///
/// Borrows its data from [`DiffScratch`] buffers owned by the runtime.
/// Valid until the next `resolve()` call.
#[derive(Debug)]
pub struct LayoutDiff<'a> {
    /// Panels present in the new frame but not the old.
    pub added: &'a [PanelId],
    /// Panels present in the old frame but not the new.
    pub removed: &'a [PanelId],
    /// Panels whose position changed.
    pub moved: &'a [RectChange],
    /// Panels whose size changed.
    pub resized: &'a [RectChange],
    /// Panels whose rect is identical across frames.
    pub unchanged: &'a [PanelId],
}

fn position_changed(a: &Rect, b: &Rect) -> bool {
    (a.x - b.x).abs() > EPSILON || (a.y - b.y).abs() > EPSILON
}

fn size_changed(a: &Rect, b: &Rect) -> bool {
    (a.w - b.w).abs() > EPSILON || (a.h - b.h).abs() > EPSILON
}

/// Classify a single element's rect change and push to the appropriate output Vec.
fn classify<Id: Copy, C>(
    id: Id,
    old_rect: &Rect,
    new_rect: &Rect,
    make_change: impl FnOnce(Id, Rect, Rect) -> C,
    moved: &mut Vec<C>,
    resized: &mut Vec<C>,
    unchanged: &mut Vec<Id>,
) where
    C: Copy,
{
    let pos = position_changed(old_rect, new_rect);
    let size = size_changed(old_rect, new_rect);

    match (pos, size) {
        (false, false) => unchanged.push(id),
        (true, true) => {
            let change = make_change(id, *old_rect, *new_rect);
            moved.push(change);
            resized.push(change);
        }
        (true, false) => moved.push(make_change(id, *old_rect, *new_rect)),
        (false, true) => resized.push(make_change(id, *old_rect, *new_rect)),
    }
}

/// Compare two resolved layouts and categorize every panel.
///
/// Standalone version that allocates its own scratch. For per-frame use,
/// prefer the runtime's `last_diff()` which reuses buffers.
pub fn diff(old: &ResolvedLayout, new: &ResolvedLayout) -> DiffScratch {
    let mut scratch = DiffScratch::default();
    diff_reuse(old, new, &mut scratch);
    scratch
}

/// Reusable scratch buffers for diffing without per-frame allocation.
#[derive(Default)]
pub struct DiffScratch {
    pub(crate) old_ids: FxHashSet<PanelId>,
    pub(crate) new_ids: FxHashSet<PanelId>,
    pub(crate) added: Vec<PanelId>,
    pub(crate) removed: Vec<PanelId>,
    pub(crate) moved: Vec<RectChange>,
    pub(crate) resized: Vec<RectChange>,
    pub(crate) unchanged: Vec<PanelId>,
}

impl DiffScratch {
    /// Borrow the diff result from this scratch buffer.
    pub fn as_diff(&self) -> LayoutDiff<'_> {
        LayoutDiff {
            added: &self.added,
            removed: &self.removed,
            moved: &self.moved,
            resized: &self.resized,
            unchanged: &self.unchanged,
        }
    }
}

/// Compare two resolved layouts, reusing scratch buffers across frames.
pub(crate) fn diff_reuse<'a>(
    old: &ResolvedLayout,
    new: &ResolvedLayout,
    scratch: &'a mut DiffScratch,
) -> LayoutDiff<'a> {
    scratch.old_ids.clear();
    scratch.old_ids.extend(old.panel_ids());
    scratch.new_ids.clear();
    scratch.new_ids.extend(new.panel_ids());

    scratch.removed.clear();
    scratch
        .removed
        .extend(scratch.old_ids.difference(&scratch.new_ids).copied());
    scratch.added.clear();
    scratch
        .added
        .extend(scratch.new_ids.difference(&scratch.old_ids).copied());

    scratch.moved.clear();
    scratch.resized.clear();
    scratch.unchanged.clear();

    for &pid in scratch.old_ids.intersection(&scratch.new_ids) {
        let (Some(old_rect), Some(new_rect)) = (old.get(pid), new.get(pid)) else {
            continue;
        };
        classify(
            pid,
            old_rect,
            new_rect,
            |id, from, to| RectChange { id, from, to },
            &mut scratch.moved,
            &mut scratch.resized,
            &mut scratch.unchanged,
        );
    }

    scratch.as_diff()
}

/// Diff when the panel set is identical, reusing scratch Vecs across frames.
pub(crate) fn diff_same_panels_reuse<'a>(
    old: &ResolvedLayout,
    new: &ResolvedLayout,
    scratch: &'a mut DiffScratch,
) -> LayoutDiff<'a> {
    scratch.added.clear();
    scratch.removed.clear();
    scratch.moved.clear();
    scratch.resized.clear();
    scratch.unchanged.clear();

    for (pid, new_rect) in new.iter() {
        let Some(old_rect) = old.get(pid) else {
            continue;
        };
        classify(
            pid,
            old_rect,
            new_rect,
            |id, from, to| RectChange { id, from, to },
            &mut scratch.moved,
            &mut scratch.resized,
            &mut scratch.unchanged,
        );
    }

    scratch.as_diff()
}

/// Produce a diff representing the first frame — all panels are added.
pub(crate) fn first_frame<'a>(
    layout: &ResolvedLayout,
    scratch: &'a mut DiffScratch,
) -> LayoutDiff<'a> {
    scratch.added.clear();
    scratch.added.extend(layout.panel_ids());
    scratch.removed.clear();
    scratch.moved.clear();
    scratch.resized.clear();
    scratch.unchanged.clear();
    scratch.as_diff()
}

/// A change in an overlay's rect between frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayRectChange {
    /// The overlay that changed.
    pub id: OverlayId,
    /// Rect in the previous frame.
    pub from: Rect,
    /// Rect in the current frame.
    pub to: Rect,
}

/// Categorized differences between overlay sets across two frames.
///
/// Borrows its data from internal scratch buffers owned by the runtime.
/// Valid until the next `resolve()` call.
#[derive(Debug)]
pub struct OverlayDiff<'a> {
    /// Overlays present in the new frame but not the old.
    pub added: &'a [OverlayId],
    /// Overlays present in the old frame but not the new.
    pub removed: &'a [OverlayId],
    /// Overlays whose position changed.
    pub moved: &'a [OverlayRectChange],
    /// Overlays whose size changed.
    pub resized: &'a [OverlayRectChange],
    /// Overlays whose rect is identical across frames.
    pub unchanged: &'a [OverlayId],
}

/// Reusable scratch buffers for overlay diffing.
#[derive(Default)]
pub(crate) struct OverlayDiffScratch {
    added: Vec<OverlayId>,
    removed: Vec<OverlayId>,
    moved: Vec<OverlayRectChange>,
    resized: Vec<OverlayRectChange>,
    unchanged: Vec<OverlayId>,
}

impl OverlayDiffScratch {
    pub(crate) fn as_diff(&self) -> OverlayDiff<'_> {
        OverlayDiff {
            added: &self.added,
            removed: &self.removed,
            moved: &self.moved,
            resized: &self.resized,
            unchanged: &self.unchanged,
        }
    }
}

/// Diff overlay rects between frames, reusing scratch buffers.
pub(crate) fn diff_overlays<'a>(
    prev: &[(OverlayId, Rect)],
    curr: &[(OverlayId, Arc<str>, Rect)],
    scratch: &'a mut OverlayDiffScratch,
) -> OverlayDiff<'a> {
    scratch.added.clear();
    scratch.removed.clear();
    scratch.moved.clear();
    scratch.resized.clear();
    scratch.unchanged.clear();

    // Find removed and common
    for (old_id, old_rect) in prev {
        let found = curr.iter().find(|(id, _, _)| id == old_id);
        match found {
            None => scratch.removed.push(*old_id),
            Some((_, _, new_rect)) => classify(
                *old_id,
                old_rect,
                new_rect,
                |id, from, to| OverlayRectChange { id, from, to },
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
    scratch.added.clear();
    scratch.added.extend(rects.iter().map(|(id, _, _)| *id));
    scratch.removed.clear();
    scratch.moved.clear();
    scratch.resized.clear();
    scratch.unchanged.clear();
    scratch.as_diff()
}
