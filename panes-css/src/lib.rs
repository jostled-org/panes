//! Transpile panes layouts into CSS flexbox and grid declarations.

mod emit;

pub use emit::{
    AdaptiveCssError, emit, emit_adaptive, emit_full, emit_with_overlays, emit_with_transitions,
};
