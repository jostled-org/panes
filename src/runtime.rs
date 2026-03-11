use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::compiler::{CompileResult, compile, compute_layout};
use crate::diff::{self, LayoutDiff, OverlayDiff, OverlayDiffScratch};
use crate::error::{MutationError, PaneError, ViewportError};
use crate::focus::{self, FocusDirection};
use crate::layout::Layout;
use crate::node::{Node, NodeId, PanelId};
use crate::overlay::{self, Overlay, OverlayDef, OverlayId, OverlayIdGenerator};
use crate::panel::fixed;
use crate::rect::Rect;
use crate::resolver::{self, ResolveScratch, ResolvedLayout};
use crate::sequence::PanelSequence;
use crate::snapshot::{self, LayoutSnapshot, SnapshotSource};
use crate::strategy::{Direction, StrategyKind};
use crate::tree::LayoutTree;
use crate::validate::{check_f32_finite, check_f32_non_negative, float_invalid_to_constraint};
use crate::viewport::ViewportState;

/// Where to place the new panel relative to the focused panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    /// New panel goes before focused (left or above).
    Before,
    /// New panel goes after focused (right or below).
    #[default]
    After,
    /// Append to the end of the sequence.
    End,
}

/// Result of a single resolve call: the resolved layout for this frame.
///
/// To access the diff between this frame and the previous one, call
/// [`LayoutRuntime::last_diff()`] or [`LayoutRuntime::last_overlay_diff()`]
/// after `resolve()`.
pub struct Frame {
    layout: Arc<ResolvedLayout>,
}

impl Frame {
    /// The resolved layout for this frame.
    pub fn layout(&self) -> &ResolvedLayout {
        &self.layout
    }
}

/// Stateful layout wrapper that tracks tree, viewport, and frame history.
pub struct LayoutRuntime {
    tree: LayoutTree,
    viewport: ViewportState,
    previous: Option<Arc<ResolvedLayout>>,
    cached_compile: Option<CompileResult>,
    cached_kinds: Option<resolver::KindIndex>,
    rects_buf: Option<Vec<Option<Rect>>>,
    diff_scratch: diff::DiffScratch,
    overlay_diff_scratch: OverlayDiffScratch,
    resolve_scratch: ResolveScratch,
    strategy: Option<StrategyKind>,
    sequence: PanelSequence,
    overlays: Vec<OverlayDef>,
    overlay_gen: OverlayIdGenerator,
    overlay_index: FxHashMap<Arc<str>, usize>,
    prev_overlay_rects: Vec<(OverlayId, Rect)>,
    overlay_rects_buf: Vec<(OverlayId, Arc<str>, Rect)>,
}

/// Shared default fields for all constructors.
fn base(
    tree: LayoutTree,
    viewport: ViewportState,
    strategy: Option<StrategyKind>,
    sequence: PanelSequence,
) -> LayoutRuntime {
    LayoutRuntime {
        tree,
        viewport,
        previous: None,
        cached_compile: None,
        cached_kinds: None,
        rects_buf: None,
        diff_scratch: diff::DiffScratch::default(),
        overlay_diff_scratch: OverlayDiffScratch::default(),
        resolve_scratch: ResolveScratch::default(),
        strategy,
        sequence,
        overlays: Vec::new(),
        overlay_gen: OverlayIdGenerator::default(),
        overlay_index: FxHashMap::default(),
        prev_overlay_rects: Vec::new(),
        overlay_rects_buf: Vec::new(),
    }
}

impl LayoutRuntime {
    /// Create a runtime from an existing tree (legacy path, no strategy).
    pub fn new(tree: LayoutTree) -> Self {
        base(
            tree,
            ViewportState::default(),
            None,
            PanelSequence::default(),
        )
    }

