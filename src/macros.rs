/// Declarative macro for building layouts from a concise DSL.
///
/// Returns `Result<Layout, PaneError>`.
///
/// # Syntax
///
/// ```text
/// layout! {
///     row(gap: 8.0) {
///         panel("editor", grow: 2.0, min: 40.0)
///         col {
///             panel("chat")
///             panel("status", fixed: 3.0)
///         }
///     }
/// }
/// ```
///
/// - Root must be a single `row` or `col`.
/// - Containers accept an optional `(gap: N)` parameter.
/// - Bare `panel("kind")` defaults to `grow(1.0)`.
/// - `panel("kind", grow: N)` and `panel("kind", fixed: N)` set constraints.
/// - Panels accept optional `min:` and `max:` after the primary constraint.
#[macro_export]
macro_rules! layout {
    // -- Root: row/col with or without gap --

    (row(gap: $gap:expr) { $($children:tt)* }) => {
        $crate::layout!(@root row [$crate::gap($gap)] $($children)*)
    };
    (row { $($children:tt)* }) => {
        $crate::layout!(@root row [$crate::gap(0.0)] $($children)*)
    };
    (col(gap: $gap:expr) { $($children:tt)* }) => {
        $crate::layout!(@root col [$crate::gap($gap)] $($children)*)
    };
    (col { $($children:tt)* }) => {
        $crate::layout!(@root col [$crate::gap(0.0)] $($children)*)
    };

    // Internal: shared root builder
    (@root $dir:ident [$gap:expr] $($children:tt)*) => {
        (|| -> ::core::result::Result<$crate::Layout, $crate::PaneError> {
            let mut __builder = $crate::LayoutBuilder::new();
            __builder.$dir($gap, |__ctx| {
                $crate::layout!(@children __ctx $($children)*);
                Ok(())
            })?;
            __builder.build()
        })()
    };

    // -- Children dispatch: peel off one child at a time --

    // Base case: no more children
    (@children $ctx:ident) => {};

    // -- Panel with grow + min + max --
    (@children $ctx:ident panel($kind:expr, grow: $val:expr, min: $min:expr, max: $max:expr) $($rest:tt)*) => {
        $ctx.panel($kind, $crate::grow($val).min($min).max($max))?;
        $crate::layout!(@children $ctx $($rest)*);
    };

    // -- Panel with grow + min --
    (@children $ctx:ident panel($kind:expr, grow: $val:expr, min: $min:expr) $($rest:tt)*) => {
        $ctx.panel($kind, $crate::grow($val).min($min))?;
        $crate::layout!(@children $ctx $($rest)*);
    };

    // -- Panel with grow + max --
    (@children $ctx:ident panel($kind:expr, grow: $val:expr, max: $max:expr) $($rest:tt)*) => {
        $ctx.panel($kind, $crate::grow($val).max($max))?;
        $crate::layout!(@children $ctx $($rest)*);
    };

    // -- Panel with grow only --
    (@children $ctx:ident panel($kind:expr, grow: $val:expr) $($rest:tt)*) => {
        $ctx.panel($kind, $crate::grow($val))?;
        $crate::layout!(@children $ctx $($rest)*);
    };

    // -- Panel with fixed + min + max --
    (@children $ctx:ident panel($kind:expr, fixed: $val:expr, min: $min:expr, max: $max:expr) $($rest:tt)*) => {
        $ctx.panel($kind, $crate::fixed($val).min($min).max($max))?;
        $crate::layout!(@children $ctx $($rest)*);
    };

    // -- Panel with fixed + min --
    (@children $ctx:ident panel($kind:expr, fixed: $val:expr, min: $min:expr) $($rest:tt)*) => {
        $ctx.panel($kind, $crate::fixed($val).min($min))?;
        $crate::layout!(@children $ctx $($rest)*);
    };

    // -- Panel with fixed + max --
    (@children $ctx:ident panel($kind:expr, fixed: $val:expr, max: $max:expr) $($rest:tt)*) => {
        $ctx.panel($kind, $crate::fixed($val).max($max))?;
        $crate::layout!(@children $ctx $($rest)*);
    };

    // -- Panel with fixed only --
    (@children $ctx:ident panel($kind:expr, fixed: $val:expr) $($rest:tt)*) => {
        $ctx.panel($kind, $crate::fixed($val))?;
        $crate::layout!(@children $ctx $($rest)*);
    };

    // -- Panel bare — defaults to grow(1.0) --
    (@children $ctx:ident panel($kind:expr) $($rest:tt)*) => {
        $ctx.panel($kind, $crate::grow(1.0))?;
        $crate::layout!(@children $ctx $($rest)*);
    };

    // -- Nested row/col with or without gap --
    (@children $ctx:ident row(gap: $gap:expr) { $($inner:tt)* } $($rest:tt)*) => {
        $crate::layout!(@nested $ctx row [$crate::gap($gap)] { $($inner)* } $($rest)*);
    };
    (@children $ctx:ident row { $($inner:tt)* } $($rest:tt)*) => {
        $crate::layout!(@nested $ctx row [$crate::gap(0.0)] { $($inner)* } $($rest)*);
    };
    (@children $ctx:ident col(gap: $gap:expr) { $($inner:tt)* } $($rest:tt)*) => {
        $crate::layout!(@nested $ctx col [$crate::gap($gap)] { $($inner)* } $($rest)*);
    };
    (@children $ctx:ident col { $($inner:tt)* } $($rest:tt)*) => {
        $crate::layout!(@nested $ctx col [$crate::gap(0.0)] { $($inner)* } $($rest)*);
    };

    // Internal: shared nested container builder
    (@nested $ctx:ident $dir:ident [$gap:expr] { $($inner:tt)* } $($rest:tt)*) => {
        $ctx.$dir($gap, |__ctx| {
            $crate::layout!(@children __ctx $($inner)*);
            Ok(())
        })?;
        $crate::layout!(@children $ctx $($rest)*);
    };
}
