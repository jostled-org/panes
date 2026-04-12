#![allow(clippy::unwrap_used)]

use panes::runtime::LayoutRuntime;
use panes::{Layout, Overlay};

fn make_runtime() -> LayoutRuntime {
    Layout::master_stack(["editor", "chat", "status"])
        .master_ratio(0.6)
        .gap(1.0)
        .into_runtime()
        .unwrap()
}

#[test]
fn overlay_diff_handles_mixed_overlay_states() {
    let mut rt = make_runtime();
    rt.add_overlay("stable", Overlay::top_left(0.0, 0.0).fixed(100.0, 100.0))
        .unwrap();
    rt.add_overlay("moves", Overlay::center().fixed(120.0, 80.0))
        .unwrap();
    rt.add_overlay("fails", Overlay::above("status").fixed(60.0, 10.0))
        .unwrap();
    rt.add_overlay("removed", Overlay::top(0.0).fixed(90.0, 30.0))
        .unwrap();

    let frame = rt.resolve(800.0, 600.0).unwrap();
    let status_id = frame.layout().by_kind("status")[0];

    rt.remove_overlay("removed");
    rt.add_overlay("added", Overlay::bottom(0.0).fixed(70.0, 25.0))
        .unwrap();
    rt.remove_panel(status_id).unwrap();

    rt.resolve(1000.0, 600.0).unwrap();
    let diff = rt.last_overlay_diff();

    let stable_id = rt.overlay("stable").unwrap().id();
    let moves_id = rt.overlay("moves").unwrap().id();
    let fails_id = rt.overlay("fails").unwrap().id();
    let added_id = rt.overlay("added").unwrap().id();

    assert!(diff.added.contains(&added_id));
    assert!(diff.removed.iter().all(|id| *id != added_id));
    assert!(diff.unchanged.contains(&stable_id));
    assert!(diff.moved.iter().any(|change| change.id == moves_id));
    assert!(diff.anchor_failed.contains(&fails_id));
}
