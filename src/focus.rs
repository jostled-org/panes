use std::cmp::Ordering;

use crate::node::PanelId;
use crate::rect::Rect;
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
/// Candidates that overlap on the cross-axis are preferred over those
/// that don't. Among equally-overlapping candidates, the nearest on
/// the primary axis wins, with secondary axis distance as tiebreaker.
pub(crate) fn find_nearest(
    layout: &ResolvedLayout,
    current: PanelId,
    candidates: &PanelSequence,
    direction: FocusDirection,
) -> Option<PanelId> {
    let origin = layout.get(current)?;

    candidates
        .iter()
        .filter(|&pid| pid != current)
        .filter_map(|pid| {
            let rect = layout.get(pid)?;
            match rect.area() > 0.0 {
                true => Some((pid, *rect)),
                false => None,
            }
        })
        .filter(|(_, rect)| {
            let (cx, cy) = rect.center();
            let (ox, oy) = origin.center();
            match direction {
                FocusDirection::Left => cx < ox,
                FocusDirection::Right => cx > ox,
                FocusDirection::Up => cy < oy,
                FocusDirection::Down => cy > oy,
            }
        })
        .min_by(|(_, a), (_, b)| {
            let score_a = direction_score(origin, a, direction);
            let score_b = direction_score(origin, b, direction);
            cmp_score(score_a, score_b)
        })
        .map(|(pid, _)| pid)
}

/// Score a candidate: `(no_overlap, primary_distance, secondary_distance)`.
///
/// `no_overlap` is false when the candidate overlaps on the cross-axis,
/// placing it ahead of non-overlapping candidates (false < true).
fn direction_score(origin: &Rect, candidate: &Rect, direction: FocusDirection) -> (bool, f32, f32) {
    let (ox, oy) = origin.center();
    let (cx, cy) = candidate.center();

    let overlaps = match direction {
        FocusDirection::Left | FocusDirection::Right => ranges_overlap(
            origin.y,
            origin.y + origin.h,
            candidate.y,
            candidate.y + candidate.h,
        ),
        FocusDirection::Up | FocusDirection::Down => ranges_overlap(
            origin.x,
            origin.x + origin.w,
            candidate.x,
            candidate.x + candidate.w,
        ),
    };

    let (primary, secondary) = match direction {
        FocusDirection::Left | FocusDirection::Right => ((cx - ox).abs(), (cy - oy).abs()),
        FocusDirection::Up | FocusDirection::Down => ((cy - oy).abs(), (cx - ox).abs()),
    };

    (!overlaps, primary, secondary)
}

fn ranges_overlap(a_start: f32, a_end: f32, b_start: f32, b_end: f32) -> bool {
    a_start < b_end && b_start < a_end
}

fn cmp_score(a: (bool, f32, f32), b: (bool, f32, f32)) -> Ordering {
    a.0.cmp(&b.0)
        .then(a.1.partial_cmp(&b.1).unwrap_or(Ordering::Greater))
        .then(a.2.partial_cmp(&b.2).unwrap_or(Ordering::Greater))
}