    /// Create a runtime from a strategy and initial panel kinds.
    pub fn from_strategy(strategy: StrategyKind, kinds: &[Arc<str>]) -> Result<Self, PaneError> {
        let mut sequence = PanelSequence::default();
        let mut viewport = ViewportState::default();
        let tree = crate::strategy::build_initial(&strategy, kinds, &mut sequence, &mut viewport)?;
        Ok(base(tree, viewport, Some(strategy), sequence))
    }

    /// Create a runtime from a pre-built tree with a pre-populated sequence.
    /// No strategy — for direct tree-topology control with sequence tracking.
    pub fn from_tree_and_sequence(tree: LayoutTree, sequence: PanelSequence) -> Self {
        let focus = sequence.get(0);
        base(
            tree,
            ViewportState {
                focus,
                ..ViewportState::default()
            },
            None,
            sequence,
        )
    }

    /// Create a runtime from a pre-built tree and a strategy.
    /// Populates the sequence by looking up each kind in the tree.
    pub fn from_tree_and_strategy(
        tree: LayoutTree,
        strategy: StrategyKind,
        kinds: &[Arc<str>],
    ) -> Self {
        let mut sequence = PanelSequence::default();
        for kind in kinds {
            for &pid in tree.panels_by_kind(kind) {
                sequence.push(pid);
            }
        }
        let focus = sequence.get(0);
        base(
            tree,
            ViewportState {
                focus,
                ..ViewportState::default()
            },
            Some(strategy),
            sequence,
        )
    }

    /// Immutable access to the underlying tree.
    pub fn tree(&self) -> &LayoutTree {
        &self.tree
    }

    /// Mutable access to the underlying tree for structural mutations.
    pub fn tree_mut(&mut self) -> &mut LayoutTree {
        &mut self.tree
    }

    /// Immutable access to the viewport state.
    pub fn viewport(&self) -> &ViewportState {
        &self.viewport
    }

    /// Toggle a panel's collapsed state.
    ///
    /// Collapsing saves the current constraints and sets the panel to fixed(0.0).
    /// Uncollapsing restores the saved constraints.
    pub fn toggle_collapsed(&mut self, pid: PanelId) -> Result<(), PaneError> {
        match self.viewport.collapsed.contains(&pid) {
            true => {
                let saved = self.viewport.saved_constraints.remove(&pid).ok_or(
                    PaneError::InvalidViewport(ViewportError::NoSavedConstraints(pid)),
                )?;
                self.tree.set_constraints(pid, saved)?;
                self.viewport.collapsed.remove(&pid);
                Ok(())
            }
            false => {
                let current = self.tree.panel_constraints(pid)?;
                self.viewport.saved_constraints.insert(pid, current);
                self.tree.set_constraints(pid, fixed(0.0))?;
                self.viewport.collapsed.insert(pid);
                Ok(())
            }
        }
    }

    /// Shift the scroll offset by a delta.
    pub fn scroll_by(&mut self, delta: f32) -> Result<(), PaneError> {
        check_f32_finite(delta)
            .map_err(|_| PaneError::InvalidViewport(ViewportError::ScrollNotFinite))?;
        self.viewport.scroll_offset += delta;
        Ok(())
    }

    /// Set the scroll offset to an absolute value.
    pub fn scroll_to(&mut self, offset: f32) -> Result<(), PaneError> {
        check_f32_finite(offset)
            .map_err(|_| PaneError::InvalidViewport(ViewportError::ScrollNotFinite))?;
        self.viewport.scroll_offset = offset;
        Ok(())
    }

    /// Set focus to a panel without strategy validation.
    ///
    /// Unlike [`focus`](Self::focus), this bypasses strategy-specific focus
    /// logic (e.g. updating tab visibility in `ActivePanel` layouts).
    /// Use when you need raw focus control outside the strategy system.
    pub fn set_focus_unchecked(&mut self, pid: PanelId) {
        self.viewport.focus = Some(pid);
    }

    /// The layout strategy, if this runtime was created via `from_strategy`.
    pub fn strategy(&self) -> Option<&StrategyKind> {
        self.strategy.as_ref()
    }

