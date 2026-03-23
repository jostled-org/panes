use rustc_hash::FxHashMap;
use std::sync::Arc;

use crate::compiler::CompileResult;
use crate::error::{PaneError, TreeError};
use crate::node::{Node, NodeId, PanelId};
use crate::overlay::{OverlayEntry, OverlayId};
use crate::rect::Rect;
use crate::tree::LayoutTree;

/// Axis of a resize boundary between sibling nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryAxis {
    /// Vertical boundary (between siblings in a row).
    Vertical,
    /// Horizontal boundary (between siblings in a column).
    Horizontal,
}

/// Result of a boundary hit-test: the resize handle closest to a query point.
#[derive(Debug, Clone, Copy)]
pub struct BoundaryHit {
    /// Whether this boundary runs vertically or horizontally.
    pub axis: BoundaryAxis,
    /// The two sibling nodes on either side of the boundary.
    pub sides: (NodeId, NodeId),
    /// The position of the boundary on its axis (x for Vertical, y for Horizontal).
    pub position: f32,
}

/// Internal boundary segment computed during resolve.
#[derive(Debug, Clone, Copy)]
struct BoundarySegment {
    axis: BoundaryAxis,
    /// Position on the main axis (x for Vertical, y for Horizontal).
    position: f32,
    /// Perpendicular span start (y for Vertical, x for Horizontal).
    span_start: f32,
    /// Perpendicular span end.
    span_end: f32,
    /// Node before the boundary (left/top).
    before: NodeId,
    /// Node after the boundary (right/bottom).
    after: NodeId,
}

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
    /// Zero-based index of this panel's kind group in iteration order.
    pub kind_index: usize,
}

impl<'a, R> PanelEntry<'a, R> {
    /// Transform the rect while preserving identity fields.
    pub fn map_rect<R2>(self, f: impl FnOnce(R) -> R2) -> PanelEntry<'a, R2> {
        PanelEntry {
            id: self.id,
            kind: self.kind,
            rect: f(self.rect),
            kind_index: self.kind_index,
        }
    }
}

/// Shared index mapping panel kind strings to their panel IDs.
pub(crate) type KindIndex = Arc<FxHashMap<Arc<str>, Box<[PanelId]>>>;

fn sorted_keys(kinds: &KindIndex) -> Box<[Arc<str>]> {
    let mut keys: Vec<_> = kinds.keys().map(Arc::clone).collect();
    keys.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
    keys.into_boxed_slice()
}

