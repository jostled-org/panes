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

/// Shared index mapping panel kind strings to their panel IDs.
pub(crate) type KindIndex = Arc<FxHashMap<Arc<str>, Box<[PanelId]>>>;

/// Resolved layout mapping each panel to its computed screen rectangle.
#[derive(Clone)]
pub struct ResolvedLayout {
    rects: Vec<Option<Rect>>,
    kinds: KindIndex,
}

impl ResolvedLayout {
    /// Look up the resolved rectangle for a panel.
    pub fn get(&self, id: PanelId) -> Option<&Rect> {
        self.rects.get(id.raw() as usize)?.as_ref()
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
        self.rects
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.as_ref().map(|r| (PanelId::from_raw(i as u32), r)))
    }

    /// Iterate over all resolved panel ids.
    pub fn panel_ids(&self) -> impl Iterator<Item = PanelId> + '_ {
        self.rects
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.as_ref().map(|_| PanelId::from_raw(i as u32)))
    }

    /// Iterate over all distinct panel kinds present in the resolved layout.
    pub fn kinds(&self) -> impl Iterator<Item = &str> {
        self.kinds.keys().map(|k| k.as_ref())
    }

    /// Shift all panel rects' x-positions by the given delta.
    pub fn shift_x(&mut self, dx: f32) {
        for rect in self.rects.iter_mut().flatten() {
            rect.x += dx;
        }
    }

    /// Iterate all panels in kind-grouped order, yielding identity and rect together.
    ///
    /// All panels of one kind appear contiguously, then the next kind, etc.
    /// No allocation — this is a lazy iterator over the internal index.
    pub fn panels(&self) -> impl Iterator<Item = PanelEntry<'_, &Rect>> + '_ {
        self.kinds.iter().flat_map(move |(kind, pids)| {
            pids.iter().filter_map(move |&pid| {
                self.get(pid).map(|rect| PanelEntry {
                    id: pid,
                    kind: kind.as_ref(),
                    rect,
                })
            })
        })
    }

    /// Borrow the shared kinds index.
    pub(crate) fn kinds_arc(&self) -> &KindIndex {
        &self.kinds
    }

    /// Take ownership of the rects buffer for reuse.
    pub(crate) fn take_rects(&mut self) -> Vec<Option<Rect>> {
        std::mem::take(&mut self.rects)
    }

    /// Linearly interpolate between two resolved layouts.
    ///
    /// Panels in `self` but not `other` interpolate against themselves (no-op).
    /// Panels only in `other` are excluded.
    pub fn lerp(&self, other: &ResolvedLayout, t: f32) -> ResolvedLayout {
        let mut rects = vec![None; self.rects.len()];
        for (i, from_rect) in self.rects.iter().enumerate() {
            let Some(from_rect) = from_rect else { continue };
            let pid = PanelId::from_raw(i as u32);
            let to_rect = other.get(pid).unwrap_or(from_rect);
            rects[i] = Some(from_rect.lerp(*to_rect, t));
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
    rects: &mut [Option<Rect>],
    kinds: &mut FxHashMap<Arc<str>, Vec<PanelId>>,
) -> Result<(), PaneError> {
    let taffy_id = result
        .node_map
        .get(node_id.raw() as usize)
        .and_then(|s| s.as_ref())
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
            rects[id.raw() as usize] = Some(rect);
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

/// Reusable scratch state for iterative DFS resolution.
#[derive(Default)]
pub(crate) struct ResolveScratch {
    stack: Vec<(NodeId, f32, f32)>,
}

/// Iterative DFS that only populates rects. Reuses the stack across frames.
fn resolve_iterative(
    tree: &LayoutTree,
    result: &CompileResult,
    root_id: NodeId,
    rects: &mut [Option<Rect>],
    scratch: &mut ResolveScratch,
) -> Result<(), PaneError> {
    scratch.stack.clear();
    scratch.stack.push((root_id, 0.0, 0.0));

    while let Some((node_id, parent_x, parent_y)) = scratch.stack.pop() {
        let taffy_id = result
            .node_map
            .get(node_id.raw() as usize)
            .and_then(|s| s.as_ref())
            .ok_or(PaneError::NodeNotFound(node_id))?;
        let layout = result
            .taffy_tree
            .layout(*taffy_id)
            .map_err(|e| PaneError::InvalidTree(e.to_string().into()))?;
        let abs_x = parent_x + layout.location.x;
        let abs_y = parent_y + layout.location.y;

        match tree.node(node_id) {
            Some(Node::Panel { id, .. }) => {
                rects[id.raw() as usize] = Some(Rect {
                    x: abs_x,
                    y: abs_y,
                    w: layout.size.width,
                    h: layout.size.height,
                });
            }
            Some(
                Node::Row { children, .. }
                | Node::Col { children, .. }
                | Node::TaffyPassthrough { children, .. },
            ) => {
                for &child_id in children.iter().rev() {
                    scratch.stack.push((child_id, abs_x, abs_y));
                }
            }
            None => {}
        }
    }
    Ok(())
}

/// Resolve rects using a previously cached kinds index. Skips kinds population.
/// Uses iterative DFS with a reusable stack.
pub(crate) fn resolve_with_cached_kinds(
    result: &CompileResult,
    tree: &LayoutTree,
    kinds: KindIndex,
    scratch: &mut ResolveScratch,
    rects_buf: Option<Vec<Option<Rect>>>,
) -> Result<ResolvedLayout, PaneError> {
    let root_id = tree
        .root()
        .ok_or_else(|| PaneError::InvalidTree("root is not set".into()))?;

    let capacity = tree.panel_id_high_water() as usize;
    let mut rects = match rects_buf {
        Some(mut buf) => {
            buf.iter_mut().for_each(|slot| *slot = None);
            buf.resize(capacity, None);
            buf
        }
        None => vec![None; capacity],
    };
    resolve_iterative(tree, result, root_id, &mut rects, scratch)?;

    Ok(ResolvedLayout { rects, kinds })
}

/// Walk the compiled Taffy tree and produce a `ResolvedLayout` mapping each panel to its rect.
pub fn resolve(result: &CompileResult, tree: &LayoutTree) -> Result<ResolvedLayout, PaneError> {
    let root_id = tree
        .root()
        .ok_or_else(|| PaneError::InvalidTree("root is not set".into()))?;

    let mut rects = vec![None; tree.panel_id_high_water() as usize];
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