    /// The panel sequence (logical order).
    pub fn sequence(&self) -> &PanelSequence {
        &self.sequence
    }

    /// The currently focused panel.
    pub fn focused(&self) -> Option<PanelId> {
        self.viewport.focus
    }

    /// The kind of the currently focused panel.
    pub fn focused_kind(&self) -> Option<&str> {
        let pid = self.viewport.focus?;
        self.tree.panel_kind(pid).ok()
    }

    /// Capture a serializable snapshot of the current runtime state.
    ///
    /// Strategy runtimes snapshot the recipe (strategy config + panel kinds).
    /// Non-strategy runtimes snapshot the tree topology.
    ///
    /// `TaffyPassthrough` nodes are not serializable and are omitted from tree
    /// snapshots. Returns `SnapshotNoRoot` if the root itself is a passthrough.
    pub fn snapshot(&self) -> Result<LayoutSnapshot, PaneError> {
        snapshot::capture(
            &self.tree,
            self.strategy.as_ref(),
            &self.sequence,
            &self.viewport,
            &self.overlays,
        )
    }

    /// Restore a runtime from a snapshot.
    ///
    /// Strategy snapshots rebuild through the preset builder.
    /// Tree snapshots rebuild via the layout builder.
    pub fn from_snapshot(snap: LayoutSnapshot) -> Result<Self, PaneError> {
        let mut rt = match snap.source() {
            SnapshotSource::Strategy { strategy, panels } => {
                let sk = StrategyKind::from(strategy);
                let kinds: Vec<Arc<str>> = panels.iter().map(|s| Arc::from(&**s)).collect();
                Self::from_strategy(sk, &kinds)?
            }
            SnapshotSource::Tree { root } => {
                let tree = snapshot::snapshot_to_tree(root)?;
                let mut seq = PanelSequence::default();
                collect_panels_depth_first(&tree, &mut seq);
                Self::from_tree_and_sequence(tree, seq)
            }
        };

        // Restore focus by kind
        if let Some(&pid) = snap
            .focused()
            .and_then(|kind| rt.tree.panels_by_kind(kind).first())
        {
            rt.focus(pid);
        }

        // Restore collapsed by kind
        for kind in snap.collapsed() {
            if let Some(&pid) = rt.tree.panels_by_kind(kind).first() {
                rt.toggle_collapsed(pid)?;
            }
        }

        restore_overlays(&mut rt, snap.overlays())?;

        Ok(rt)
    }

    /// Whether `pid` is a decorative panel (tab bar, title bar) for `content_pid`.
    pub fn is_decoration_for(&self, pid: PanelId, content_pid: PanelId) -> bool {
        let (Ok(dec_kind), Ok(content_kind)) =
            (self.tree.panel_kind(pid), self.tree.panel_kind(content_pid))
        else {
            return false;
        };
        let base = dec_kind
            .strip_suffix("_tab")
            .or_else(|| dec_kind.strip_suffix("_title"));
        matches!(base, Some(b) if b == content_kind)
    }

    /// Add a panel using the active strategy.
    ///
    /// With a strategy: inserts after focused (or at end if no focus),
    /// then rebuilds the tree through the strategy's preset builder.
    ///
    /// Without a strategy: hyprland-style split (auto-direction from
    /// aspect ratio, `grow(1.0)`, placed after focused).
    pub fn add_panel(&mut self, kind: Arc<str>) -> Result<PanelId, PaneError> {
        self.add_panel_with(kind, Placement::After)
    }

    /// Add a panel at an explicit position relative to focused.
    ///
    /// - `Placement::Before` — before focused
    /// - `Placement::After` — after focused (same as `add_panel`)
    /// - `Placement::End` — append to sequence end
    pub fn add_panel_with(
        &mut self,
        kind: Arc<str>,
        placement: Placement,
    ) -> Result<PanelId, PaneError> {
        match self.strategy.as_ref() {
            Some(strategy) => {
                let index = self.placement_to_index(placement);
                crate::strategy::apply_add(
                    strategy,
                    &mut self.tree,
                    &mut self.sequence,
                    &mut self.viewport,
                    kind,
                    index,
                )
            }
            None => {
                let direction = self.auto_direction();
                self.add_panel_adjacent_with(kind, direction, crate::panel::grow(1.0), placement)
            }
        }
    }