/// Resolved layout mapping each panel to its computed screen rectangle.
pub struct ResolvedLayout {
    rects: Vec<Option<Rect>>,
    kinds: KindIndex,
    sorted_kind_keys: Box<[Arc<str>]>,
    overlay_rects: Vec<(OverlayId, Arc<str>, Rect)>,
    boundaries: Box<[BoundarySegment]>,
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
        self.rects.iter().enumerate().filter_map(|(i, slot)| {
            let pid = PanelId::from_raw(u32::try_from(i).ok()?);
            slot.as_ref().map(|r| (pid, r))
        })
    }

    /// Iterate over all resolved panel ids.
    pub fn panel_ids(&self) -> impl Iterator<Item = PanelId> + '_ {
        self.rects.iter().enumerate().filter_map(|(i, slot)| {
            let pid = PanelId::from_raw(u32::try_from(i).ok()?);
            slot.as_ref().map(|_| pid)
        })
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
    /// Kind groups are sorted lexicographically so `kind_index` is stable
    /// across runs regardless of hash-map iteration order.
    pub fn panels(&self) -> impl Iterator<Item = PanelEntry<'_, &Rect>> + '_ {
        self.sorted_kind_keys
            .iter()
            .enumerate()
            .flat_map(move |(kind_index, kind)| {
                let pids = self.kinds.get(kind).map(|b| b.as_ref()).unwrap_or(&[]);
                pids.iter().filter_map(move |&pid| {
                    self.get(pid).map(|rect| PanelEntry {
                        id: pid,
                        kind: kind.as_ref(),
                        rect,
                        kind_index,
                    })
                })
            })
    }

    /// Iterate resolved overlays in z-order (insertion order).
    pub fn overlays(&self) -> impl Iterator<Item = OverlayEntry<'_, &Rect>> {
        self.overlay_rects
            .iter()
            .map(|(id, kind, rect)| OverlayEntry {
                id: *id,
                kind: kind.as_ref(),
                rect,
            })
    }

    /// Return the panel whose rect contains the given point.
    ///
    /// Iterates panels in reverse insertion order so that later (higher z)
    /// panels win when rects overlap. Overlays are ignored — use
    /// [`overlay_at_point`](Self::overlay_at_point) for overlay hit-testing.
    pub fn panel_at_point(&self, x: f32, y: f32) -> Option<PanelId> {
        self.rects
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(i, slot)| {
                let rect = slot.as_ref()?;
                let raw = u32::try_from(i).ok()?;
                rect.contains(x, y).then(|| PanelId::from_raw(raw))
            })
            .next()
    }

    /// Return the overlay whose rect contains the given point.
    ///
    /// Iterates in reverse z-order (topmost overlay first) so the
    /// visually-frontmost overlay wins.
    pub fn overlay_at_point(&self, x: f32, y: f32) -> Option<OverlayId> {
        self.overlay_rects
            .iter()
            .rev()
            .find(|(_, _, rect)| rect.contains(x, y))
            .map(|(id, _, _)| *id)
    }

    /// Look up the resolved rectangle for an overlay by its id.
    pub fn overlay_rect(&self, id: OverlayId) -> Option<&Rect> {
        self.overlay_rects
            .iter()
            .find(|(oid, _, _)| *oid == id)
            .map(|(_, _, r)| r)
    }

    /// Return the resize boundary closest to the given point within tolerance.
    ///
    /// Scans pre-computed boundary segments between adjacent siblings.
    /// Returns the nearest boundary whose main-axis distance is within
    /// `tolerance` and whose perpendicular span covers the query point.
    pub fn boundary_at_point(&self, x: f32, y: f32, tolerance: f32) -> Option<BoundaryHit> {
        let mut best: Option<(f32, &BoundarySegment)> = None;

        for seg in &self.boundaries {
            let (dist, in_span) = match seg.axis {
                BoundaryAxis::Vertical => (
                    (x - seg.position).abs(),
                    y >= seg.span_start && y < seg.span_end,
                ),
                BoundaryAxis::Horizontal => (
                    (y - seg.position).abs(),
                    x >= seg.span_start && x < seg.span_end,
                ),
            };
            match (dist <= tolerance && in_span, &best) {
                (true, Some((best_dist, _))) if dist < *best_dist => {
                    best = Some((dist, seg));
                }
                (true, None) => best = Some((dist, seg)),
                _ => {}
            }
        }

        best.map(|(_, seg)| BoundaryHit {
            axis: seg.axis,
            sides: (seg.before, seg.after),
            position: seg.position,
        })
    }

    /// Raw overlay rects for diffing.
    pub(crate) fn overlay_rects_raw(&self) -> &[(OverlayId, Arc<str>, Rect)] {
        &self.overlay_rects
    }

    /// Swap the resolved overlay rects buffer into the layout, returning the
    /// layout's previous buffer so the caller retains its capacity.
    pub(crate) fn swap_overlay_rects(&mut self, buf: &mut Vec<(OverlayId, Arc<str>, Rect)>) {
        std::mem::swap(&mut self.overlay_rects, buf);
    }

    /// Borrow the shared kinds index.
    pub(crate) fn kinds_arc(&self) -> &KindIndex {
        &self.kinds
    }

    /// Take ownership of the rects buffer for reuse.
    pub fn take_rects(&mut self) -> Vec<Option<Rect>> {
        std::mem::take(&mut self.rects)
    }

    /// Take ownership of the overlay rects buffer for reuse.
    pub fn take_overlay_rects(&mut self) -> Vec<(OverlayId, Arc<str>, Rect)> {
        std::mem::take(&mut self.overlay_rects)
    }

    /// Linearly interpolate between two resolved layouts.
    ///
    /// Panels in `self` but not `other` interpolate against themselves (no-op).
    /// Panels only in `other` are excluded.
    pub fn lerp(&self, other: &ResolvedLayout, t: f32) -> ResolvedLayout {
        let mut buf = Vec::new();
        self.lerp_into(other, t, &mut buf)
    }

    /// Interpolate into a reusable buffer, avoiding per-call allocation.
    ///
    /// The caller can reclaim the buffer from the returned layout via
    /// [`take_rects`](ResolvedLayout::take_rects).
    pub fn lerp_into(
        &self,
        other: &ResolvedLayout,
        t: f32,
        buf: &mut Vec<Option<Rect>>,
    ) -> ResolvedLayout {
        let taken = std::mem::take(buf);
        let mut rects = prepare_rects_buf(Some(taken), self.rects.len());

        for (i, from_rect) in self.rects.iter().enumerate() {
            let Some(from_rect) = from_rect else { continue };
            let Some(raw) = u32::try_from(i).ok() else {
                continue;
            };
            let pid = PanelId::from_raw(raw);
            let to_rect = other.get(pid).unwrap_or(from_rect);
            rects[i] = Some(from_rect.lerp(*to_rect, t));
        }

        let kinds = Arc::clone(&self.kinds);
        let sorted_kind_keys = self.sorted_kind_keys.clone();
        ResolvedLayout {
            rects,
            kinds,
            sorted_kind_keys,
            overlay_rects: Vec::new(),
            boundaries: Box::default(),
        }
    }
}

