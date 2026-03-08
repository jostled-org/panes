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