    /// Convert a Placement to a sequence index.
    fn placement_to_index(&self, placement: Placement) -> usize {
        match placement {
            Placement::Before => self
                .viewport
                .focus
                .and_then(|pid| self.sequence.index_of(pid))
                .unwrap_or(self.sequence.len()),
            Placement::After => self
                .viewport
                .focus
                .and_then(|pid| self.sequence.index_of(pid).map(|i| i + 1))
                .unwrap_or(self.sequence.len()),
            Placement::End => self.sequence.len(),
        }
    }

    /// Remove a panel using the active strategy. Returns the new focus panel.
    pub fn remove_panel(&mut self, pid: PanelId) -> Result<Option<PanelId>, PaneError> {
        let strategy = self
            .strategy
            .as_ref()
            .ok_or(PaneError::InvalidMutation(MutationError::NoStrategy))?;
        crate::strategy::apply_remove(
            strategy,
            &mut self.tree,
            &mut self.sequence,
            &mut self.viewport,
            pid,
        )
    }

    /// Move a panel to a new sequence index using the active strategy.
    pub fn move_panel(&mut self, pid: PanelId, new_index: usize) -> Result<PanelId, PaneError> {
        let strategy = self
            .strategy
            .as_ref()
            .ok_or(PaneError::InvalidMutation(MutationError::NoStrategy))?;
        crate::strategy::apply_move(
            strategy,
            &mut self.tree,
            &mut self.sequence,
            &mut self.viewport,
            pid,
            new_index,
        )
    }

    /// Set focus to a specific panel.
    ///
    /// Returns `true` if focus was set, `false` if `pid` is not in the
    /// sequence (strategy path) or not a known panel.
    pub fn focus(&mut self, pid: PanelId) -> bool {
        let Some(strategy) = self.strategy.as_ref() else {
            self.set_focus_unchecked(pid);
            return true;
        };
        crate::strategy::try_apply_focus(
            strategy,
            &mut self.tree,
            &mut self.sequence,
            &mut self.viewport,
            pid,
        )
    }

    /// Swap the focused panel with the next panel in the sequence (wrapping).
    /// No-op if there is no focus or fewer than two panels.
    pub fn swap_next(&mut self) {
        self.swap_by(1);
    }

    /// Swap the focused panel with the previous panel in the sequence (wrapping).
    /// No-op if there is no focus or fewer than two panels.
    pub fn swap_prev(&mut self) {
        self.swap_by(-1);
    }

    fn swap_by(&mut self, delta: isize) {
        let (pid, idx) = match (
            self.viewport.focus,
            self.viewport.focus.and_then(|c| self.sequence.index_of(c)),
        ) {
            (Some(pid), Some(idx)) => (pid, idx),
            _ => return,
        };
        let len = self.sequence.len();
        match len <= 1 {
            true => {}
            false => {
                let target = ((idx as isize + delta).rem_euclid(len as isize)) as usize;
                // move_panel can only fail if: no strategy (impossible,
                // swap_by requires a strategy), OOB index (impossible,
                // rem_euclid guarantees bounds), or rebuild fails on empty
                // kinds (impossible, len > 1 checked above).
                let _ = self.move_panel(pid, target);
            }
        }
    }

    /// Move focus to the next panel in the sequence.
    /// No-op if the sequence is empty.
    pub fn focus_next(&mut self) {
        self.focus_by(1);
    }

    /// Move focus to the previous panel in the sequence.
    /// No-op if the sequence is empty.
    pub fn focus_prev(&mut self) {
        self.focus_by(-1);
    }

