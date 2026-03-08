use rustc_hash::FxHashMap;
use std::sync::Arc;

use crate::compiler::CompileResult;
use crate::error::PaneError;
use crate::node::{Node, NodeId, PanelId};
use crate::rect::Rect;
use crate::tree::LayoutTree;

/// A single panel's identity and computed rectangle.
///
/// Generic over `R` so the core crate yields `PanelEntry<'_, &Rect>` while
/// output crates yield their own rect type (e.g. `ratatui::Rect`, `egui::Rect`).
pub struct PanelEntry<'a, R> {
    /// Panel identifier.
    pub id: PanelId,
    /// Panel kind string (e.g. `"editor"`, `"terminal"`).
    pub kind: &'a str,
    /// Computed rectangle in the target coordinate system.
    pub rect: R,
}

/// Resolved layout mapping each panel to its computed screen rectangle.
#[derive(Clone)]
pub struct ResolvedLayout {
    rects: FxHashMap<PanelId, Rect>,
    kinds: Arc<FxHashMap<Arc<str>, Box<[PanelId]>>>,
}

impl ResolvedLayout {
    /// Look up the resolved rectangle for a panel.
    pub fn get(&self, id: PanelId) -> Option<&Rect> {
        self.rects.get(&id)
    }

    /// All panel ids with the given kind. Empty slice if kind is absent.
    pub fn by_kind(&self, kind: &str) -> &[PanelId] {
        match self.kinds.get(kind) {
            Some(ids) => ids,
            None => &[],
        }
    }

    /// Iterate over all (PanelId, Rect) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (PanelId, &Rect)> {
        self.rects.iter().map(|(&pid, rect)| (pid, rect))
    }

    /// Iterate over all resolved panel ids.
    pub fn panel_ids(&self) -> impl Iterator<Item = PanelId> + '_ {
        self.rects.keys().copied()
    }

    /// Iterate over all distinct panel kinds present in the resolved layout.
    pub fn kinds(&self) -> impl Iterator<Item = &str> {
        self.kinds.keys().map(|k| k.as_ref())
    }

    /// Shift all panel rects' x-positions by the given delta.
    pub fn shift_x(&mut self, dx: f32) {
        for rect in self.rects.values_mut() {
            rect.x += dx;
        }
    }

    /// Iterate all panels in kind-grouped order, yielding identity and rect together.
    ///
    /// All panels of one kind appear contiguously, then the next kind, etc.
    /// No allocation — this is a lazy iterator over the internal index.
    pub fn panels(&self) -> impl Iterator<Item = PanelEntry<'_, &Rect>> + '_ {
        let rects = &self.rects;
        self.kinds.iter().flat_map(move |(kind, pids)| {
            pids.iter().filter_map(move |&pid| {
                rects.get(&pid).map(|rect| PanelEntry {
                    id: pid,
                    kind: kind.as_ref(),
                    rect,
                })
            })
        })
    }

    /// Linearly interpolate between two resolved layouts.
    ///
    /// Panels in `self` but not `other` interpolate against themselves (no-op).
    /// Panels only in `other` are excluded.
    pub fn lerp(&self, other: &ResolvedLayout, t: f32) -> ResolvedLayout {
        let mut rects = FxHashMap::default();
        for (&pid, from_rect) in &self.rects {
            let to_rect = other.rects.get(&pid).unwrap_or(from_rect);
            rects.insert(pid, from_rect.lerp(*to_rect, t));
        }
        let kinds = Arc::clone(&self.kinds);
        ResolvedLayout { rects, kinds }
    }
}

/// Single-pass top-down DFS to resolve all panel rects with accumulated offsets.
fn resolve_dfs(
    tree: &LayoutTree,
    result: &CompileResult,
    node_id: NodeId,
    parent_x: f32,
    parent_y: f32,
    rects: &mut FxHashMap<PanelId, Rect>,
    kinds: &mut FxHashMap<Arc<str>, Vec<PanelId>>,
) -> Result<(), PaneError> {
    let taffy_id = result
        .node_map
        .get(&node_id)
        .ok_or(PaneError::NodeNotFound(node_id))?;
    let layout = result
        .taffy_tree
        .layout(*taffy_id)
        .map_err(|e| PaneError::InvalidTree(e.to_string().into()))?;
    let abs_x = parent_x + layout.location.x;
    let abs_y = parent_y + layout.location.y;

    match tree.node(node_id) {
        Some(Node::Panel { id, kind, .. }) => {
            let rect = Rect {
                x: abs_x,
                y: abs_y,
                w: layout.size.width,
                h: layout.size.height,
            };
            rects.insert(*id, rect);
            kinds.entry(Arc::clone(kind)).or_default().push(*id);
        }
        Some(
            Node::Row { children, .. }
            | Node::Col { children, .. }
            | Node::TaffyPassthrough { children, .. },
        ) => {
            for &child_id in children {
                resolve_dfs(tree, result, child_id, abs_x, abs_y, rects, kinds)?;
            }
        }
        None => {}
    }
    Ok(())
}

/// Walk the compiled Taffy tree and produce a `ResolvedLayout` mapping each panel to its rect.
pub fn resolve(result: &CompileResult, tree: &LayoutTree) -> Result<ResolvedLayout, PaneError> {
    let root_id = tree
        .root()
        .ok_or_else(|| PaneError::InvalidTree("root is not set".into()))?;

    let mut rects = FxHashMap::with_capacity_and_hasher(tree.panel_count(), Default::default());
    let mut kinds: FxHashMap<Arc<str>, Vec<PanelId>> =
        FxHashMap::with_capacity_and_hasher(tree.kind_count(), Default::default());

    resolve_dfs(tree, result, root_id, 0.0, 0.0, &mut rects, &mut kinds)?;

    let kinds = Arc::new(
        kinds
            .into_iter()
            .map(|(k, v)| (k, v.into_boxed_slice()))
            .collect(),
    );

    Ok(ResolvedLayout { rects, kinds })
}
