use rustc_hash::FxHashSet;

use crate::node::PanelId;
use crate::rect::Rect;
use crate::resolver::ResolvedLayout;

const EPSILON: f32 = 1e-4;

/// A panel whose rect changed between two frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectChange {
    pub id: PanelId,
    pub from: Rect,
    pub to: Rect,
}

/// Categorized differences between two resolved layouts.
#[derive(Debug)]
pub struct LayoutDiff {
    pub added: Box<[PanelId]>,
    pub removed: Box<[PanelId]>,
    pub moved: Box<[RectChange]>,
    pub resized: Box<[RectChange]>,
    pub unchanged: Box<[PanelId]>,
}

fn position_changed(a: &Rect, b: &Rect) -> bool {
    (a.x - b.x).abs() > EPSILON || (a.y - b.y).abs() > EPSILON
}

fn size_changed(a: &Rect, b: &Rect) -> bool {
    (a.w - b.w).abs() > EPSILON || (a.h - b.h).abs() > EPSILON
}

/// Compare two resolved layouts and categorize every panel.
pub fn diff(old: &ResolvedLayout, new: &ResolvedLayout) -> LayoutDiff {
    let old_ids: FxHashSet<PanelId> = old.panel_ids().collect();
    let new_ids: FxHashSet<PanelId> = new.panel_ids().collect();

    let removed: Box<[PanelId]> = old_ids.difference(&new_ids).copied().collect();
    let added: Box<[PanelId]> = new_ids.difference(&old_ids).copied().collect();

    let common_count = old_ids.intersection(&new_ids).count();
    let mut moved = Vec::with_capacity(common_count);
    let mut resized = Vec::with_capacity(common_count);
    let mut unchanged = Vec::with_capacity(common_count);

    for &pid in old_ids.intersection(&new_ids) {
        let (old_rect, new_rect) = match (old.get(pid), new.get(pid)) {
            (Some(o), Some(n)) => (o, n),
            _ => continue,
        };

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

    LayoutDiff {
        added,
        removed,
        moved: moved.into_boxed_slice(),
        resized: resized.into_boxed_slice(),
        unchanged: unchanged.into_boxed_slice(),
    }
}

/// Produce a diff representing the first frame — all panels are added.
pub fn first_frame(layout: &ResolvedLayout) -> LayoutDiff {
    LayoutDiff {
        added: layout.panel_ids().collect(),
        removed: Box::default(),
        moved: Box::default(),
        resized: Box::default(),
        unchanged: Box::default(),
    }
}