/// Iterative DFS that populates both rects and kinds. Reuses the stack across frames.
fn resolve_iterative_with_kinds(
    tree: &LayoutTree,
    result: &CompileResult,
    root_id: NodeId,
    rects: &mut [Option<Rect>],
    scratch: &mut ResolveScratch,
) -> Result<(), PaneError> {
    scratch.stack.clear();
    scratch.boundary_buf.clear();
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
            .map_err(|e| PaneError::InvalidTree(TreeError::TaffyError(e.to_string().into())))?;
        let abs_x = parent_x + layout.location.x;
        let abs_y = parent_y + layout.location.y;

        match tree.node(node_id) {
            Some(Node::Panel { id, kind, .. }) => {
                *rects
                    .get_mut(id.raw() as usize)
                    .ok_or(PaneError::PanelNotFound(*id))? = Some(Rect {
                    x: abs_x,
                    y: abs_y,
                    w: layout.size.width,
                    h: layout.size.height,
                });
                scratch
                    .kinds_buf
                    .entry(Arc::clone(kind))
                    .or_default()
                    .push(*id);
            }
            Some(Node::Row { children, .. }) => {
                emit_boundaries(
                    scratch.collect_boundaries,
                    result,
                    children,
                    BoundaryAxis::Vertical,
                    (abs_x, abs_y),
                    &layout.size,
                    &mut scratch.boundary_buf,
                );
                for &child_id in children.iter().rev() {
                    scratch.stack.push((child_id, abs_x, abs_y));
                }
            }
            Some(Node::Col { children, .. }) => {
                emit_boundaries(
                    scratch.collect_boundaries,
                    result,
                    children,
                    BoundaryAxis::Horizontal,
                    (abs_x, abs_y),
                    &layout.size,
                    &mut scratch.boundary_buf,
                );
                for &child_id in children.iter().rev() {
                    scratch.stack.push((child_id, abs_x, abs_y));
                }
            }
            Some(Node::TaffyPassthrough { children, .. }) => {
                for &child_id in children.iter().rev() {
                    scratch.stack.push((child_id, abs_x, abs_y));
                }
            }
            None => {}
        }
    }
    Ok(())
}

/// Reusable scratch state for DFS resolution.
pub(crate) struct ResolveScratch {
    stack: Vec<(NodeId, f32, f32)>,
    kinds_buf: FxHashMap<Arc<str>, Vec<PanelId>>,
    boundary_buf: Vec<BoundarySegment>,
    /// When false, skip boundary collection during resolve.
    pub(crate) collect_boundaries: bool,
}

impl Default for ResolveScratch {
    fn default() -> Self {
        Self {
            stack: Vec::new(),
            kinds_buf: FxHashMap::default(),
            boundary_buf: Vec::new(),
            collect_boundaries: true,
        }
    }
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
    scratch.boundary_buf.clear();
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
            .map_err(|e| PaneError::InvalidTree(TreeError::TaffyError(e.to_string().into())))?;
        let abs_x = parent_x + layout.location.x;
        let abs_y = parent_y + layout.location.y;

        match tree.node(node_id) {
            Some(Node::Panel { id, .. }) => {
                *rects
                    .get_mut(id.raw() as usize)
                    .ok_or(PaneError::PanelNotFound(*id))? = Some(Rect {
                    x: abs_x,
                    y: abs_y,
                    w: layout.size.width,
                    h: layout.size.height,
                });
            }
            Some(Node::Row { children, .. }) => {
                emit_boundaries(
                    scratch.collect_boundaries,
                    result,
                    children,
                    BoundaryAxis::Vertical,
                    (abs_x, abs_y),
                    &layout.size,
                    &mut scratch.boundary_buf,
                );
                for &child_id in children.iter().rev() {
                    scratch.stack.push((child_id, abs_x, abs_y));
                }
            }
            Some(Node::Col { children, .. }) => {
                emit_boundaries(
                    scratch.collect_boundaries,
                    result,
                    children,
                    BoundaryAxis::Horizontal,
                    (abs_x, abs_y),
                    &layout.size,
                    &mut scratch.boundary_buf,
                );
                for &child_id in children.iter().rev() {
                    scratch.stack.push((child_id, abs_x, abs_y));
                }
            }
            Some(Node::TaffyPassthrough { children, .. }) => {
                for &child_id in children.iter().rev() {
                    scratch.stack.push((child_id, abs_x, abs_y));
                }
            }
            None => {}
        }
    }
    Ok(())
}

