use rustc_hash::FxHashMap;
use std::sync::Arc;

use crate::compiler::CompileResult;
use crate::decoration::DecorationRole;
use crate::error::{PaneError, TreeError};
use crate::node::{Node, NodeId, PanelId};
use crate::overlay::{AnchorFailure, OverlayEntry, OverlayId};
use crate::rect::Rect;
use crate::tree::LayoutTree;

/// Axis along which a resize boundary runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryAxis {
    /// A vertical boundary separating left/right siblings in a row.
    Vertical,
    /// A horizontal boundary separating top/bottom siblings in a column.
    Horizontal,
}

/// Result of a boundary hit-test: the resize handle closest to a query point.
#[derive(Debug, Clone, Copy)]
pub struct BoundaryHit {
    /// The axis this boundary runs along.
    pub axis: BoundaryAxis,
    /// The two node IDs on either side of the boundary (before, after).
    pub sides: (NodeId, NodeId),
    /// The absolute coordinate of the boundary along its perpendicular axis.
    pub position: f32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BoundarySegment {
    axis: BoundaryAxis,
    position: f32,
    span_start: f32,
    span_end: f32,
    before: NodeId,
    after: NodeId,
}

/// Axis-partitioned, position-sorted boundary index for fast hit-testing.
///
/// Segments are stored contiguously: all vertical segments first (sorted by
/// position), then all horizontal segments (sorted by position).
/// Binary search narrows candidates to the tolerance window before span
/// filtering, replacing the previous linear scan.
///
/// Owns the scratch buffer's `Vec` between resolves — `reclaim()` returns
/// it so the next frame reuses the allocation instead of re-growing.
#[derive(Debug)]
struct BoundaryIndex {
    segments: Vec<BoundarySegment>,
    /// Number of vertical segments; horizontal segments start at this offset.
    vertical_count: usize,
}

impl BoundaryIndex {
    /// Sort the scratch buffer in-place and take ownership of it.
    fn from_scratch(buf: &mut Vec<BoundarySegment>) -> Self {
        let mut segments = std::mem::take(buf);
        segments.sort_by(|a, b| {
            a.axis
                .cmp_order()
                .cmp(&b.axis.cmp_order())
                .then(a.position.total_cmp(&b.position))
        });
        let vertical_count = segments
            .iter()
            .position(|s| s.axis == BoundaryAxis::Horizontal)
            .unwrap_or(segments.len());
        BoundaryIndex {
            segments,
            vertical_count,
        }
    }

    fn empty() -> Self {
        BoundaryIndex {
            segments: Vec::new(),
            vertical_count: 0,
        }
    }

    /// Return the inner vec so the scratch buffer can reuse its allocation.
    fn reclaim(self) -> Vec<BoundarySegment> {
        self.segments
    }

    fn vertical(&self) -> &[BoundarySegment] {
        &self.segments[..self.vertical_count]
    }

    fn horizontal(&self) -> &[BoundarySegment] {
        &self.segments[self.vertical_count..]
    }
}

impl BoundaryAxis {
    /// Ordering key: Vertical < Horizontal (for partition sort).
    fn cmp_order(self) -> u8 {
        match self {
            BoundaryAxis::Vertical => 0,
            BoundaryAxis::Horizontal => 1,
        }
    }
}

/// Binary-search a position-sorted segment slice for the nearest hit within tolerance.
///
/// `query_pos` is the coordinate perpendicular to the boundary (x for vertical, y for horizontal).
/// `query_span` is the coordinate along the boundary (y for vertical, x for horizontal).
fn search_sorted_segments(
    segments: &[BoundarySegment],
    query_pos: f32,
    query_span: f32,
    tolerance: f32,
) -> Option<(f32, &BoundarySegment)> {
    let lo = segments.partition_point(|s| s.position < query_pos - tolerance);
    let hi = segments.partition_point(|s| s.position <= query_pos + tolerance);

    segments[lo..hi]
        .iter()
        .filter(|s| query_span >= s.span_start && query_span < s.span_end)
        .map(|s| ((query_pos - s.position).abs(), s))
        .min_by(|(a, _), (b, _)| a.total_cmp(b))
}

/// A single panel's identity and computed rectangle.
///
/// Generic over `R` so the core crate yields `PanelEntry<'_, &Rect>` while
/// output crates yield their own rect type (e.g. `ratatui::Rect`, `egui::Rect`).
pub struct PanelEntry<'a, R> {
    /// The panel's unique identifier.
    pub id: PanelId,
    /// The panel's kind string (e.g. `"editor"`, `"terminal"`).
    pub kind: &'a str,
    /// The computed rectangle, generic over the coordinate type.
    pub rect: R,
    /// Index into [`ResolvedLayout::sorted_kind_keys`] for this panel's kind.
    pub kind_index: usize,
}

