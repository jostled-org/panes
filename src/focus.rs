use std::cmp::Ordering;

use crate::node::PanelId;
use crate::resolver::ResolvedLayout;
use crate::sequence::PanelSequence;

/// Spatial direction for focus navigation.
///
/// Distinct from [`Direction`](crate::Direction), which describes a container's
/// axis orientation (`Horizontal`/`Vertical`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusDirection {
    /// Move focus to the nearest panel on the left.
    Left,
    /// Move focus to the nearest panel on the right.
    Right,
    /// Move focus to the nearest panel above.
    Up,
    /// Move focus to the nearest panel below.
    Down,
}

/// Find the nearest panel in the given direction from `current`.
///
/// Scores candidates by `(primary_axis_distance, secondary_axis_distance)`
/// between centers. Returns `None` when no candidate lies in that direction.
pub(crate) fn find_nearest(
    layout: &ResolvedLayout,
    current: PanelId,
    candidates: &PanelSequence,
    direction: FocusDirection,
) -> Option<PanelId> {
    let origin = layout.get(current)?;
    let (ox, oy) = origin.center();

    candidates
        .iter()
        .filter(|&pid| pid != current)
        .filter_map(|pid| {
            let rect = layout.get(pid)?;
            match rect.area() > 0.0 {
                true => Some((pid, rect.center())),
                false => None,
            }
        })
        .filter(|&(_, (cx, cy))| match direction {
            FocusDirection::Left => cx < ox,
            FocusDirection::Right => cx > ox,
            FocusDirection::Up => cy < oy,
            FocusDirection::Down => cy > oy,
        })
        .filter(|&(_, (cx, cy))| {
            let (a, b) = direction_score(ox, oy, cx, cy, direction);
            a.is_finite() && b.is_finite()
        })
        .min_by(|&(_, (ax, ay)), &(_, (bx, by))| {
            let score_a = direction_score(ox, oy, ax, ay, direction);
            let score_b = direction_score(ox, oy, bx, by, direction);
            f32_pair_cmp(score_a, score_b)
        })
        .map(|(pid, _)| pid)
}

fn direction_score(ox: f32, oy: f32, cx: f32, cy: f32, direction: FocusDirection) -> (f32, f32) {
    match direction {
        FocusDirection::Left | FocusDirection::Right => ((cx - ox).abs(), (cy - oy).abs()),
        FocusDirection::Up | FocusDirection::Down => ((cy - oy).abs(), (cx - ox).abs()),
    }
}

fn f32_pair_cmp(a: (f32, f32), b: (f32, f32)) -> Ordering {
    a.0.partial_cmp(&b.0)
        .unwrap_or(Ordering::Greater)
        .then(a.1.partial_cmp(&b.1).unwrap_or(Ordering::Greater))
}
