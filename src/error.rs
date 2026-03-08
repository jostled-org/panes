use crate::{NodeId, PanelId};

/// Errors arising from layout operations on panels and nodes.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum PaneError {
    /// A panel ID does not exist in the tree.
    #[error("panel not found: {0}")]
    PanelNotFound(PanelId),

    /// A constraint value is invalid (NaN, negative, mutually exclusive).
    #[error("invalid constraint: {0}")]
    InvalidConstraint(Box<str>),

    /// A node ID does not exist in the arena.
    #[error("node not found: {0}")]
    NodeNotFound(NodeId),

    /// The tree structure is invalid or incomplete.
    #[error("tree validation failed: {0}")]
    InvalidTree(Box<str>),

    /// Viewport dimensions are invalid (NaN, negative, infinite).
    #[error("invalid viewport: {0}")]
    InvalidViewport(Box<str>),

    /// A mutation is not supported for the current strategy.
    #[error("invalid mutation: {0}")]
    InvalidMutation(Box<str>),

    /// A sequence index is out of bounds.
    #[error("sequence index {0} out of bounds for length {1}")]
    SequenceOutOfBounds(usize, usize),
}