    fn focus_by(&mut self, delta: isize) {
        let target = match (
            self.viewport.focus,
            self.viewport.focus.and_then(|c| self.sequence.index_of(c)),
        ) {
            (Some(_), Some(idx)) => {
                let len = self.sequence.len().max(1);
                let next_idx = ((idx as isize + delta).rem_euclid(len as isize)) as usize;
                self.sequence.get(next_idx)
            }
            _ => self.sequence.get(0),
        };
        if let Some(pid) = target {
            self.focus(pid);
        }
    }

    /// Move focus to the nearest panel in a spatial direction.
    ///
    /// Returns `Some(target)` when focus moved, `None` when no candidate
    /// exists in that direction or no panel is focused.
    pub fn focus_direction(
        &mut self,
        layout: &ResolvedLayout,
        direction: FocusDirection,
    ) -> Option<PanelId> {
        let focused = self.focused()?;
        let target = focus::find_nearest(layout, focused, &self.sequence, direction)?;
        self.focus(target);
        Some(target)
    }

    /// Move focus to the nearest panel in a spatial direction, using the
    /// most recently resolved layout.
    ///
    /// Returns `Some(target)` when focus moved, `None` when no layout has
    /// been resolved, no panel is focused, or no candidate exists.
    pub fn focus_direction_current(&mut self, direction: FocusDirection) -> Option<PanelId> {
        let layout = Arc::clone(self.previous.as_ref()?);
        self.focus_direction(&layout, direction)
    }

    /// Pick a split direction from the focused panel's aspect ratio.
    /// Splits the longer axis: wider → horizontal, taller → vertical.
    /// Falls back to horizontal if no layout is cached or no panel is focused.
    fn auto_direction(&self) -> Direction {
        let rect = self
            .viewport
            .focus
            .and_then(|pid| self.previous.as_ref()?.get(pid));
        match rect {
            Some(r) if r.h > r.w => Direction::Vertical,
            _ => Direction::Horizontal,
        }
    }

    /// Add a panel adjacent to the currently focused panel with full control.
    ///
    /// With a strategy: delegates to strategy rebuild (direction/constraints
    /// ignored, placement controls sequence position).
    ///
    /// Without a strategy: works directly on tree topology. If `direction`
    /// matches the parent container's axis, the new panel is inserted as a
    /// sibling. If it conflicts, the focused panel is wrapped in a new
    /// sub-container.
    pub fn add_panel_adjacent_with(
        &mut self,
        kind: Arc<str>,
        direction: Direction,
        constraints: crate::Constraints,
        placement: Placement,
    ) -> Result<PanelId, PaneError> {
        match self.strategy.as_ref() {
            Some(strategy) => {
                let index = self.placement_to_index(placement);
                crate::strategy::apply_add(
                    strategy,
                    &mut self.tree,
                    &mut self.sequence,
                    &mut self.viewport,
                    kind,
                    index,
                )
            }
            None => self.add_panel_adjacent_no_strategy(kind, direction, constraints, placement),
        }
    }

    fn add_panel_adjacent_no_strategy(
        &mut self,
        kind: Arc<str>,
        direction: Direction,
        constraints: crate::Constraints,
        placement: Placement,
    ) -> Result<PanelId, PaneError> {
        let focused = self
            .focused()
            .ok_or(PaneError::InvalidMutation(MutationError::NoFocusedPanel))?;
        let (focused_nid, parent_id, focused_idx, parent_axis) =
            find_focused_position(&self.tree, focused)?;

        let (new_pid, new_nid) = self.tree.add_panel(kind, constraints)?;

        match (parent_axis == direction, placement) {
            (true, Placement::Before) => {
                self.tree.insert_child_at(parent_id, focused_idx, new_nid)?;
            }
            (true, Placement::After | Placement::End) => {
                self.tree
                    .insert_child_at(parent_id, focused_idx + 1, new_nid)?;
            }
            (false, Placement::End | Placement::After) => {
                wrap_in_container(
                    &mut self.tree,
                    parent_id,
                    focused_nid,
                    focused_idx,
                    new_nid,
                    direction,
                    Placement::After,
                )?;
            }
            (false, Placement::Before) => {
                wrap_in_container(
                    &mut self.tree,
                    parent_id,
                    focused_nid,
                    focused_idx,
                    new_nid,
                    direction,
                    Placement::Before,
                )?;
            }
        }

        let seq_idx = match (self.sequence.index_of(focused), placement) {
            (Some(idx), Placement::Before) => idx,
            (Some(idx), Placement::After) => idx + 1,
            (_, Placement::End) | (None, _) => self.sequence.len(),
        };
        self.sequence.insert(seq_idx, new_pid);

        self.viewport.focus = Some(new_pid);
        Ok(new_pid)
    }