impl<'a, R> PanelEntry<'a, R> {
    /// Transform the rect type, preserving identity and kind metadata.
    pub fn map_rect<R2>(self, f: impl FnOnce(R) -> R2) -> PanelEntry<'a, R2> {
        PanelEntry {
            id: self.id,
            kind: self.kind,
            rect: f(self.rect),
            kind_index: self.kind_index,
        }
    }
}

/// A decoration panel's identity and relationship to a content panel.
pub struct DecorationPanelInfo {
    /// The decoration panel's id.
    pub id: PanelId,
    /// The role this decoration plays (tab chrome, title chrome).
    pub role: DecorationRole,
    /// The kind of the content panel this decoration belongs to.
    pub content_kind: Arc<str>,
}

/// Shared index mapping panel kind strings to their panel IDs.
pub(crate) type KindIndex = Arc<FxHashMap<Arc<str>, Box<[PanelId]>>>;

/// Shared index of decoration panel metadata.
pub(crate) type DecorationIndex = Arc<[DecorationPanelInfo]>;

/// Reverse index mapping panel id slots to decoration roles.
pub(crate) type DecorationRoleIndex = Arc<[Option<DecorationRole>]>;

fn sorted_keys(kinds: &KindIndex) -> Arc<[Arc<str>]> {
    let mut keys: Vec<_> = kinds.keys().map(Arc::clone).collect();
    keys.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
    keys.into()
}

/// Build a reverse index from PanelId → sorted kind index.
///
/// Iterates `kinds` (kind→panel_ids map) and binary-searches `sorted_kind_keys`
/// for each kind to assign the sorted index to every panel in that group.
fn build_panel_kind_indices(
    capacity: usize,
    kinds: &KindIndex,
    sorted_kind_keys: &[Arc<str>],
) -> Arc<[Option<u16>]> {
    let mut buf = vec![None; capacity];
    for (kind, pids) in kinds.iter() {
        let Ok(idx) = sorted_kind_keys.binary_search_by(|k| k.as_ref().cmp(kind.as_ref())) else {
            continue;
        };
        let idx16 = idx as u16;
        for &pid in pids.iter() {
            let slot = pid.raw() as usize;
            if slot < buf.len() {
                buf[slot] = Some(idx16);
            }
        }
    }
    buf.into()
}

/// Build a reverse index from PanelId → decoration role.
fn build_decoration_role_index(
    capacity: usize,
    decorations: &[DecorationPanelInfo],
) -> DecorationRoleIndex {
    let mut buf = vec![None; capacity];
    for decoration in decorations {
        let slot = decoration.id.raw() as usize;
        if slot < buf.len() {
            buf[slot] = Some(decoration.role);
        }
    }
    buf.into()
}

/// Output of layout resolution: a map from [`PanelId`] to [`Rect`] with
/// indexed lookups by kind, hit-testing, overlay tracking, and interpolation.
///
/// Produced by [`resolve`] (one-shot) or by `LayoutRuntime::resolve()` (cached,
/// incremental). Immutable after construction except for animation helpers.
pub struct ResolvedLayout {
    rects: Vec<Option<Rect>>,
    live_panel_ids: Arc<[PanelId]>,
    kinds: KindIndex,
    sorted_kind_keys: Arc<[Arc<str>]>,
    /// Reverse index: `panel_kind_indices[pid.raw()]` is the sorted kind index for that panel.
    panel_kind_indices: Arc<[Option<u16>]>,
    overlay_rects: Vec<(OverlayId, Arc<str>, Rect)>,
    overlay_failures: Vec<(OverlayId, Arc<str>, AnchorFailure)>,
    boundaries: BoundaryIndex,
    decorations: DecorationIndex,
    decoration_roles: DecorationRoleIndex,
}

impl ResolvedLayout {
    /// Returns the computed rectangle for `id`, or `None` if the panel is not
    /// present in this layout. O(1).
    pub fn get(&self, id: PanelId) -> Option<&Rect> {
        self.rects.get(id.raw() as usize)?.as_ref()
    }

