use crate::{NodeId, PanelId};

/// Errors arising from layout operations on panels and nodes.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum PaneError {
    #[error("panel not found: {0}")]
    PanelNotFound(PanelId),

    #[error("invalid constraint: {0}")]
    InvalidConstraint(Box<str>),

    #[error("node not found: {0}")]
    NodeNotFound(NodeId),

    #[error("tree validation failed: {0}")]
    InvalidTree(Box<str>),

    #[error("invalid viewport: {0}")]
    InvalidViewport(Box<str>),
}