    /// Resize a panel's share of its container by `delta` (fraction of container space).
    ///
    /// Positive delta gives the panel more space; negative gives it less.
    /// All siblings in the parent container must be panels with grow constraints.
    pub fn resize_boundary(&mut self, pid: PanelId, delta: f32) -> Result<(), PaneError> {
        crate::resize::resize_boundary(&mut self.tree, pid, delta)
    }

    /// Add an overlay. Returns the existing id if the kind already exists.
    pub fn add_overlay(
        &mut self,
        kind: impl Into<Arc<str>>,
        builder: Overlay,
    ) -> Result<OverlayId, PaneError> {
        crate::runtime_overlay::add_overlay_impl(
            &mut self.overlays,
            &mut self.overlay_index,
            &mut self.overlay_gen,
            kind.into(),
            builder,
        )
    }

    /// Remove an overlay by kind. No-op if the kind is not found.
    pub fn remove_overlay(&mut self, kind: &str) {
        crate::runtime_overlay::remove_overlay_impl(
            &mut self.overlays,
            &mut self.overlay_index,
            kind,
        );
    }

    /// Show or hide an overlay without removing it.
    pub fn set_overlay_visible(&mut self, kind: &str, visible: bool) {
        if let Some(&idx) = self.overlay_index.get(kind) {
            self.overlays[idx].visible = visible;
        }
    }

    /// Update an overlay's height (fixed value).
    pub fn set_overlay_height(&mut self, kind: &str, h: f32) -> Result<(), PaneError> {
        validate_overlay_dimension("overlay_height", h)?;
        if let Some(&idx) = self.overlay_index.get(kind) {
            self.overlays[idx].height.value = overlay::ExtentValue::Fixed(h);
        }
        Ok(())
    }

    /// Update an overlay's width (fixed value).
    pub fn set_overlay_width(&mut self, kind: &str, w: f32) -> Result<(), PaneError> {
        validate_overlay_dimension("overlay_width", w)?;
        if let Some(&idx) = self.overlay_index.get(kind) {
            self.overlays[idx].width.value = overlay::ExtentValue::Fixed(w);
        }
        Ok(())
    }

    /// Look up an overlay definition by kind.
    pub fn overlay(&self, kind: &str) -> Option<&OverlayDef> {
        self.overlay_index.get(kind).map(|&idx| &self.overlays[idx])
    }

    /// Resolve the layout at the given dimensions, producing a Frame with layout and diff.
    pub fn resolve(&mut self, width: f32, height: f32) -> Result<Frame, PaneError> {
        let tree_dirty = self.tree.is_dirty();
        let (mut result, cached_kinds) = self.compile_tree(tree_dirty)?;
        compute_layout(&mut result, width, height)?;

        let mut layout = self.resolve_layout(&result, cached_kinds)?;
        self.cached_compile = Some(result);

        apply_scroll_offset(&mut layout, self.viewport.scroll_offset);
        self.resolve_overlays(&mut layout, width, height);

        self.compute_diffs(&layout, tree_dirty);

        let layout = Arc::new(layout);
        let prev_arc = self.previous.replace(Arc::clone(&layout));

        // Reclaim the previous frame's buffers if no other consumers hold a reference.
        if let Some(Ok(mut prev_layout)) = prev_arc.map(Arc::try_unwrap) {
            self.rects_buf = Some(prev_layout.take_rects());
            self.overlay_rects_buf = prev_layout.take_overlay_rects();
        }

        Ok(Frame { layout })
    }