    /// Returns all [`PanelId`]s that share the given kind string.
    /// Returns an empty slice when no panels of that kind exist.
    pub fn by_kind(&self, kind: &str) -> &[PanelId] {
        match self.kinds.get(kind) {
            Some(ids) => ids,
            None => &[],
        }
    }

    /// Iterates all live `(PanelId, &Rect)` pairs in allocation order.
    /// See [`panels`](Self::panels) for kind-grouped iteration with metadata.
    pub fn iter(&self) -> impl Iterator<Item = (PanelId, &Rect)> {
        self.live_panel_ids.iter().filter_map(|&pid| {
            self.rects
                .get(pid.raw() as usize)?
                .as_ref()
                .map(|r| (pid, r))
        })
    }

    /// Iterates all live [`PanelId`]s in allocation order.
    pub fn panel_ids(&self) -> impl Iterator<Item = PanelId> + '_ {
        self.live_panel_ids.iter().copied()
    }

    /// Returns the number of live panels in this layout.
    pub fn panel_count(&self) -> usize {
        self.live_panel_ids.len()
    }

    /// Iterates distinct kind strings present in this layout (unordered).
    /// See [`sorted_kind_keys`](Self::sorted_kind_keys) for lexicographic order.
    pub fn kinds(&self) -> impl Iterator<Item = &str> {
        self.kinds.keys().map(|k| k.as_ref())
    }

    /// Sorted kind keys in the same order used by [`panels()`](Self::panels).
    ///
    /// Index position matches the `kind_index` yielded by `panels()`.
    pub fn sorted_kind_keys(&self) -> &[Arc<str>] {
        &self.sorted_kind_keys
    }

    /// Returns the kind string for `pid`, or `None` if the panel is not in this
    /// layout. O(1) via the reverse panel-kind index.
    pub fn kind_of(&self, pid: PanelId) -> Option<&str> {
        let idx = *self.panel_kind_indices.get(pid.raw() as usize)?.as_ref()?;
        self.sorted_kind_keys.get(idx as usize).map(|k| k.as_ref())
    }

    /// Returns the index into [`sorted_kind_keys`](Self::sorted_kind_keys) for
    /// the given panel, or `None` if `pid` is not in this layout. O(1).
    pub fn kind_index_of_panel(&self, pid: PanelId) -> Option<usize> {
        self.panel_kind_indices
            .get(pid.raw() as usize)?
            .map(|idx| idx as usize)
    }

    /// Returns the index into [`sorted_kind_keys`](Self::sorted_kind_keys) for
    /// the given kind string, or `None` if the kind is absent. O(log k) where
    /// k is the number of distinct kinds (binary search).
    pub fn kind_index_of(&self, kind: &str) -> Option<usize> {
        self.sorted_kind_keys
            .binary_search_by(|k| k.as_ref().cmp(kind))
            .ok()
    }

    /// Iterates decoration panels whose `content_kind` matches `kind`, paired
    /// with that kind's index in [`sorted_kind_keys`](Self::sorted_kind_keys).
    /// Yields nothing when the kind is absent or has no decorations.
    pub fn decoration_entries(
        &self,
        kind: &str,
    ) -> impl Iterator<Item = (&DecorationPanelInfo, usize)> {
        let kind_index = self.kind_index_of(kind);
        self.decorations
            .iter()
            .filter(move |d| d.content_kind.as_ref() == kind)
            .filter_map(move |d| Some((d, kind_index?)))
    }

    /// Shifts all panel rects horizontally by `dx`. Used for slide animations.
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

    /// Iterates resolved overlay rects in z-order (back to front).
    pub fn overlays(&self) -> impl Iterator<Item = OverlayEntry<'_, &Rect>> {
        self.overlay_rects
            .iter()
            .map(|(id, kind, rect)| OverlayEntry {
                id: *id,
                kind: kind.as_ref(),
                rect,
            })
    }

    /// Decoration metadata for all decoration panels in this layout.
    pub fn decoration_panels(&self) -> &[DecorationPanelInfo] {
        &self.decorations
    }

    /// Returns the [`DecorationRole`] for `pid`, or `None` if it is not a
    /// decoration panel. O(1) via the reverse decoration-role index.
    pub fn decoration_role(&self, pid: PanelId) -> Option<DecorationRole> {
        self.decoration_roles
            .get(pid.raw() as usize)
            .copied()
            .flatten()
    }

    /// Hit-tests panels in reverse allocation order and returns the first
    /// [`PanelId`] whose rect contains `(x, y)`, or `None` if no panel matches.
    pub fn panel_at_point(&self, x: f32, y: f32) -> Option<PanelId> {
        self.live_panel_ids
            .iter()
            .rev()
            .copied()
            .find(|&pid| self.get(pid).is_some_and(|rect| rect.contains(x, y)))
    }

    /// Hit-tests overlays in reverse z-order and returns the first
    /// [`OverlayId`] whose rect contains `(x, y)`, or `None` if no overlay matches.
    pub fn overlay_at_point(&self, x: f32, y: f32) -> Option<OverlayId> {
        self.overlay_rects
            .iter()
            .rev()
            .find(|(_, _, rect)| rect.contains(x, y))
            .map(|(id, _, _)| *id)
    }

    /// Returns the computed rectangle for the given overlay, or `None` if the
    /// overlay is not present in this layout. O(n) linear scan.
    pub fn overlay_rect(&self, id: OverlayId) -> Option<&Rect> {
        self.overlay_rects
            .iter()
            .find(|(oid, _, _)| *oid == id)
            .map(|(_, _, r)| r)
    }

    /// Return the resize boundary closest to the given point within tolerance.
    ///
    /// Uses an axis-partitioned, position-sorted index with binary search
    /// to narrow candidates before span filtering. Returns the nearest
    /// boundary whose main-axis distance is within `tolerance` and whose
    /// perpendicular span covers the query point.
    pub fn boundary_at_point(&self, x: f32, y: f32, tolerance: f32) -> Option<BoundaryHit> {
        // Search vertical boundaries (keyed by x, span-checked by y)
        let v_hit = search_sorted_segments(self.boundaries.vertical(), x, y, tolerance);
        // Search horizontal boundaries (keyed by y, span-checked by x)
        let h_hit = search_sorted_segments(self.boundaries.horizontal(), y, x, tolerance);

        let best = match (v_hit, h_hit) {
            (Some(v), Some(h)) if v.0 <= h.0 => Some(v),
            (Some(_), Some(h)) => Some(h),
            (Some(v), None) => Some(v),
            (None, h) => h,
        };

        best.map(|(_, seg)| BoundaryHit {
            axis: seg.axis,
            sides: (seg.before, seg.after),
            position: seg.position,
        })
    }

    /// Overlays that failed to anchor during the most recent resolve.
    pub fn overlay_failures(&self) -> &[(OverlayId, Arc<str>, AnchorFailure)] {
        &self.overlay_failures
    }

    pub(crate) fn overlay_rects_raw(&self) -> &[(OverlayId, Arc<str>, Rect)] {
        &self.overlay_rects
    }

    pub(crate) fn swap_overlay_rects(&mut self, buf: &mut Vec<(OverlayId, Arc<str>, Rect)>) {
        std::mem::swap(&mut self.overlay_rects, buf);
    }

    pub(crate) fn swap_overlay_failures(
        &mut self,
        buf: &mut Vec<(OverlayId, Arc<str>, AnchorFailure)>,
    ) {
        std::mem::swap(&mut self.overlay_failures, buf);
    }

    pub(crate) fn kinds_arc(&self) -> &KindIndex {
        &self.kinds
    }

    pub(crate) fn sorted_kind_keys_arc(&self) -> &Arc<[Arc<str>]> {
        &self.sorted_kind_keys
    }

    pub(crate) fn decorations_arc(&self) -> &DecorationIndex {
        &self.decorations
    }

    pub(crate) fn decoration_roles_arc(&self) -> &DecorationRoleIndex {
        &self.decoration_roles
    }

    pub(crate) fn live_panel_ids_arc(&self) -> &Arc<[PanelId]> {
        &self.live_panel_ids
    }

    pub(crate) fn panel_kind_indices_arc(&self) -> &Arc<[Option<u16>]> {
        &self.panel_kind_indices
    }

    /// Takes ownership of the panel rect buffer, leaving it empty.
    /// Used to reclaim the allocation for the next resolve cycle.
    pub fn take_rects(&mut self) -> Vec<Option<Rect>> {
        std::mem::take(&mut self.rects)
    }

    /// Takes ownership of the overlay rect buffer, leaving it empty.
    /// Used to reclaim the allocation for the next resolve cycle.
    pub fn take_overlay_rects(&mut self) -> Vec<(OverlayId, Arc<str>, Rect)> {
        std::mem::take(&mut self.overlay_rects)
    }

    pub(crate) fn take_overlay_failures(&mut self) -> Vec<(OverlayId, Arc<str>, AnchorFailure)> {
        std::mem::take(&mut self.overlay_failures)
    }

    /// Return the boundary buffer so the scratch can reuse its allocation.
    pub(crate) fn take_boundaries(&mut self) -> Vec<BoundarySegment> {
        std::mem::replace(&mut self.boundaries, BoundaryIndex::empty()).reclaim()
    }

    /// Interpolates between `self` and `other` at parameter `t` (0.0 = self,
    /// 1.0 = other). Allocates a fresh rect buffer. See [`lerp_into`](Self::lerp_into)
    /// to reuse an existing buffer.
    pub fn lerp(&self, other: &ResolvedLayout, t: f32) -> ResolvedLayout {
        let mut buf = Vec::new();
        self.lerp_into(other, t, &mut buf)
    }

    /// Interpolates between `self` and `other` at parameter `t`, reusing `buf`
    /// for the output rect storage to avoid allocation. Panels present in `self`
    /// but absent in `other` interpolate toward their current position.
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
        let panel_kind_indices = self.panel_kind_indices.clone();
        let live_panel_ids = self.live_panel_ids.clone();
        let decorations = Arc::clone(&self.decorations);
        let decoration_roles = Arc::clone(&self.decoration_roles);
        ResolvedLayout {
            rects,
            live_panel_ids,
            kinds,
            sorted_kind_keys,
            panel_kind_indices,
            overlay_rects: Vec::new(),
            overlay_failures: Vec::new(),
            boundaries: BoundaryIndex::empty(),
            decorations,
            decoration_roles,
        }
    }
}

