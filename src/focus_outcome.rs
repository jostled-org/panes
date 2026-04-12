/// Result of a focus operation on [`LayoutRuntime`](crate::runtime::LayoutRuntime).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusOutcome {
    /// Focus moved to a new panel.
    Applied,
    /// The target panel was already focused; no change.
    Unchanged,
    /// Focus was rejected for the given reason.
    Rejected(FocusRejection),
}

/// Reason a focus request was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusRejection {
    /// The panel does not exist in the tree or sequence.
    PanelNotFound,
    /// The strategy rejected the focus request (e.g. panel missing from tree
    /// after constraint application).
    StrategyRejected,
}

impl FocusOutcome {
    /// Whether focus was applied (moved to a new panel).
    pub fn is_applied(self) -> bool {
        matches!(self, Self::Applied)
    }

    /// Whether focus is on the target panel (either applied or was already there).
    pub fn is_on_target(self) -> bool {
        matches!(self, Self::Applied | Self::Unchanged)
    }
}
