use thiserror::Error;

/// Invalid breakpoint list passed to [`emit_adaptive`](crate::emit_adaptive).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AdaptiveCssError {
    #[error(
        "adaptive breakpoints must be strictly increasing; index {index} width {width}px follows {previous_width}px"
    )]
    UnsortedOrDuplicate {
        index: usize,
        previous_width: u32,
        width: u32,
    },
}
