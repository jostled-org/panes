use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::breakpoint::BreakpointEntry;
use crate::compiler::CompileResult;
use crate::diff::{self, OverlayDiffScratch};
use crate::overlay::{OverlayDef, OverlayId, OverlayIdGenerator};
use crate::rect::Rect;
use crate::resolver::{self, ResolveScratch, ResolvedLayout};
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

/// Stateful layout wrapper that tracks tree, viewport, and frame history.
pub struct LayoutRuntime {
    pub(crate) tree: LayoutTree,
    pub(crate) viewport: ViewportState,
    pub(crate) previous: Option<Arc<ResolvedLayout>>,
    pub(crate) cached_compile: Option<CompileResult>,
    pub(crate) cached_kinds: Option<resolver::KindIndex>,
    pub(crate) rects_buf: Option<Vec<Option<Rect>>>,
    pub(crate) rects_buf_alt: Option<Vec<Option<Rect>>>,
    pub(crate) diff_scratch: diff::DiffScratch,
    pub(crate) overlay_diff_scratch: OverlayDiffScratch,
    pub(crate) resolve_scratch: ResolveScratch,
    pub(crate) strategy_source: StrategySource,
    pub(crate) sequence: PanelSequence,
    pub(crate) overlays: Vec<OverlayDef>,
    pub(crate) overlay_gen: OverlayIdGenerator,
    pub(crate) overlay_index: FxHashMap<Arc<str>, usize>,
    pub(crate) prev_overlay_rects: Vec<(OverlayId, Rect)>,
    pub(crate) overlay_rects_buf: Vec<(OverlayId, Arc<str>, Rect)>,
    pub(crate) overlay_rects_buf_alt: Vec<(OverlayId, Arc<str>, Rect)>,
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
        tree,
        viewport,
        previous: None,
        cached_compile: None,
        cached_kinds: None,
        rects_buf: None,
        rects_buf_alt: None,
        diff_scratch: diff::DiffScratch::default(),
        overlay_diff_scratch: OverlayDiffScratch::default(),
        resolve_scratch: ResolveScratch::default(),
        strategy_source,
        sequence,
        overlays: Vec::new(),
        overlay_gen: OverlayIdGenerator::default(),
        overlay_index: FxHashMap::default(),
        prev_overlay_rects: Vec::new(),
        overlay_rects_buf: Vec::new(),
        overlay_rects_buf_alt: Vec::new(),
        panel_sizes: Vec::new(),
        breakpoints: None,
        active_bp_idx: 0,
    }
}
