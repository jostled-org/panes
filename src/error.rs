use crate::{NodeId, PanelId};

/// Invalid constraint parameter.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ConstraintError {
    /// A constraint value is NaN.
    #[error("{0} is NaN")]
    IsNan(&'static str),
    /// A constraint value is negative.
    #[error("{0} is negative")]
    IsNegative(&'static str),
    /// A constraint value is infinite.
    #[error("{0} is infinite")]
    IsInfinite(&'static str),
    /// Grow and fixed constraints are mutually exclusive.
    #[error("grow and fixed are mutually exclusive")]
    GrowFixedExclusive,
    /// Minimum exceeds maximum.
    #[error("min exceeds max")]
    MinExceedsMax,
    /// Resize delta must be finite.
    #[error("delta must be finite")]
    DeltaNotFinite,
    /// Grid span overflows u16.
    #[error("grid span {0} exceeds u16 max")]
    GridSpanOverflow(usize),
}

/// Tree structure validation failure.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum TreeError {
    /// Root is not set.
    #[error("root is not set")]
    RootNotSet,
    /// Root was already set.
    #[error("root already set")]
    RootAlreadySet,
    /// Root node missing from arena.
    #[error("root node {0} missing from arena")]
    RootMissing(NodeId),
    /// Panel ID counter exhausted.
    #[error("panel ID counter exhausted")]
    PanelIdExhausted,
    /// Overlay ID counter exhausted.
    #[error("overlay ID counter exhausted")]
    OverlayIdExhausted,
    /// Node arena size exceeds u32 capacity.
    #[error("node arena size exceeds u32 capacity")]
    ArenaOverflow,
    /// Node arena index exceeds u32 capacity.
    #[error("node arena index exceeds u32 capacity")]
    ArenaIndexOverflow,
    /// A container references a missing child.
    #[error("node {parent} references missing child {child}")]
    MissingChild {
        /// The parent node.
        parent: NodeId,
        /// The missing child node.
        child: NodeId,
    },
    /// A node has no parent entry.
    #[error("node {0} has no parent entry")]
    NoParentEntry(NodeId),
    /// A node's parent is missing from the arena.
    #[error("parent {parent} of node {child} missing from arena")]
    ParentMissing {
        /// The missing parent node.
        parent: NodeId,
        /// The child node.
        child: NodeId,
    },
    /// Parent-child mismatch between parent_map and children list.
    #[error("parent_map says {parent} is parent of {child}, but children list disagrees")]
    ParentChildMismatch {
        /// The parent node.
        parent: NodeId,
        /// The child node.
        child: NodeId,
    },
    /// At least one kind required.
    #[error("at least one kind required")]
    NoKinds,
    /// Active index out of bounds.
    #[error("active index {active} out of bounds for {len} panels")]
    ActiveOutOfBounds {
        /// The active index.
        active: usize,
        /// The number of panels.
        len: usize,
    },
    /// Dashboard requires at least one card.
    #[error("dashboard requires at least one card")]
    DashboardNoCards,
    /// Dashboard columns must be at least 1.
    #[error("dashboard columns must be at least 1")]
    DashboardNoColumns,
    /// Dashboard min_column_width must be positive and finite.
    #[error("dashboard min_column_width must be positive and finite")]
    DashboardMinWidthInvalid,
    /// Grid min_column_width must be positive and finite.
    #[error("min_column_width must be positive and finite")]
    GridMinWidthInvalid,
    /// Window size must be at least 1.
    #[error("window size must be at least 1")]
    WindowSizeZero,
    /// Tree empty after rebuild.
    #[error("empty after rebuild")]
    EmptyAfterRebuild,
    /// No root node.
    #[error("no root")]
    NoRoot,
    /// Column count must be at least 1.
    #[error("column count must be at least 1")]
    ColumnsCountZero,
    /// Tree has no serializable root for snapshot.
    #[error("no serializable root for snapshot")]
    SnapshotNoRoot,
    /// Adaptive layout requires at least one breakpoint.
    #[error("adaptive layout requires at least one breakpoint")]
    NoBreakpoints,
    /// Snapshot node tree exceeds maximum recursion depth.
    #[error("snapshot tree exceeds maximum depth of {0}")]
    SnapshotTooDeep(usize),
    /// Insert index exceeds container length.
    #[error("insert index {index} exceeds container length {len}")]
    InsertOutOfBounds {
        /// The requested index.
        index: usize,
        /// The container's child count.
        len: usize,
    },
    /// Wrapped error from Taffy or other dynamic source.
    #[error("{0}")]
    Dynamic(Box<str>),
}

/// Invalid viewport dimensions.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ViewportError {
    /// Dimension is NaN.
    #[error("dimension is NaN")]
    IsNan,
    /// Dimension is negative.
    #[error("dimension is negative")]
    IsNegative,
    /// Dimension is infinite.
    #[error("dimension is infinite")]
    IsInfinite,
    /// Scroll value is NaN or infinite.
    #[error("scroll value is not finite")]
    ScrollNotFinite,
    /// No saved constraints for a panel.
    #[error("no saved constraints for panel {0}")]
    NoSavedConstraints(PanelId),
}

/// A mutation is not supported or invalid for the current state.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum MutationError {
    /// No strategy set on the runtime.
    #[error("no strategy set")]
    NoStrategy,
    /// No panel is focused.
    #[error("no focused panel")]
    NoFocusedPanel,
    /// Focused panel has no parent.
    #[error("focused panel has no parent")]
    FocusedNoParent,
    /// Parent is not a container node.
    #[error("parent is not a container")]
    ParentNotContainer,
    /// Panel has no parent.
    #[error("panel has no parent")]
    PanelNoParent,
    /// Panel is the only child.
    #[error("panel is the only child")]
    OnlyChild,
    /// resize_boundary requires all siblings to be panels.
    #[error("resize_boundary requires all siblings to be panels")]
    SiblingsNotPanels,
    /// resize_boundary requires all siblings to use grow constraints.
    #[error("resize_boundary requires all siblings to use grow constraints")]
    SiblingsNotGrow,
    /// No collapsed slots to uncollapse.
    #[error("no collapsed slots to uncollapse")]
    NoCollapsedSlots,
    /// Slot has no saved constraints.
    #[error("slot has no saved constraints")]
    SlotNoSavedConstraints,
    /// Move not supported for this layout.
    #[error("move not supported for this layout")]
    MoveNotSupported,
    /// Spatial focus navigation is not supported for this strategy.
    #[error("spatial navigation not supported — use focus_next/focus_prev")]
    SpatialNavUnsupported,
}

/// Errors arising from layout operations on panels and nodes.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum PaneError {
    /// A panel ID does not exist in the tree.
    #[error("panel not found: {0}")]
    PanelNotFound(PanelId),

    /// A constraint value is invalid (NaN, negative, mutually exclusive).
    #[error("invalid constraint: {0}")]
    InvalidConstraint(ConstraintError),

    /// A node ID does not exist in the arena.
    #[error("node not found: {0}")]
    NodeNotFound(NodeId),

    /// The tree structure is invalid or incomplete.
    #[error("tree validation failed: {0}")]
    InvalidTree(TreeError),

    /// Viewport dimensions are invalid (NaN, negative, infinite).
    #[error("invalid viewport: {0}")]
    InvalidViewport(ViewportError),

    /// A mutation is not supported for the current strategy.
    #[error("invalid mutation: {0}")]
    InvalidMutation(MutationError),

    /// A sequence index is out of bounds.
    #[error("sequence index {0} out of bounds for length {1}")]
    SequenceOutOfBounds(usize, usize),
}