fn set_panel_rect(
    rects: &mut [Option<Rect>],
    id: PanelId,
    x: f32,
    y: f32,
    size: &taffy::Size<f32>,
) -> Result<(), PaneError> {
    *rects
        .get_mut(id.raw() as usize)
        .ok_or(PaneError::PanelNotFound(id))? = Some(Rect {
        x,
        y,
        w: size.width,
        h: size.height,
    });
    Ok(())
}

/// Iterative DFS that populates rects (and optionally kinds). Reuses the stack across frames.
fn resolve_iterative(
    tree: &LayoutTree,
    result: &CompileResult,
    root_id: NodeId,
    rects: &mut [Option<Rect>],
    scratch: &mut ResolveScratch,
    collect_kinds: bool,
) -> Result<(), PaneError> {
    scratch.stack.clear();
    scratch.boundary_buf.clear();
    scratch.live_panel_buf.clear();
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
            Some(Node::Panel { id, kind, .. }) if collect_kinds && !tree.is_decoration(*id) => {
                set_panel_rect(rects, *id, abs_x, abs_y, &layout.size)?;
                scratch.live_panel_buf.push(*id);
                scratch
                    .kinds_buf
                    .entry(Arc::clone(kind))
                    .or_default()
                    .push(*id);
            }
            Some(Node::Panel { id, .. }) if collect_kinds => {
                set_panel_rect(rects, *id, abs_x, abs_y, &layout.size)?;
                scratch.live_panel_buf.push(*id);
            }
            Some(Node::Panel { id, .. }) => {
                set_panel_rect(rects, *id, abs_x, abs_y, &layout.size)?;
                scratch.live_panel_buf.push(*id);
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
            Some(Node::Grid { children, .. }) => {
                for &child_id in children.iter().rev() {
                    scratch.stack.push((child_id, abs_x, abs_y));
                }
            }
            Some(Node::GridItemWrapper { child, .. }) => {
                scratch.stack.push((*child, abs_x, abs_y));
            }
            Some(Node::TaffyPassthrough { children, .. }) => {
                for &child_id in children.iter().rev() {
                    scratch.stack.push((child_id, abs_x, abs_y));
                }
            }
            None => return Err(PaneError::NodeNotFound(node_id)),
        }
    }
    Ok(())
}

