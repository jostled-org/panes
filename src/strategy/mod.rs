mod build;
mod focus;
mod mutate;

use std::sync::Arc;

use crate::panel::Constraints;

/// Direction for linear layouts (split, columns).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Direction {
    /// Left-to-right.
    Horizontal,
    /// Top-to-bottom.
    Vertical,
}

/// Sub-variant for single-visible-panel layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ActivePanelVariant {
    /// Full-screen single panel.
    Monocle,
    /// Tab bar above content panels.
    Tabbed,
    /// Title bars stacked vertically above content.
    Stacked,
}

/// Definition of a named slot with fixed or grow constraints.
#[derive(Debug, Clone)]
pub struct SlotDef {
    /// The panel kind occupying this slot.
    pub kind: Arc<str>,
    /// Constraints for this slot when visible.
    pub constraints: Constraints,
}

/// Behavioral strategy for a layout, determining how add/remove/move/focus
/// mutations are applied to the tree.
#[derive(Debug, Clone)]
pub enum StrategyKind {
    /// Linear sequence of equal panels (split, columns).
    Sequence {
        /// Layout direction.
        direction: Direction,
        /// Gap between panels.
        gap: f32,
    },

    /// One master panel with a vertical stack (master-stack).
    MasterStack {
        /// Master panel's share of the viewport (0.0-1.0).
        master_ratio: f32,
        /// Gap between panels.
        gap: f32,
    },

    /// Master panel with a deck of one-at-a-time stack panels (deck).
    Deck {
        /// Master panel's share of the viewport (0.0-1.0).
        master_ratio: f32,
        /// Gap between panels.
        gap: f32,
    },

    /// Master panel centered between two side stacks (centered-master).
    CenteredMaster {
        /// Master panel's share of the viewport (0.0-1.0).
        master_ratio: f32,
        /// Gap between panels.
        gap: f32,
    },

    /// Recursive binary split (dwindle, spiral).
    BinarySplit {
        /// Whether child order reverses on even-depth levels (spiral).
        spiral: bool,
        /// Split ratio at each level.
        ratio: f32,
        /// Gap between panels.
        gap: f32,
    },

    /// Uniform grid of panels (grid).
    ColumnGrid {
        /// Number of columns.
        columns: usize,
        /// Gap between panels.
        gap: f32,
    },

    /// CSS-grid dashboard with per-card column spans (dashboard).
    Dashboard {
        /// Number of columns.
        columns: usize,
        /// Gap between panels.
        gap: f32,
        /// Column span per card, in order.
        spans: Arc<[usize]>,
    },

    /// Only one panel visible at a time (monocle, tabbed, stacked).
    ActivePanel {
        /// Which sub-variant of active-panel layout.
        variant: ActivePanelVariant,
        /// Height of the tab bar (tabbed) or title bars (stacked).
        /// Ignored for monocle.
        bar_height: f32,
    },

    /// Scrollable window showing N adjacent panels (scrollable/NIRI).
    Window {
        /// How many panels the window shows at once.
        size: usize,
        /// Gap between visible panels.
        gap: f32,
    },

    /// Fixed-slot layout with named positions (sidebar, holy-grail).
    Slotted {
        /// Slot definitions in layout order.
        slots: Arc<[SlotDef]>,
        /// Gap between slots.
        gap: f32,
        /// Direction of the outer container.
        direction: Direction,
    },
}

impl StrategyKind {
    /// Gap value for this strategy.
    pub fn gap(&self) -> f32 {
        match self {
            Self::Sequence { gap, .. }
            | Self::MasterStack { gap, .. }
            | Self::Deck { gap, .. }
            | Self::CenteredMaster { gap, .. }
            | Self::BinarySplit { gap, .. }
            | Self::ColumnGrid { gap, .. }
            | Self::Dashboard { gap, .. }
            | Self::Window { gap, .. }
            | Self::Slotted { gap, .. } => *gap,
            Self::ActivePanel { .. } => 0.0,
        }
    }

    /// Whether this strategy supports the move operation.
    pub fn supports_move(&self) -> bool {
        !matches!(self, Self::Slotted { .. })
    }
}

pub use build::build_initial;
pub use focus::try_apply_focus;
pub use mutate::{apply_add, apply_move, apply_remove};
