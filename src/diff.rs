use rustc_hash::FxHashSet;

use crate::node::PanelId;
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
#[derive(Debug)]
pub struct LayoutDiff {
    /// Panels present in the new frame but not the old.
    pub added: Box<[PanelId]>,
    /// Panels present in the old frame but not the new.
    pub removed: Box<[PanelId]>,
    /// Panels whose position changed.
    pub moved: Box<[RectChange]>,
    /// Panels whose size changed.
    pub resized: Box<[RectChange]>,
    /// Panels whose rect is identical across frames.
    pub unchanged: Box<[PanelId]>,
}

fn position_changed(a: &Rect, b: &Rect) -> bool {
    (a.x - b.x).abs() > EPSILON || (a.y - b.y).abs() > EPSILON
}

fn size_changed(a: &Rect, b: &Rect) -> bool {
    (a.w - b.w).abs() > EPSILON || (a.h - b.h).abs() > EPSILON
}

/// Classify a single panel's rect change and push to the appropriate output Vec.
fn classify_change(
    pid: PanelId,
    old_rect: &Rect,
    new_rect: &Rect,
    moved: &mut Vec<RectChange>,
    resized: &mut Vec<RectChange>,
    unchanged: &mut Vec<PanelId>,
) {
    let pos = position_changed(old_rect, new_rect);
    let size = size_changed(old_rect, new_rect);

    let change = RectChange {
        id: pid,
        from: *old_rect,
        to: *new_rect,
    };

    match (pos, size) {
        (false, false) => unchanged.push(pid),
        (true, true) => {
            moved.push(change);
            resized.push(change);
        }
        (true, false) => moved.push(change),
        (false, true) => resized.push(change),
    }
}

/// Shared diff logic: given pre-computed id sets, classify all panels.
fn diff_from_sets(
    old_ids: &FxHashSet<PanelId>,
    new_ids: &FxHashSet<PanelId>,
    old: &ResolvedLayout,
    new: &ResolvedLayout,
) -> LayoutDiff {
    let removed: Vec<PanelId> = old_ids.difference(new_ids).copied().collect();
    let added: Vec<PanelId> = new_ids.difference(old_ids).copied().collect();

    let common_count = old_ids.len().min(new_ids.len());
    let mut moved = Vec::with_capacity(common_count);
    let mut resized = Vec::with_capacity(common_count);
    let mut unchanged = Vec::with_capacity(common_count);

    for &pid in old_ids.intersection(new_ids) {
        let (Some(old_rect), Some(new_rect)) = (old.get(pid), new.get(pid)) else {
            continue;
        };
        classify_change(
            pid,
            old_rect,
            new_rect,
            &mut moved,
            &mut resized,
            &mut unchanged,
        );
    }

    LayoutDiff {
        added: added.into_boxed_slice(),
        removed: removed.into_boxed_slice(),
        moved: moved.into_boxed_slice(),
        resized: resized.into_boxed_slice(),
        unchanged: unchanged.into_boxed_slice(),
    }
}

/// Compare two resolved layouts and categorize every panel.
pub fn diff(old: &ResolvedLayout, new: &ResolvedLayout) -> LayoutDiff {
    let old_ids: FxHashSet<PanelId> = old.panel_ids().collect();
    let new_ids: FxHashSet<PanelId> = new.panel_ids().collect();
    diff_from_sets(&old_ids, &new_ids, old, new)
}

/// Reusable scratch buffers for diffing without per-frame HashSet allocation.
#[derive(Default)]
pub(crate) struct DiffScratch {
    old_ids: FxHashSet<PanelId>,
    new_ids: FxHashSet<PanelId>,
}

/// Compare two resolved layouts, reusing scratch HashSets across frames.
pub(crate) fn diff_reuse(
    old: &ResolvedLayout,
    new: &ResolvedLayout,
    scratch: &mut DiffScratch,
) -> LayoutDiff {
    scratch.old_ids.clear();
    scratch.old_ids.extend(old.panel_ids());
    scratch.new_ids.clear();
    scratch.new_ids.extend(new.panel_ids());
    diff_from_sets(&scratch.old_ids, &scratch.new_ids, old, new)
}

/// Diff when the panel set is known to be identical (tree not dirty).
///
/// Skips HashSet construction entirely — single pass over new layout.
pub(crate) fn diff_same_panels(old: &ResolvedLayout, new: &ResolvedLayout) -> LayoutDiff {
    let panel_hint = new.iter().size_hint().0;
    let mut moved = Vec::with_capacity(panel_hint);
    let mut resized = Vec::with_capacity(panel_hint);
    let mut unchanged = Vec::with_capacity(panel_hint);

    for (pid, new_rect) in new.iter() {
        let Some(old_rect) = old.get(pid) else {
            continue;
        };
        classify_change(
            pid,
            old_rect,
            new_rect,
            &mut moved,
            &mut resized,
            &mut unchanged,
        );
    }

    LayoutDiff {
        added: Box::default(),
        removed: Box::default(),
        moved: moved.into_boxed_slice(),
        resized: resized.into_boxed_slice(),
        unchanged: unchanged.into_boxed_slice(),
    }
}

/// Produce a diff representing the first frame — all panels are added.
pub fn first_frame(layout: &ResolvedLayout) -> LayoutDiff {
    LayoutDiff {
        added: layout.panel_ids().collect::<Vec<_>>().into_boxed_slice(),
        removed: Box::default(),
        moved: Box::default(),
        resized: Box::default(),
        unchanged: Box::default(),
    }
}
