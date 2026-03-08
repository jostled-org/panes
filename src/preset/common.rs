use std::sync::Arc;

use crate::builder::LayoutBuilder;
use crate::error::PaneError;
use crate::layout::Layout;
use crate::panel::{fixed, grow};

/// Collect an iterator of string-like items into an `Arc<[Arc<str>]>`.
pub(crate) fn collect_kinds(
    kinds: impl IntoIterator<Item = impl Into<Arc<str>>>,
) -> Arc<[Arc<str>]> {
    kinds.into_iter().map(Into::into).collect()
}

/// Build a single-panel layout. Shared by presets that degenerate when given one kind.
pub(crate) fn build_single(kind: Arc<str>) -> Result<Layout, PaneError> {
    let mut b = LayoutBuilder::new();
    b.row(|r| {
        r.panel(kind);
    })?;
    b.build()
}

/// Add one grow(1.0) panel per kind. Shared by columns and grid presets.
pub(crate) fn add_grow_panels(ctx: &mut crate::ContainerCtx, kinds: &[Arc<str>]) {
    for kind in kinds {
        ctx.panel(Arc::clone(kind));
    }
}

/// Validate that at least one kind was provided.
pub(crate) fn validate_kinds(kinds: &[Arc<str>]) -> Result<(), PaneError> {
    match kinds.is_empty() {
        true => Err(PaneError::InvalidTree(
            "preset requires at least one kind".into(),
        )),
        false => Ok(()),
    }
}

/// Validate that an `f32` parameter is finite and non-negative.
pub(crate) fn validate_f32_param(name: &str, value: f32) -> Result<(), PaneError> {
    match value {
        v if v.is_nan() => Err(PaneError::InvalidConstraint(
            format!("{name} is NaN").into(),
        )),
        v if v < 0.0 => Err(PaneError::InvalidConstraint(
            format!("{name} is negative").into(),
        )),
        v if v.is_infinite() => Err(PaneError::InvalidConstraint(
            format!("{name} is infinite").into(),
        )),
        _ => Ok(()),
    }
}

/// Validate that `active` is within bounds.
pub(crate) fn validate_active(active: usize, len: usize) -> Result<(), PaneError> {
    match active >= len {
        true => Err(PaneError::InvalidTree(
            format!("active index {active} out of bounds for {len} panels").into(),
        )),
        false => Ok(()),
    }
}

/// Add panels where only the active one grows; the rest are hidden (fixed 0).
pub(crate) fn add_active_hidden_panels(
    ctx: &mut crate::ContainerCtx,
    kinds: &[Arc<str>],
    active: usize,
) {
    for (i, kind) in kinds.iter().enumerate() {
        let constraint = match i == active {
            true => grow(1.0),
            false => fixed(0.0),
        };
        ctx.panel_with(Arc::clone(kind), constraint);
    }
}

/// Shared implementation for all preset types: `resolve()` shorthand and `TryFrom` conversion.
macro_rules! impl_preset {
    ($Type:ty) => {
        impl $Type {
            /// Build and resolve the preset at the given viewport size.
            pub fn resolve(
                &self,
                width: f32,
                height: f32,
            ) -> Result<$crate::ResolvedLayout, $crate::PaneError> {
                self.build()?.resolve(width, height)
            }
        }

        impl TryFrom<$Type> for $crate::Layout {
            type Error = $crate::PaneError;

            fn try_from(preset: $Type) -> Result<Self, Self::Error> {
                preset.build()
            }
        }
    };
}

pub(crate) use impl_preset;
