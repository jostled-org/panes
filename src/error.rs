use crate::{NodeId, PanelId};

/// Invalid [`Constraints`](crate::Constraints) parameter.
#[derive(Debug, thiserror::Error)]
pub enum ConstraintError {
    /// The named field is NaN.
    #[error("{0} is NaN")]
    IsNan(&'static str),
    /// The named field is negative.
    #[error("{0} is negative")]
    IsNegative(&'static str),
    /// The named field is infinite.
    #[error("{0} is infinite")]
    IsInfinite(&'static str),
    /// `grow` and `fixed` are mutually exclusive.
    #[error("grow and fixed are mutually exclusive")]
    GrowFixedExclusive,
    /// `min` exceeds `max`.
    #[error("min exceeds max")]
    MinExceedsMax,
    /// Resize delta is NaN or infinite.
    #[error("delta must be finite")]
    DeltaNotFinite,
    /// Grid span exceeds `u16::MAX`.
    #[error("grid span {0} exceeds u16 max")]
    GridSpanOverflow(usize),
    /// A proportional value (0.0..=1.0) exceeded 1.0.
    #[error("{0} exceeds 1.0")]
    ExceedsOne(&'static str),
}

/// Broken tree structural invariant.
///
/// Returned by [`LayoutTree::validate`](crate::LayoutTree::validate) and
/// builder finalization. Most variants indicate an internal bug rather than
/// invalid user input.
#[derive(Debug, thiserror::Error)]
pub enum TreeError {
    #[error("root is not set")]
    RootNotSet,
    #[error("root already set")]
    RootAlreadySet,
    /// Root ID points to a deallocated arena slot.
    #[error("root node {0} missing from arena")]
    RootMissing(NodeId),
    #[error("panel ID counter exhausted")]
    PanelIdExhausted,
    #[error("overlay ID counter exhausted")]
    OverlayIdExhausted,
    #[error("node arena size exceeds u32 capacity")]
    ArenaOverflow,
    #[error("node arena index exceeds u32 capacity")]
    ArenaIndexOverflow,
    /// Container references a child absent from the arena.
    #[error("node {parent} references missing child {child}")]
    MissingChild {
        /// The parent node.
        parent: NodeId,
        /// The missing child node.
        child: NodeId,
    },
    /// Live node has no parent-map entry.
    #[error("node {0} has no parent entry")]
    NoParentEntry(NodeId),
    /// Parent ID points to a deallocated arena slot.
    #[error("parent {parent} of node {child} missing from arena")]
    ParentMissing {
        /// The missing parent node.
        parent: NodeId,
        /// The child node.
        child: NodeId,
    },
    /// Parent map and child list disagree.
    #[error("parent_map says {parent} is parent of {child}, but children list disagrees")]
    ParentChildMismatch {
        /// The parent node.
        parent: NodeId,
        /// The child node.
        child: NodeId,
    },
    /// Child appears under two different containers.
    #[error("child {child} appears under multiple containers: {first_parent} and {second_parent}")]
    ChildListedMultipleTimes {
        /// The duplicated child node.
        child: NodeId,
        /// The first container that listed the child.
        first_parent: NodeId,
        /// The second container that listed the child.
        second_parent: NodeId,
    },
    /// Live node unreachable from root.
    #[error("live node {0} is disconnected from the root")]
    DisconnectedNode(NodeId),
    /// Panel kind is an empty string.
    #[error("panel kind must not be empty")]
    EmptyKind,
    #[error("at least one kind required")]
    NoKinds,
    #[error("active index {active} out of bounds for {len} panels")]
    ActiveOutOfBounds {
        /// The active index.
        active: usize,
        /// The number of panels.
        len: usize,
    },
    #[error("dashboard requires at least one card")]
    DashboardNoCards,
    #[error("dashboard columns must be at least 1")]
    DashboardNoColumns,
    #[error("dashboard min_column_width must be positive and finite")]
    DashboardMinWidthInvalid,
    #[error("window size must be at least 1")]
    WindowSizeZero,
    /// Strategy rebuild produced zero nodes.
    #[error("empty after rebuild")]
    EmptyAfterRebuild,
    #[error("no root")]
    NoRoot,
    #[error("no serializable root for snapshot")]
    SnapshotNoRoot,
    /// Focused panel not in the snapshot's panel sequence.
    #[error("snapshot focused panel {0} missing from sequence")]
    SnapshotFocusedMissingFromSequence(PanelId),
    /// Collapsed panel not in the snapshot's panel sequence.
    #[error("snapshot collapsed panel {0} missing from sequence")]
    SnapshotCollapsedMissingFromSequence(PanelId),
    #[error("adaptive layout requires at least one breakpoint")]
    NoBreakpoints,
    /// Duplicate minimum-width value across adaptive breakpoints.
    #[error("adaptive breakpoint width {width}px appears more than once")]
    DuplicateBreakpointWidth {
        /// The duplicated minimum width.
        width: u32,
    },
    #[error("snapshot tree exceeds maximum depth of {0}")]
    SnapshotTooDeep(usize),
    /// Span metadata on a non-panel grid item.
    #[error("snapshot grid item span requires a panel node, got {0}")]
    SnapshotSpanRequiresPanel(&'static str),
    /// Node variant that cannot be serialized (e.g. `TaffyPassthrough`).
    #[error("snapshot capture does not support node {0}")]
    UnsupportedSnapshotNode(NodeId),
    #[error("insert index {index} exceeds container length {len}")]
    InsertOutOfBounds {
        /// The requested index.
        index: usize,
        /// The container's child count.
        len: usize,
    },
    /// Taffy layout engine error during resolve.
    #[error("taffy error: {0}")]
    TaffyError(Box<str>),
    /// TOML node has both `kind` and `type`.
    #[error("node has both 'kind' and 'type'; use one or the other")]
    NodeKindAndType,
    #[error("unknown node type '{0}'; expected 'row' or 'col'")]
    UnknownNodeType(Box<str>),
    #[error("node must have either 'kind' (panel) or 'type' (container)")]
    NodeMissingKindOrType,
}

/// Invalid viewport dimension or scroll offset.
#[derive(Debug, thiserror::Error)]
pub enum ViewportError {
    #[error("dimension is NaN")]
    IsNan,
    #[error("dimension is negative")]
    IsNegative,
    #[error("dimension is infinite")]
    IsInfinite,
    #[error("scroll value is not finite")]
    ScrollNotFinite,
    /// Panel has no saved constraints (never collapsed).
    #[error("no saved constraints for panel {0}")]
    NoSavedConstraints(PanelId),
}

/// A runtime mutation that cannot be applied in the current state.
#[derive(Debug, thiserror::Error)]
pub enum MutationError {
    /// No strategy attached.
    #[error("no strategy set")]
    NoStrategy,
    /// No panel is focused.
    #[error("no focused panel")]
    NoFocusedPanel,
    /// Focused panel is the root (no parent container).
    #[error("focused panel has no parent")]
    FocusedNoParent,
    /// Parent node is a leaf, not a container.
    #[error("parent is not a container")]
    ParentNotContainer,
    /// Panel's parent not found in the tree.
    #[error("panel has no parent")]
    PanelNoParent,
    /// Panel is the sole child of its container.
    #[error("panel is the only child")]
    OnlyChild,
    /// `resize_boundary` requires all siblings to be panel leaves.
    #[error("resize_boundary requires all siblings to be panels")]
    SiblingsNotPanels,
    /// `resize_boundary` requires all siblings to use grow constraints.
    #[error("resize_boundary requires all siblings to use grow constraints")]
    SiblingsNotGrow,
    /// No panels are currently collapsed.
    #[error("no collapsed slots to uncollapse")]
    NoCollapsedSlots,
    /// Collapsed slot lost its saved constraints.
    #[error("slot has no saved constraints")]
    SlotNoSavedConstraints,
    /// Strategy does not support panel reordering.
    #[error("move not supported for this layout")]
    MoveNotSupported,
    /// `set_card_span` requires a dashboard strategy.
    #[error("set_card_span requires a dashboard strategy")]
    SpanNotSupported,
    /// Strategy does not support spatial focus — use `focus_next`/`focus_prev`.
    #[error("spatial navigation not supported — use focus_next/focus_prev")]
    SpatialNavUnsupported,
}

/// Top-level error for all panes operations.
///
/// Wraps [`ConstraintError`], [`TreeError`], [`ViewportError`], and
/// [`MutationError`].
#[derive(Debug, thiserror::Error)]
pub enum PaneError {
    #[error("panel not found: {0}")]
    PanelNotFound(PanelId),

    #[error("invalid constraint: {0}")]
    InvalidConstraint(ConstraintError),

    #[error("node not found: {0}")]
    NodeNotFound(NodeId),

    #[error("tree validation failed: {0}")]
    InvalidTree(TreeError),

    #[error("invalid viewport: {0}")]
    InvalidViewport(ViewportError),

    #[error("invalid mutation: {0}")]
    InvalidMutation(MutationError),

    #[error("sequence index {0} out of bounds for length {1}")]
    SequenceOutOfBounds(usize, usize),
}
