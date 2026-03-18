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

/// Column span for a dashboard card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CardSpan {
    /// Span a fixed number of columns.
    Columns(usize),
    /// Span all columns regardless of how many the viewport produces.
    /// Emits `grid-column: 1 / -1` in CSS.
    FullWidth,
}

impl From<usize> for CardSpan {
    fn from(n: usize) -> Self {
        Self::Columns(n)
    }
}

/// Column mode for CSS Grid-based presets.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) enum GridColumnMode {
    /// Fixed number of equal-width columns.
    Fixed(usize),
    /// Responsive columns via `repeat(auto-fill, minmax(min_width, 1fr))`.
    AutoFill {
        /// Minimum column width in pixels.
        min_width: f32,
    },
    /// Responsive columns via `repeat(auto-fit, minmax(min_width, 1fr))`.
    AutoFit {
        /// Minimum column width in pixels.
        min_width: f32,
    },
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
        /// When `Some(r)` and exactly 2 panels, applies `grow(r)` / `grow(1-r)`
        /// instead of equal sizing. Used by the split preset/strategy.
        ratio: Option<f32>,
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

    /// CSS-grid dashboard with per-card column spans (dashboard, grid, columns).
    Dashboard {
        /// Fixed number of columns.
        columns: usize,
        /// Gap between panels.
        gap: f32,
        /// Column span per card, in order.
        spans: Arc<[CardSpan]>,
    },

    /// Dashboard with responsive auto-fill columns.
    DashboardAutoFill {
        /// Minimum column width in pixels.
        min_width: f32,
        /// Gap between panels.
        gap: f32,
        /// Column span per card, in order.
        spans: Arc<[CardSpan]>,
    },

    /// Dashboard with responsive auto-fit columns.
    DashboardAutoFit {
        /// Minimum column width in pixels.
        min_width: f32,
        /// Gap between panels.
        gap: f32,
        /// Column span per card, in order.
        spans: Arc<[CardSpan]>,
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
            | Self::Dashboard { gap, .. }
            | Self::DashboardAutoFill { gap, .. }
            | Self::DashboardAutoFit { gap, .. }
            | Self::Window { gap, .. }
            | Self::Slotted { gap, .. } => *gap,
            Self::ActivePanel { .. } => 0.0,
        }
    }

    /// Whether this strategy supports the move operation.
    pub fn supports_move(&self) -> bool {
        !matches!(self, Self::Slotted { .. })
    }

    /// Whether this strategy supports spatial focus navigation.
    ///
    /// Strategies that hide most panels (ActivePanel, Window) don't
    /// produce meaningful spatial relationships — use `focus_next`/`focus_prev`.
    pub fn supports_spatial_nav(&self) -> bool {
        !matches!(self, Self::ActivePanel { .. } | Self::Window { .. })
    }
}
