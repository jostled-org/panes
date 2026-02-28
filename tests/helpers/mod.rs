use panes::{Constraints, LayoutTree, PanelId};

pub fn build_row_tree(count: usize, constraint: Constraints) -> (LayoutTree, Vec<PanelId>) {
    let mut tree = LayoutTree::new();
    let mut pids = Vec::new();
    let mut nids = Vec::new();
    for i in 0..count {
        let (pid, nid) = tree.add_panel(format!("p{i}"), constraint).unwrap();
        pids.push(pid);
        nids.push(nid);
    }
    let root = tree.add_row(0.0, nids).unwrap();
    tree.set_root(root);
    (tree, pids)
}
