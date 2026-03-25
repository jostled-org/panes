/// Generates a newtype wrapper around `u32` with `from_raw`, `raw`, and `Display`.
macro_rules! id_newtype {
    ($(#[$meta:meta])* $vis:vis $Name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        $vis struct $Name(u32);

        impl $Name {
            /// Construct from a raw integer.
            pub fn from_raw(raw: u32) -> Self {
                Self(raw)
            }

            /// Return the underlying integer.
            pub fn raw(self) -> u32 {
                self.0
            }
        }

        impl std::fmt::Display for $Name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

pub(crate) use id_newtype;

/// Generates simple builder setter methods: `self.field = param; self`.
///
/// Each item is separated by `;`:
/// ```ignore
/// builder_setters!(
///     /// doc
///     sidebar_width(width: f32);
///     /// doc
///     gap(gap: f32)
/// );
/// ```
macro_rules! builder_setters {
    ($( $(#[$doc:meta])* $field:ident ($param:ident : $ty:ty) );+ $(;)?) => {
        $(
            $(#[$doc])*
            pub fn $field(mut self, $param: $ty) -> Self {
                self.$field = $param;
                self
            }
        )+
    };
}

/// Generates mapped builder setter methods: `self.field = expr; self`.
///
/// Method name may differ from field name. Each item is separated by `;`:
/// ```ignore
/// builder_mapped_setters!(
///     /// doc
///     columns(columns: usize) -> columns = GridColumnMode::Fixed(columns);
///     /// doc
///     auto_fill(min_width: f32) -> columns = GridColumnMode::AutoFill { min_width }
/// );
/// ```
macro_rules! builder_mapped_setters {
    ($( $(#[$doc:meta])* $method:ident ($param:ident : $ty:ty) -> $field:ident = $val:expr );+ $(;)?) => {
        $(
            $(#[$doc])*
            pub fn $method(mut self, $param: $ty) -> Self {
                self.$field = $val;
                self
            }
        )+
    };
}

/// Generates no-arg builder setter methods: `self.field = expr; self`.
///
/// Each item is separated by `;`:
/// ```ignore
/// builder_flag_setters!(
///     /// doc
///     auto_rows -> auto_rows = true
/// );
/// ```
macro_rules! builder_flag_setters {
    ($( $(#[$doc:meta])* $method:ident -> $field:ident = $val:expr );+ $(;)?) => {
        $(
            $(#[$doc])*
            pub fn $method(mut self) -> Self {
                self.$field = $val;
                self
            }
        )+
    };
}

pub(crate) use builder_flag_setters;
pub(crate) use builder_mapped_setters;
pub(crate) use builder_setters;

/// Declarative macro for building layouts from a concise DSL.
///
/// Returns `Result<Layout, PaneError>`.
///
/// # Syntax
///
/// ```text
/// layout! {
///     row(gap: 8.0) {
///         panel("editor", grow: 2.0)
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
/// - Panels accept optional modifiers after the primary constraint:
///   `min:`, `max:`, `min_width:`, `max_width:`, `min_height:`, `max_height:`, `align:`.
/// - Align values: `start`, `center`, `end`, `stretch`.
#[macro_export]
macro_rules! layout {
    // -- Root: row/col with or without gap --

    (row(gap: $gap:expr) { $($children:tt)* }) => {
        $crate::layout!(@root_gap row_gap [$gap] $($children)*)
    };
    (row { $($children:tt)* }) => {
        $crate::layout!(@root row $($children)*)
    };
    (col(gap: $gap:expr) { $($children:tt)* }) => {
        $crate::layout!(@root_gap col_gap [$gap] $($children)*)
    };
    (col { $($children:tt)* }) => {
        $crate::layout!(@root col $($children)*)
    };

    // Internal: root without gap
    (@root $dir:ident $($children:tt)*) => {
        (|| -> ::core::result::Result<$crate::Layout, $crate::PaneError> {
            let mut __builder = $crate::LayoutBuilder::new();
            __builder.$dir(|__ctx| {
                $crate::layout!(@children __ctx $($children)*);
            })?;
            __builder.build()
        })()
    };

    // Internal: root with gap
    (@root_gap $dir:ident [$gap:expr] $($children:tt)*) => {
        (|| -> ::core::result::Result<$crate::Layout, $crate::PaneError> {
            let mut __builder = $crate::LayoutBuilder::new();
            __builder.$dir($gap, |__ctx| {
                $crate::layout!(@children __ctx $($children)*);
            })?;
            __builder.build()
        })()
    };

    // -- Children dispatch: peel off one child at a time --

    // Base case: no more children
    (@children $ctx:ident) => {};

    // -- Panel with grow + modifiers --
    (@children $ctx:ident panel($kind:expr, grow: $val:expr, $($mods:tt)+) $($rest:tt)*) => {
        $ctx.panel_with($kind, $crate::layout!(@apply $crate::grow($val), $($mods)+));
        $crate::layout!(@children $ctx $($rest)*);
    };

    // -- Panel with grow only --
    (@children $ctx:ident panel($kind:expr, grow: $val:expr) $($rest:tt)*) => {
        $ctx.panel_with($kind, $crate::grow($val));
        $crate::layout!(@children $ctx $($rest)*);
    };

    // -- Panel with fixed + modifiers --
    (@children $ctx:ident panel($kind:expr, fixed: $val:expr, $($mods:tt)+) $($rest:tt)*) => {
        $ctx.panel_with($kind, $crate::layout!(@apply $crate::fixed($val), $($mods)+));
        $crate::layout!(@children $ctx $($rest)*);
    };

    // -- Panel with fixed only --
    (@children $ctx:ident panel($kind:expr, fixed: $val:expr) $($rest:tt)*) => {
        $ctx.panel_with($kind, $crate::fixed($val));
        $crate::layout!(@children $ctx $($rest)*);
    };

    // -- Panel bare — defaults to grow(1.0) --
    (@children $ctx:ident panel($kind:expr) $($rest:tt)*) => {
        $ctx.panel($kind);
        $crate::layout!(@children $ctx $($rest)*);
    };

    // -- Nested row/col with or without gap --
    (@children $ctx:ident row(gap: $gap:expr) { $($inner:tt)* } $($rest:tt)*) => {
        $crate::layout!(@nested_gap $ctx row_gap [$gap] { $($inner)* } $($rest)*);
    };
    (@children $ctx:ident row { $($inner:tt)* } $($rest:tt)*) => {
        $crate::layout!(@nested $ctx row { $($inner)* } $($rest)*);
    };
    (@children $ctx:ident col(gap: $gap:expr) { $($inner:tt)* } $($rest:tt)*) => {
        $crate::layout!(@nested_gap $ctx col_gap [$gap] { $($inner)* } $($rest)*);
    };
    (@children $ctx:ident col { $($inner:tt)* } $($rest:tt)*) => {
        $crate::layout!(@nested $ctx col { $($inner)* } $($rest)*);
    };

    // Internal: nested container without gap
    (@nested $ctx:ident $dir:ident { $($inner:tt)* } $($rest:tt)*) => {
        $ctx.$dir(|__ctx| {
            $crate::layout!(@children __ctx $($inner)*);
        });
        $crate::layout!(@children $ctx $($rest)*);
    };

    // Internal: nested container with gap
    (@nested_gap $ctx:ident $dir:ident [$gap:expr] { $($inner:tt)* } $($rest:tt)*) => {
        $ctx.$dir($gap, |__ctx| {
            $crate::layout!(@children __ctx $($inner)*);
        });
        $crate::layout!(@children $ctx $($rest)*);
    };

    // -- @apply: recursive modifier chaining --
    // Each modifier peels off one key: value pair and chains the builder method.

    (@apply $c:expr, min: $v:expr, $($rest:tt)+) => {
        $crate::layout!(@apply $c.min($v), $($rest)+)
    };
    (@apply $c:expr, min: $v:expr) => { $c.min($v) };

    (@apply $c:expr, max: $v:expr, $($rest:tt)+) => {
        $crate::layout!(@apply $c.max($v), $($rest)+)
    };
    (@apply $c:expr, max: $v:expr) => { $c.max($v) };

    (@apply $c:expr, min_width: $v:expr, $($rest:tt)+) => {
        $crate::layout!(@apply $c.min_width($v), $($rest)+)
    };
    (@apply $c:expr, min_width: $v:expr) => { $c.min_width($v) };

    (@apply $c:expr, max_width: $v:expr, $($rest:tt)+) => {
        $crate::layout!(@apply $c.max_width($v), $($rest)+)
    };
    (@apply $c:expr, max_width: $v:expr) => { $c.max_width($v) };

    (@apply $c:expr, min_height: $v:expr, $($rest:tt)+) => {
        $crate::layout!(@apply $c.min_height($v), $($rest)+)
    };
    (@apply $c:expr, min_height: $v:expr) => { $c.min_height($v) };

    (@apply $c:expr, max_height: $v:expr, $($rest:tt)+) => {
        $crate::layout!(@apply $c.max_height($v), $($rest)+)
    };
    (@apply $c:expr, max_height: $v:expr) => { $c.max_height($v) };

    (@apply $c:expr, align: $v:ident, $($rest:tt)+) => {
        $crate::layout!(@apply $c.align($crate::layout!(@align $v)), $($rest)+)
    };
    (@apply $c:expr, align: $v:ident) => { $c.align($crate::layout!(@align $v)) };

    // size_mode: simple variants (min_content, max_content)
    (@apply $c:expr, size_mode: $v:ident, $($rest:tt)+) => {
        $crate::layout!(@apply $c.size_mode($crate::layout!(@size_mode $v)), $($rest)+)
    };
    (@apply $c:expr, size_mode: $v:ident) => { $c.size_mode($crate::layout!(@size_mode $v)) };

    // size_mode: fit_content(N)
    (@apply $c:expr, size_mode: fit_content($v:expr), $($rest:tt)+) => {
        $crate::layout!(@apply $c.size_mode($crate::SizeMode::FitContent($v)), $($rest)+)
    };
    (@apply $c:expr, size_mode: fit_content($v:expr)) => { $c.size_mode($crate::SizeMode::FitContent($v)) };

    // -- @align: map bare identifiers to Align variants --
    (@align start) => { $crate::Align::Start };
    (@align center) => { $crate::Align::Center };
    (@align end) => { $crate::Align::End };
    (@align stretch) => { $crate::Align::Stretch };

    // -- @size_mode: map bare identifiers to SizeMode variants --
    (@size_mode min_content) => { $crate::SizeMode::MinContent };
    (@size_mode max_content) => { $crate::SizeMode::MaxContent };
}

/// Generates the 5 standard adapter functions for a renderer backend.
///
/// Each adapter converts `panes::Rect` into a renderer-specific rect type.
/// The macro eliminates the duplicated `convert`, `panels`, `panels_at`,
/// `overlays`, `overlays_at` pattern across adapter crates.
///
/// # Parameters
///
/// - `rect`: target rect type (e.g. `ratatui::layout::Rect`, `egui::Rect`)
/// - `origin`: origin type for `_at` offset variants (must be `Copy`)
/// - `convert_fn`: expression `&panes::Rect -> TargetRect`
/// - `convert_at_fn`: expression `(&panes::Rect, Origin) -> TargetRect`
#[macro_export]
macro_rules! impl_adapter {
    (
        rect: $target_rect:ty,
        origin: $origin_ty:ty,
        convert_fn: $convert:expr,
        convert_at_fn: $convert_at:expr $(,)?
    ) => {
        pub fn convert(
            resolved: &$crate::ResolvedLayout,
        ) -> $crate::__FxHashMap<$crate::PanelId, $target_rect> {
            resolved
                .iter()
                .map(|(pid, r)| (pid, ($convert)(r)))
                .collect()
        }

        pub fn panels(
            resolved: &$crate::ResolvedLayout,
        ) -> impl Iterator<Item = $crate::PanelEntry<'_, $target_rect>> {
            resolved.panels().map(|e| e.map_rect($convert))
        }

        pub fn panels_at(
            resolved: &$crate::ResolvedLayout,
            origin: $origin_ty,
        ) -> impl Iterator<Item = $crate::PanelEntry<'_, $target_rect>> {
            resolved
                .panels()
                .map(move |e| e.map_rect(|r| ($convert_at)(r, origin)))
        }

        pub fn overlays(
            resolved: &$crate::ResolvedLayout,
        ) -> impl Iterator<Item = $crate::OverlayEntry<'_, $target_rect>> {
            resolved.overlays().map(|e| e.map_rect($convert))
        }

        pub fn overlays_at(
            resolved: &$crate::ResolvedLayout,
            origin: $origin_ty,
        ) -> impl Iterator<Item = $crate::OverlayEntry<'_, $target_rect>> {
            resolved
                .overlays()
                .map(move |e| e.map_rect(|r| ($convert_at)(r, origin)))
        }
    };
}
