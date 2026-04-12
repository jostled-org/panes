#![allow(clippy::unwrap_used, clippy::panic)]
use panes::Layout;

#[test]
fn panels_yields_all_panels() {
    let layout = Layout::row(["a", "b", "c"]).unwrap();
    let resolved = layout.resolve(300.0, 100.0).unwrap();

    let entries: Vec<_> = resolved.panels().collect();
    assert_eq!(entries.len(), 3);

    for entry in &entries {
        let rect = resolved.get(entry.id).unwrap();
        assert_eq!(entry.rect, rect);
    }
}

#[test]
fn panels_grouped_by_kind() {
    let layout = Layout::row(["x", "y", "x"]).unwrap();
    let resolved = layout.resolve(300.0, 100.0).unwrap();

    let kinds: Vec<&str> = resolved.panels().map(|e| e.kind).collect();

    // All entries of the same kind should be contiguous
    let mut seen = std::collections::HashSet::new();
    let mut prev = "";
    for kind in &kinds {
        if *kind != prev {
            assert!(
                seen.insert(*kind),
                "kind {kind} appeared in a non-contiguous run"
            );
            prev = kind;
        }
    }
}

#[test]
fn panels_entry_kind_matches_by_kind() {
    let layout = Layout::row(["editor", "terminal"]).unwrap();
    let resolved = layout.resolve(400.0, 200.0).unwrap();

    for entry in resolved.panels() {
        let pids = resolved.by_kind(entry.kind);
        assert!(
            pids.contains(&entry.id),
            "entry.id not found in by_kind({:?})",
            entry.kind
        );
    }
}

#[test]
fn panels_empty_layout_yields_nothing() {
    // Single panel resolved at zero size still has one entry
    let layout = Layout::row(["a"]).unwrap();
    let resolved = layout.resolve(0.0, 0.0).unwrap();

    let count = resolved.panels().count();
    assert_eq!(count, 1);
}

#[test]
fn panels_kind_index_increments_per_group() {
    let layout = Layout::row(["x", "y", "x"]).unwrap();
    let resolved = layout.resolve(300.0, 100.0).unwrap();

    let entries: Vec<_> = resolved.panels().collect();

    // All entries of the same kind share one kind_index
    let mut seen_indices = std::collections::HashMap::new();
    for entry in &entries {
        let idx = *seen_indices.entry(entry.kind).or_insert(entry.kind_index);
        // First time seeing this kind: assign its index
        // Subsequent: must match
        assert_eq!(
            idx, entry.kind_index,
            "kind {:?} had inconsistent kind_index",
            entry.kind
        );
        // kind_index should be less than total distinct kinds
        assert!(entry.kind_index < 2, "kind_index out of range");
    }

    // Distinct kinds get distinct indices
    let indices: std::collections::HashSet<usize> = entries.iter().map(|e| e.kind_index).collect();
    assert_eq!(indices.len(), 2);
}