    fn compile_tree(
        &mut self,
        tree_dirty: bool,
    ) -> Result<(CompileResult, Option<resolver::KindIndex>), PaneError> {
        let result = match (tree_dirty, self.cached_compile.take()) {
            (false, Some(cached)) => cached,
            _ => {
                self.tree.clear_dirty();
                compile(&self.tree)?
            }
        };
        let cached_kinds = match tree_dirty {
            false => self.cached_kinds.take(),
            true => None,
        };
        Ok((result, cached_kinds))
    }

    fn resolve_layout(
        &mut self,
        result: &CompileResult,
        cached_kinds: Option<resolver::KindIndex>,
    ) -> Result<ResolvedLayout, PaneError> {
        let layout = match cached_kinds {
            Some(kinds) => resolver::resolve_with_cached_kinds(
                result,
                &self.tree,
                kinds,
                &mut self.resolve_scratch,
                self.rects_buf.take(),
            )?,
            None => resolver::resolve(result, &self.tree)?,
        };
        self.cached_kinds = Some(Arc::clone(layout.kinds_arc()));
        Ok(layout)
    }

    fn resolve_overlays(&mut self, layout: &mut ResolvedLayout, width: f32, height: f32) {
        crate::runtime_overlay::resolve_overlays_impl(
            &self.overlays,
            &mut self.overlay_rects_buf,
            layout,
            width,
            height,
        );
    }

    /// The layout diff from the most recent `resolve()` call.
    ///
    /// Borrows from internal scratch buffers. Valid until the next `resolve()`.
    pub fn last_diff(&self) -> LayoutDiff<'_> {
        self.diff_scratch.as_diff()
    }

    /// The overlay diff from the most recent `resolve()` call.
    ///
    /// Borrows from internal scratch buffers. Valid until the next `resolve()`.
    pub fn last_overlay_diff(&self) -> OverlayDiff<'_> {
        self.overlay_diff_scratch.as_diff()
    }

    fn compute_diffs(&mut self, layout: &ResolvedLayout, tree_dirty: bool) {
        select_diff(
            tree_dirty,
            self.previous.as_deref(),
            layout,
            &mut self.diff_scratch,
        );

        match self.prev_overlay_rects.is_empty() {
            true => {
                diff::first_frame_overlays(
                    layout.overlay_rects_raw(),
                    &mut self.overlay_diff_scratch,
                );
            }
            false => {
                diff::diff_overlays(
                    &self.prev_overlay_rects,
                    layout.overlay_rects_raw(),
                    &mut self.overlay_diff_scratch,
                );
            }
        };

        self.prev_overlay_rects.clear();
        self.prev_overlay_rects.extend(
            layout
                .overlay_rects_raw()
                .iter()
                .map(|(id, _, rect)| (*id, *rect)),
        );
    }
}

/// Restore overlay definitions from snapshot data.
fn restore_overlays(
    rt: &mut LayoutRuntime,
    overlays: &[overlay::SnapshotOverlay],
) -> Result<(), PaneError> {
    for snap_overlay in overlays {
        let kind: Arc<str> = Arc::from(&*snap_overlay.kind);
        let id = rt.overlay_gen.next_id()?;
        let def = OverlayDef {
            id,
            kind: Arc::clone(&kind),
            anchor: snap_overlay.anchor.clone(),
            width: snap_overlay.width,
            height: snap_overlay.height,
            visible: snap_overlay.visible,
        };
        let idx = rt.overlays.len();
        rt.overlays.push(def);
        rt.overlay_index.insert(kind, idx);
    }
    Ok(())
}

