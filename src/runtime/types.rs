use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::breakpoint::BreakpointEntry;
use crate::compiler::CompileResult;
use crate::diff;
use crate::node::PanelId;
use crate::overlay::{AnchorFailure, OverlayDef, OverlayId, OverlayIdGenerator};
use crate::rect::Rect;
use crate::resolver::{self, DecorationIndex, ResolveScratch, ResolvedLayout};
use crate::sequence::PanelSequence;
use crate::strategy::StrategyKind;
use crate::tree::LayoutTree;
use crate::viewport::ViewportState;

/// Where the active strategy lives.
///
/// For adaptive layouts the strategy is already stored inside
/// `breakpoints[active_bp_idx]`, so we borrow it instead of cloning.
pub(crate) enum StrategySource {
    /// No strategy (direct tree control).
    None,
    /// Non-adaptive: the runtime owns a standalone strategy.
    Standalone(StrategyKind),
    /// Adaptive: the active strategy lives in `breakpoints[active_bp_idx]`.
    Adaptive,
}

/// Borrow the active strategy from disjoint fields, enabling split borrows.
pub(crate) fn strategy_ref<'a>(
    source: &'a StrategySource,
    breakpoints: &'a Option<Box<[BreakpointEntry]>>,
    active_bp_idx: usize,
) -> Option<&'a StrategyKind> {
    match source {
        StrategySource::None => None,
        StrategySource::Standalone(s) => Some(s),
        StrategySource::Adaptive => breakpoints.as_ref().map(|bps| &bps[active_bp_idx].strategy),
    }
}

/// Mutable variant of [`strategy_ref`] for in-place strategy updates.
pub(crate) fn strategy_ref_mut<'a>(
    source: &'a mut StrategySource,
    breakpoints: &'a mut Option<Box<[BreakpointEntry]>>,
    active_bp_idx: usize,
) -> Option<&'a mut StrategyKind> {
    match source {
        StrategySource::None => None,
        StrategySource::Standalone(s) => Some(s),
        StrategySource::Adaptive => breakpoints
            .as_mut()
            .map(|bps| &mut bps[active_bp_idx].strategy),
    }
}

/// Tracks which kind of invalidation is pending.
///
/// `topology` means the panel set or kind membership changed — cached kind
/// indices, sorted kind keys, and decoration index must be rebuilt.
/// `layout` means constraints or sizes changed — recompilation is needed but
/// kind caches can be reused.
#[derive(Clone, Copy, Default)]
pub(crate) struct DirtyState {
    pub(crate) topology: bool,
    pub(crate) layout: bool,
}

impl DirtyState {
    /// Panel set or kind membership changed — full invalidation.
    pub(crate) fn mark_topology(&mut self) {
        self.topology = true;
        self.layout = true;
    }

    /// Constraints or sizes changed — recompile but preserve kind caches.
    pub(crate) fn mark_layout(&mut self) {
        self.layout = true;
    }

    pub(crate) fn clear(&mut self) {
        self.topology = false;
        self.layout = false;
    }
}

/// Stateful layout wrapper that tracks tree, viewport, and frame history.
pub struct LayoutRuntime {
    pub(crate) dirty: DirtyState,
    pub(crate) tree: LayoutTree,
    pub(crate) viewport: ViewportState,
    pub(crate) previous: Option<Arc<ResolvedLayout>>,
    pub(crate) cached_compile: Option<CompileResult>,
    pub(crate) cached_kinds: Option<resolver::KindIndex>,
    pub(crate) cached_sorted_kind_keys: Option<Arc<[Arc<str>]>>,
    pub(crate) cached_panel_kind_indices: Option<Arc<[Option<u16>]>>,
    pub(crate) cached_decorations: Option<DecorationIndex>,
    pub(crate) cached_decoration_roles: Option<resolver::DecorationRoleIndex>,
    pub(crate) cached_live_panel_ids: Option<Arc<[PanelId]>>,
    pub(crate) rects_buf: Option<Vec<Option<Rect>>>,
    pub(crate) rects_buf_alt: Option<Vec<Option<Rect>>>,
    pub(crate) diff_scratch: diff::PanelScratch,
    pub(crate) overlay_diff_scratch: diff::OverlayDiffScratch,
    pub(crate) resolve_scratch: ResolveScratch,
    pub(crate) strategy_source: StrategySource,
    pub(crate) sequence: PanelSequence,
    pub(crate) overlays: Vec<OverlayDef>,
    pub(crate) overlay_gen: OverlayIdGenerator,
    pub(crate) overlay_index: FxHashMap<Arc<str>, usize>,
    pub(crate) overlay_rects_buf: Vec<(OverlayId, Arc<str>, Rect)>,
    pub(crate) overlay_rects_buf_alt: Vec<(OverlayId, Arc<str>, Rect)>,
    pub(crate) overlay_failures_buf: Vec<(OverlayId, Arc<str>, AnchorFailure)>,
    pub(crate) overlay_failures_buf_alt: Vec<(OverlayId, Arc<str>, AnchorFailure)>,
    pub(crate) panel_sizes: Vec<Option<(f32, f32)>>,
    pub(crate) breakpoints: Option<Box<[BreakpointEntry]>>,
    pub(crate) active_bp_idx: usize,
}

/// Shared default fields for all constructors.
pub(crate) fn base(
    tree: LayoutTree,
    viewport: ViewportState,
    strategy_source: StrategySource,
    sequence: PanelSequence,
) -> LayoutRuntime {
    LayoutRuntime {
        dirty: DirtyState::default(),
        tree,
        viewport,
        previous: None,
        cached_compile: None,
        cached_kinds: None,
        cached_sorted_kind_keys: None,
        cached_panel_kind_indices: None,
        cached_decorations: None,
        cached_decoration_roles: None,
        cached_live_panel_ids: None,
        rects_buf: None,
        rects_buf_alt: None,
        diff_scratch: diff::PanelScratch::default(),
        overlay_diff_scratch: diff::OverlayDiffScratch::default(),
        resolve_scratch: ResolveScratch::default(),
        strategy_source,
        sequence,
        overlays: Vec::new(),
        overlay_gen: OverlayIdGenerator::default(),
        overlay_index: FxHashMap::default(),
        overlay_rects_buf: Vec::new(),
        overlay_rects_buf_alt: Vec::new(),
        overlay_failures_buf: Vec::new(),
        overlay_failures_buf_alt: Vec::new(),
        panel_sizes: Vec::new(),
        breakpoints: None,
        active_bp_idx: 0,
    }
}