/// Emit boundary segments between adjacent children of a container.
///
/// Called during the main DFS where the container's absolute position and
/// layout size are already known, avoiding a separate ancestor walk.
/// No-op when `collect` is false.
fn emit_boundaries(
    collect: bool,
    result: &CompileResult,
    children: &[NodeId],
    axis: BoundaryAxis,
    abs: (f32, f32),
    container_size: &taffy::Size<f32>,
    boundaries: &mut Vec<BoundarySegment>,
) {
    if !collect || children.len() < 2 {
        return;
    }
    let (container_abs_x, container_abs_y) = abs;

    for pair in children.windows(2) {
        let (a_id, b_id) = (pair[0], pair[1]);
        let (Some(a_layout), Some(b_layout)) =
            (child_layout(result, a_id), child_layout(result, b_id))
        else {
            continue;
        };

        let (position, span_start, span_end) = match axis {
            BoundaryAxis::Vertical => {
                let a_abs_x = container_abs_x + a_layout.location.x;
                let b_abs_x = container_abs_x + b_layout.location.x;
                let pos = (a_abs_x + a_layout.size.width + b_abs_x) / 2.0;
                (
                    pos,
                    container_abs_y,
                    container_abs_y + container_size.height,
                )
            }
            BoundaryAxis::Horizontal => {
                let a_abs_y = container_abs_y + a_layout.location.y;
                let b_abs_y = container_abs_y + b_layout.location.y;
                let pos = (a_abs_y + a_layout.size.height + b_abs_y) / 2.0;
                (pos, container_abs_x, container_abs_x + container_size.width)
            }
        };

        boundaries.push(BoundarySegment {
            axis,
            position,
            span_start,
            span_end,
            before: a_id,
            after: b_id,
        });
    }
}

/// Look up a node's taffy layout (relative to its parent).
fn child_layout(result: &CompileResult, node_id: NodeId) -> Option<&taffy::Layout> {
    let taffy_id = result.node_map.get(node_id.raw() as usize)?.as_ref()?;
    result.taffy_tree.layout(*taffy_id).ok()
}

/// Prepare a rects buffer: clear existing slots and resize to `capacity`,
/// or allocate a fresh one if no buffer is provided.
fn prepare_rects_buf(buf: Option<Vec<Option<Rect>>>, capacity: usize) -> Vec<Option<Rect>> {
    match buf {
        Some(mut buf) => {
            buf.iter_mut().for_each(|slot| *slot = None);
            buf.resize(capacity, None);
            buf
        }
        None => vec![None; capacity],
    }
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
        .ok_or(PaneError::InvalidTree(TreeError::RootNotSet))?;

    let capacity = tree.panel_id_high_water() as usize;
    let mut rects = prepare_rects_buf(rects_buf, capacity);
    resolve_iterative(tree, result, root_id, &mut rects, scratch)?;

    let boundaries = match scratch.collect_boundaries {
        true => scratch.boundary_buf.as_slice().into(),
        false => Box::default(),
    };
    let sorted_kind_keys = sorted_keys(&kinds);
    Ok(ResolvedLayout {
        rects,
        kinds,
        sorted_kind_keys,
        overlay_rects: Vec::new(),
        boundaries,
    })
}

/// Walk the compiled Taffy tree and produce a `ResolvedLayout` mapping each panel to its rect.
pub fn resolve(result: &CompileResult, tree: &LayoutTree) -> Result<ResolvedLayout, PaneError> {
    resolve_dirty(result, tree, &mut ResolveScratch::default(), None)
}

/// Like [`resolve`] but reuses scratch buffers across frames.
pub(crate) fn resolve_dirty(
    result: &CompileResult,
    tree: &LayoutTree,
    scratch: &mut ResolveScratch,
    rects_buf: Option<Vec<Option<Rect>>>,
) -> Result<ResolvedLayout, PaneError> {
    let root_id = tree
        .root()
        .ok_or(PaneError::InvalidTree(TreeError::RootNotSet))?;

    let capacity = tree.panel_id_high_water() as usize;
    let mut rects = prepare_rects_buf(rects_buf, capacity);

    // Reuse kinds buffer: clear values but retain map capacity.
    for v in scratch.kinds_buf.values_mut() {
        v.clear();
    }

    resolve_iterative_with_kinds(tree, result, root_id, &mut rects, scratch)?;

    // Remove stale entries for panel kinds no longer present in the tree.
    scratch.kinds_buf.retain(|_, v| !v.is_empty());

    let kinds = Arc::new(
        scratch
            .kinds_buf
            .iter()
            .map(|(k, v)| (Arc::clone(k), v.as_slice().into()))
            .collect(),
    );

    let boundaries = match scratch.collect_boundaries {
        true => scratch.boundary_buf.as_slice().into(),
        false => Box::default(),
    };
    let sorted_kind_keys = sorted_keys(&kinds);
    Ok(ResolvedLayout {
        rects,
        kinds,
        sorted_kind_keys,
        overlay_rects: Vec::new(),
        boundaries,
    })
}