/// Reusable scratch state for DFS resolution.
pub(crate) struct ResolveScratch {
    stack: Vec<(NodeId, f32, f32)>,
    kinds_buf: FxHashMap<Arc<str>, Vec<PanelId>>,
    boundary_buf: Vec<BoundarySegment>,
    live_panel_buf: Vec<PanelId>,
    /// When false, skip boundary collection during resolve.
    pub(crate) collect_boundaries: bool,
}

impl Default for ResolveScratch {
    fn default() -> Self {
        Self {
            stack: Vec::new(),
            kinds_buf: FxHashMap::default(),
            boundary_buf: Vec::new(),
            live_panel_buf: Vec::new(),
            collect_boundaries: true,
        }
    }
}

impl ResolveScratch {
    /// Donate a boundary buffer so the next frame reuses its allocation.
    pub(crate) fn donate_boundary_buf(&mut self, buf: Vec<BoundarySegment>) {
        match self.boundary_buf.capacity() {
            0 => self.boundary_buf = buf,
            _ => {}
        }
    }
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

    let mut adjacent = children.iter().copied();
    let Some(first_child_id) = adjacent.next() else {
        return;
    };
    let mut previous = child_layout(result, first_child_id).map(|layout| (first_child_id, layout));

    for child_id in adjacent {
        let current = child_layout(result, child_id).map(|layout| (child_id, layout));
        let (Some((a_id, a_layout)), Some((b_id, b_layout))) = (previous, current) else {
            previous = current;
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

        previous = Some((b_id, b_layout));
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
            buf.fill(None);
            buf.resize(capacity, None);
            buf
        }
        None => vec![None; capacity],
    }
}

