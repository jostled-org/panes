//! Transpile panes layouts into CSS flexbox and grid declarations.

mod emit;

pub use emit::{emit, emit_adaptive};
