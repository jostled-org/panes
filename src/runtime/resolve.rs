use std::sync::Arc;

use super::frame::Frame;
use super::types::LayoutRuntime;
use crate::compiler::CompileResult;
use crate::diff::{self, LayoutDiff, OverlayDiff};
use crate::error::PaneError;
use crate::overlay::OverlayId;
use crate::rect::Rect;
use crate::resolver::{self, ResolvedLayout};

impl LayoutRuntime {
    /// Resolve the layout at the given dimensions, producing a Frame with layout and diff.
    pub fn resolve(&mut self, width: f32, height: f32) -> Result<Frame, PaneError> {
        self.maybe_switch_breakpoint(width)?;
        let tree_dirty = self.tree.is_dirty();
        let (mut result, cached_kinds) = self.compile_tree(tree_dirty)?;
        crate::compiler::compute_layout(&mut result, width, height)?;

        let mut layout = self.resolve_layout(&result, cached_kinds)?;
        self.cached_compile = Some(result);

        apply_scroll_offset(&mut layout, self.viewport.scroll_offset);
        self.resolve_overlays(&mut layout, width, height);

        self.compute_diffs(&layout, tree_dirty);

        let layout = Arc::new(layout);
        let prev_arc = self.previous.replace(Arc::clone(&layout));

        // Double-buffer rotation: move the alternate buffer into the primary slot
        // so the next resolve() always has a buffer to give the resolver, even when
        // the consumer still holds a Frame from the previous call.
        rotate_buf(&mut self.rects_buf, &mut self.rects_buf_alt);
        rotate_overlay_buf(&mut self.overlay_rects_buf, &mut self.overlay_rects_buf_alt);

        // Bonus: reclaim the previous frame's buffers if no other consumers hold a reference.
        // This replenishes the alternate slot for the frame after next.
        reclaim_buffers(prev_arc, self);

        Ok(Frame::new(layout))
    }

    pub(crate) fn compile_tree(
        &mut self,
        tree_dirty: bool,
    ) -> Result<(CompileResult, Option<resolver::KindIndex>), PaneError> {
        let result = match (tree_dirty, self.cached_compile.take()) {
            (false, Some(cached)) => cached,
            (_, old) => {
                self.tree.clear_dirty();
                crate::compiler::compile_with(&self.tree, old)?
            }
        };
        let cached_kinds = match tree_dirty {
            false => self.cached_kinds.take(),
            true => None,
        };
        Ok((result, cached_kinds))
    }

    pub(crate) fn resolve_layout(
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
            None => resolver::resolve_dirty(
                result,
                &self.tree,
                &mut self.resolve_scratch,
                self.rects_buf.take(),
            )?,
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

        let new_overlay_rects = layout.overlay_rects_raw();
        match (
            self.prev_overlay_rects.is_empty(),
            new_overlay_rects.is_empty(),
        ) {
            (true, true) => {}
            _ => {
                self.prev_overlay_rects.clear();
                self.prev_overlay_rects
                    .extend(new_overlay_rects.iter().map(|(id, _, rect)| (*id, *rect)));
            }
        }
    }
}

/// Promote the alternate rects buffer into the primary slot when the primary is empty.
fn rotate_buf(primary: &mut Option<Vec<Option<Rect>>>, alt: &mut Option<Vec<Option<Rect>>>) {
    if let (None, Some(_)) = (primary.as_ref(), alt.as_ref()) {
        *primary = alt.take();
    }
}

/// Promote the alternate overlay buffer into the primary slot when the primary is empty.
fn rotate_overlay_buf(
    primary: &mut Vec<(OverlayId, Arc<str>, Rect)>,
    alt: &mut Vec<(OverlayId, Arc<str>, Rect)>,
) {
    if primary.is_empty() {
        std::mem::swap(primary, alt);
    }
}

/// Reclaim buffers from the previous frame if no other consumers hold a reference.
fn reclaim_buffers(prev_arc: Option<Arc<ResolvedLayout>>, rt: &mut LayoutRuntime) {
    let Some(Ok(mut prev_layout)) = prev_arc.map(Arc::try_unwrap) else {
        return;
    };
    let reclaimed_rects = prev_layout.take_rects();
    let reclaimed_overlay = prev_layout.take_overlay_rects();

    match rt.rects_buf.is_none() {
        true => rt.rects_buf = Some(reclaimed_rects),
        false => rt.rects_buf_alt = Some(reclaimed_rects),
    }
    match rt.overlay_rects_buf_alt.is_empty() {
        true => rt.overlay_rects_buf_alt = reclaimed_overlay,
        false => {}
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

/// Shift all resolved rect x-positions by the negative scroll offset.
fn apply_scroll_offset(layout: &mut ResolvedLayout, offset: f32) {
    match offset.abs() < f32::EPSILON {
        true => {}
        false => layout.shift_x(-offset),
    }
}