impl From<LayoutTree> for LayoutRuntime {
    fn from(tree: LayoutTree) -> Self {
        Self::new(tree)
    }
}

fn select_diff(
    tree_dirty: bool,
    prev: Option<&ResolvedLayout>,
    new: &ResolvedLayout,
    scratch: &mut diff::DiffScratch,
) {
    match (tree_dirty, prev) {
        (_, None) => {
            diff::first_frame(new, scratch);
        }
        (false, Some(prev)) => {
            diff::diff_same_panels_reuse(prev, new, scratch);
        }
        (true, Some(prev)) => {
            diff::diff_reuse(prev, new, scratch);
        }
    };
}

fn find_focused_position(
    tree: &LayoutTree,
    focused: PanelId,
) -> Result<(NodeId, NodeId, usize, Direction), PaneError> {
    let focused_nid = tree
        .node_for_panel(focused)
        .ok_or(PaneError::PanelNotFound(focused))?;
    let parent_id = tree
        .parent(focused_nid)?
        .ok_or(PaneError::InvalidMutation(MutationError::FocusedNoParent))?;
    let parent_axis = parent_axis_direction(tree, parent_id)?;
    let focused_idx = tree
        .children(parent_id)?
        .iter()
        .position(|&c| c == focused_nid)
        .ok_or(PaneError::PanelNotFound(focused))?;
    Ok((focused_nid, parent_id, focused_idx, parent_axis))
}

fn parent_axis_direction(tree: &LayoutTree, parent_id: NodeId) -> Result<Direction, PaneError> {
    let node = tree.node(parent_id).ok_or(PaneError::InvalidMutation(
        MutationError::ParentNotContainer,
    ))?;
    match node {
        Node::Panel { .. } => Err(PaneError::InvalidMutation(
            MutationError::ParentNotContainer,
        )),
        _ => Ok(crate::compiler::direction_of(node)),
    }
}

fn wrap_in_container(
    tree: &mut LayoutTree,
    parent_id: NodeId,
    focused_nid: NodeId,
    focused_idx: usize,
    new_nid: NodeId,
    direction: Direction,
    placement: Placement,
) -> Result<(), PaneError> {
    tree.detach(focused_nid);
    let children = match placement {
        Placement::Before => vec![new_nid, focused_nid],
        Placement::After | Placement::End => vec![focused_nid, new_nid],
    };
    let c = match direction {
        Direction::Horizontal => tree.add_row(0.0, children)?,
        Direction::Vertical => tree.add_col(0.0, children)?,
    };
    tree.insert_child_at(parent_id, focused_idx, c)
}

/// Shift all resolved rect x-positions by the negative scroll offset.
fn apply_scroll_offset(layout: &mut ResolvedLayout, offset: f32) {
    match offset.abs() < f32::EPSILON {
        true => {}
        false => layout.shift_x(-offset),
    }
}

/// Collect all panel IDs from the tree in depth-first order.
fn collect_panels_depth_first(tree: &LayoutTree, seq: &mut PanelSequence) {
    let Some(root) = tree.root() else { return };
    collect_panels_recursive(tree, root, seq);
}

fn collect_panels_recursive(tree: &LayoutTree, nid: NodeId, seq: &mut PanelSequence) {
    let Some(node) = tree.node(nid) else { return };
    match node {
        Node::Panel { id, .. } => seq.push(*id),
        _ => {
            for &child in node.children() {
                collect_panels_recursive(tree, child, seq);
            }
        }
    }
}

fn validate_overlay_dimension(name: &'static str, value: f32) -> Result<(), PaneError> {
    check_f32_non_negative(value)
        .map_err(|e| PaneError::InvalidConstraint(float_invalid_to_constraint(name, e)))
}

impl From<Layout> for LayoutRuntime {
    fn from(layout: Layout) -> Self {
        Self::new(LayoutTree::from(layout))
    }
}