/// Build a `DecorationIndex` from the tree's decoration metadata.
fn collect_decorations(tree: &LayoutTree) -> DecorationIndex {
    tree.decoration_entries()
        .map(|(pid, meta)| DecorationPanelInfo {
            id: pid,
            role: meta.role,
            content_kind: Arc::clone(&meta.content_kind),
        })
        .collect()
}

/// Cached layout indices passed from the runtime when topology is unchanged.
pub(crate) struct CachedLayoutState {
    pub kinds: KindIndex,
    pub sorted_kind_keys: Arc<[Arc<str>]>,
    pub panel_kind_indices: Arc<[Option<u16>]>,
    pub decorations: DecorationIndex,
    pub decoration_roles: DecorationRoleIndex,
    pub live_panel_ids: Arc<[PanelId]>,
}

/// Resolve rects using a previously cached kinds index. Skips kinds population.
/// Uses iterative DFS with a reusable stack.
pub(crate) fn resolve_with_cached_kinds(
    result: &CompileResult,
    tree: &LayoutTree,
    cached: CachedLayoutState,
    scratch: &mut ResolveScratch,
    rects_buf: Option<Vec<Option<Rect>>>,
) -> Result<ResolvedLayout, PaneError> {
    let root_id = tree
        .root()
        .ok_or(PaneError::InvalidTree(TreeError::RootNotSet))?;

    let capacity = tree.panel_id_high_water() as usize;
    let mut rects = prepare_rects_buf(rects_buf, capacity);
    resolve_iterative(tree, result, root_id, &mut rects, scratch, false)?;

    let boundaries = match scratch.collect_boundaries {
        true => BoundaryIndex::from_scratch(&mut scratch.boundary_buf),
        false => BoundaryIndex::empty(),
    };
    Ok(ResolvedLayout {
        rects,
        live_panel_ids: cached.live_panel_ids,
        kinds: cached.kinds,
        sorted_kind_keys: cached.sorted_kind_keys,
        panel_kind_indices: cached.panel_kind_indices,
        overlay_rects: Vec::new(),
        overlay_failures: Vec::new(),
        boundaries,
        decorations: cached.decorations,
        decoration_roles: cached.decoration_roles,
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

    resolve_iterative(tree, result, root_id, &mut rects, scratch, true)?;

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
        true => BoundaryIndex::from_scratch(&mut scratch.boundary_buf),
        false => BoundaryIndex::empty(),
    };
    let sorted_kind_keys = sorted_keys(&kinds);
    let decorations = collect_decorations(tree);
    let decoration_roles = build_decoration_role_index(capacity, &decorations);
    let live_panel_ids = scratch.live_panel_buf.as_slice().into();
    let panel_kind_indices = build_panel_kind_indices(capacity, &kinds, &sorted_kind_keys);
    Ok(ResolvedLayout {
        rects,
        live_panel_ids,
        kinds,
        sorted_kind_keys,
        panel_kind_indices,
        overlay_rects: Vec::new(),
        overlay_failures: Vec::new(),
        boundaries,
        decorations,
        decoration_roles,
    })
}
