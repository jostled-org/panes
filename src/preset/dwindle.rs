use std::sync::Arc;

use crate::builder::{LayoutBuilder, gap};
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::grow;
use crate::preset::{collect_kinds, validate_f32_param, validate_kinds};
use crate::preset::master_stack::{col_style, row_style};

pub struct Dwindle {
    kinds: Arc<[Arc<str>]>,
    ratio: f32,
    gap: f32,
}

impl Dwindle {
    pub(crate) fn new(kinds: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self {
            kinds: collect_kinds(kinds),
            ratio: 0.5,
            gap: 0.0,
        }
    }

    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn build(&self) -> Result<Layout, PaneError> {
        validate_kinds(&self.kinds)?;
        validate_f32_param("ratio", self.ratio)?;

        let mut b = LayoutBuilder::new();
        let kinds = &self.kinds;
        let ratio = self.ratio;
        let gap_px = self.gap;

        b.row(gap(gap_px), |r| {
            build_recursive(r, kinds, 0, ratio, gap_px, false)
        })?;

        b.build()
    }
}

/// Recursively build dwindle/spiral layout.
/// `reverse_even` controls spiral behavior (child order reversal on even-depth levels >= 2).
pub(crate) fn build_recursive(
    ctx: &mut crate::ContainerCtx,
    kinds: &[Arc<str>],
    depth: usize,
    ratio: f32,
    gap_px: f32,
    reverse_even: bool,
) -> Result<(), PaneError> {
    match kinds.len() {
        0 => Ok(()),
        1 => {
            ctx.panel(Arc::clone(&kinds[0]), grow(1.0))?;
            Ok(())
        }
        2 => add_pair(ctx, kinds, depth, ratio, reverse_even),
        _ => add_nested(ctx, kinds, depth, ratio, gap_px, reverse_even),
    }
}

fn add_pair(
    ctx: &mut crate::ContainerCtx,
    kinds: &[Arc<str>],
    depth: usize,
    ratio: f32,
    reverse_even: bool,
) -> Result<(), PaneError> {
    let (first, second) = (Arc::clone(&kinds[0]), Arc::clone(&kinds[1]));
    let should_reverse = reverse_even && depth >= 2 && depth.is_multiple_of(2);
    match should_reverse {
        true => {
            ctx.panel(second, grow(1.0 - ratio))?;
            ctx.panel(first, grow(ratio))?;
        }
        false => {
            ctx.panel(first, grow(ratio))?;
            ctx.panel(second, grow(1.0 - ratio))?;
        }
    }
    Ok(())
}

fn add_nested(
    ctx: &mut crate::ContainerCtx,
    kinds: &[Arc<str>],
    depth: usize,
    ratio: f32,
    gap_px: f32,
    reverse_even: bool,
) -> Result<(), PaneError> {
    let first = Arc::clone(&kinds[0]);
    let rest = &kinds[1..];
    let should_reverse = reverse_even && depth >= 2 && depth.is_multiple_of(2);
    let next_depth = depth + 1;

    // Alternate between row and col at each depth
    let nest_style = match next_depth.is_multiple_of(2) {
        true => row_style(1.0 - ratio, gap_px),
        false => col_style(1.0 - ratio, gap_px),
    };

    match should_reverse {
        true => {
            ctx.taffy_node(nest_style, |inner| {
                build_recursive(inner, rest, next_depth, ratio, gap_px, reverse_even)
            })?;
            ctx.panel(first, grow(ratio))?;
        }
        false => {
            ctx.panel(first, grow(ratio))?;
            ctx.taffy_node(nest_style, |inner| {
                build_recursive(inner, rest, next_depth, ratio, gap_px, reverse_even)
            })?;
        }
    }
    Ok(())
}

super::impl_preset!(Dwindle);
