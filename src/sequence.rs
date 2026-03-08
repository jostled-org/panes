use crate::node::PanelId;

/// Flat ordered list of panel IDs, separate from tree topology.
///
/// Tracks the logical order of panels for focus navigation and
/// strategy-based mutations (add, remove, move).
#[derive(Debug, Clone, Default)]
pub struct PanelSequence {
    ids: Vec<PanelId>,
}

impl PanelSequence {
    /// Number of panels in the sequence.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether the sequence is empty.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Get the panel at the given index.
    pub fn get(&self, index: usize) -> Option<PanelId> {
        self.ids.get(index).copied()
    }

    /// Find the index of a panel, if present.
    pub fn index_of(&self, pid: PanelId) -> Option<usize> {
        self.ids.iter().position(|&id| id == pid)
    }

    /// Append a panel to the end.
    pub fn push(&mut self, pid: PanelId) {
        self.ids.push(pid);
    }

    /// Insert a panel at the given index.
    pub fn insert(&mut self, index: usize, pid: PanelId) {
        self.ids.insert(index, pid);
    }

    /// Remove a panel by id. Returns its former index, or `None` if absent.
    pub fn remove(&mut self, pid: PanelId) -> Option<usize> {
        let idx = self.index_of(pid)?;
        self.ids.remove(idx);
        Some(idx)
    }

    /// Move a panel to a new index. Returns `None` if the panel is absent
    /// or the target index is out of bounds.
    pub fn move_to(&mut self, pid: PanelId, new_index: usize) -> Option<usize> {
        let old = self.index_of(pid)?;
        match new_index >= self.ids.len() {
            true => return None,
            false => {}
        }
        self.ids.remove(old);
        self.ids.insert(new_index, pid);
        Some(old)
    }

    /// After removing the panel at `removed_index`, return the best neighbor
    /// to receive focus. Prefers the panel that was after the removed one;
    /// falls back to the one before. Returns `None` if the sequence is empty
    /// after removal.
    pub fn neighbor_after_removal(&self, removed_index: usize) -> Option<PanelId> {
        match self.ids.is_empty() {
            true => None,
            false => {
                let idx = removed_index.min(self.ids.len() - 1);
                Some(self.ids[idx])
            }
        }
    }

    /// Iterate over panel IDs in order.
    pub fn iter(&self) -> impl Iterator<Item = PanelId> + '_ {
        self.ids.iter().copied()
    }

    /// View the underlying slice.
    pub fn as_slice(&self) -> &[PanelId] {
        &self.ids
    }
}
