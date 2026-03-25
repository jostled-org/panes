use std::sync::Arc;

use crate::error::PaneError;
use crate::layout::Layout;
use crate::runtime::LayoutRuntime;
use crate::tree::LayoutTree;

use super::build::build_tree_for_strategy;
use super::dashboard::DashboardStrategy;
use super::holy_grail::HolyGrailStrategy;
use super::sidebar::SidebarStrategy;
use super::{ActivePanelVariant, Direction, GridColumnMode, StrategyKind};

/// Generate a `build() -> Strategy` method from 1:1 field-to-variant mapping.
macro_rules! impl_build_strategy {
    ($Builder:ty => $Variant:ident { $($field:ident),* }) => {
        impl $Builder {
            /// Convert to a generic [`Strategy`].
            pub fn build(self) -> Strategy {
                Strategy {
                    kind: StrategyKind::$Variant { $($field: self.$field),* },
                }
            }
        }
    };
}

impl_build_strategy!(MasterStackStrategy => MasterStack { master_ratio, gap });
impl_build_strategy!(CenteredMasterStrategy => CenteredMaster { master_ratio, gap });
impl_build_strategy!(DeckStrategy => Deck { master_ratio, gap });
impl_build_strategy!(ActivePanelStrategy => ActivePanel { variant, bar_height });
impl_build_strategy!(WindowStrategy => Window { size, gap });
impl_build_strategy!(BinarySplitStrategy => BinarySplit { spiral, ratio, gap });

/// Generate a `with_panels` shorthand that delegates to `self.build().with_panels(panels)`.
macro_rules! impl_with_panels {
    ($($ty:ty),+ $(,)?) => { $(
        impl $ty {
            /// Bind panels directly.
            pub fn with_panels(
                self,
                panels: impl IntoIterator<Item = impl Into<Arc<str>>>,
            ) -> BoundStrategy {
                self.build().with_panels(panels)
            }
        }
    )+ };
}

impl_with_panels!(
    MasterStackStrategy,
    CenteredMasterStrategy,
    DeckStrategy,
    ActivePanelStrategy,
    WindowStrategy,
    BinarySplitStrategy,
);

/// Generate `From<Builder> for Strategy` impls via `.build()`.
macro_rules! impl_into_strategy {
    ($($ty:ty),+ $(,)?) => { $(
        impl From<$ty> for Strategy {
            fn from(builder: $ty) -> Self {
                builder.build()
            }
        }
    )+ };
}

impl_into_strategy!(
    MasterStackStrategy,
    CenteredMasterStrategy,
    DeckStrategy,
    ActivePanelStrategy,
    WindowStrategy,
    BinarySplitStrategy,
    SplitStrategy,
);

// ---------------------------------------------------------------------------
// Strategy — a configured layout shape, decoupled from panel content.
// ---------------------------------------------------------------------------

/// A configured layout strategy, decoupled from panel content.
/// Clone and reuse across different panel sets.
#[derive(Debug, Clone)]
pub struct Strategy {
    pub(crate) kind: StrategyKind,
}

impl Strategy {
    /// Wrap an existing [`StrategyKind`].
    pub fn from_kind(kind: StrategyKind) -> Self {
        Self { kind }
    }

    /// Access the inner strategy kind.
    pub fn kind(&self) -> &StrategyKind {
        &self.kind
    }

    /// Bind panels to this strategy. Works for all non-dashboard strategies.
    /// Dashboard strategies with spans must use [`DashboardStrategy::with_cards`].
    pub fn with_panels(
        self,
        panels: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> BoundStrategy {
        let panels: Box<[Arc<str>]> = panels.into_iter().map(Into::into).collect();
        BoundStrategy {
            kind: self.kind,
            panels,
            tree_override: None,
        }
    }

    // -- Factory methods --

    /// Master-stack strategy: one master panel with a vertical stack.
    pub fn master_stack() -> MasterStackStrategy {
        MasterStackStrategy {
            master_ratio: 0.5,
            gap: 0.0,
        }
    }

    /// Centered-master strategy: master panel centered between two side stacks.
    pub fn centered_master() -> CenteredMasterStrategy {
        CenteredMasterStrategy {
            master_ratio: 0.5,
            gap: 0.0,
        }
    }

    /// Deck strategy: master panel with one-at-a-time stack.
    pub fn deck() -> DeckStrategy {
        DeckStrategy {
            master_ratio: 0.5,
            gap: 0.0,
        }
    }

    /// Monocle strategy: full-screen single panel.
    pub fn monocle() -> ActivePanelStrategy {
        ActivePanelStrategy {
            variant: ActivePanelVariant::Monocle,
            bar_height: 0.0,
        }
    }

    /// Tabbed strategy: tab bar above content panels.
    pub fn tabbed() -> ActivePanelStrategy {
        ActivePanelStrategy {
            variant: ActivePanelVariant::Tabbed,
            bar_height: 1.0,
        }
    }

    /// Stacked strategy: title bars stacked vertically above content.
    pub fn stacked() -> ActivePanelStrategy {
        ActivePanelStrategy {
            variant: ActivePanelVariant::Stacked,
            bar_height: 1.0,
        }
    }

    /// Scrollable strategy: window showing N adjacent panels.
    pub fn scrollable() -> WindowStrategy {
        WindowStrategy { size: 2, gap: 0.0 }
    }

    /// Dwindle strategy: recursive binary split without spiral.
    pub fn dwindle() -> BinarySplitStrategy {
        BinarySplitStrategy {
            spiral: false,
            ratio: 0.5,
            gap: 0.0,
        }
    }

    /// Spiral strategy: recursive binary split with spiral.
    pub fn spiral() -> BinarySplitStrategy {
        BinarySplitStrategy {
            spiral: true,
            ratio: 0.5,
            gap: 0.0,
        }
    }

    /// Split strategy: two panels with configurable ratio.
    pub fn split() -> SplitStrategy {
        SplitStrategy {
            ratio: 0.5,
            gap: 0.0,
            is_vertical: false,
        }
    }

    /// Dashboard strategy: CSS-grid layout with per-card column spans.
    pub fn dashboard() -> DashboardStrategy {
        DashboardStrategy {
            columns: GridColumnMode::Fixed(4),
            gap: 0.0,
            auto_rows: false,
        }
    }

    /// Sidebar strategy: fixed-width sidebar with grow content.
    pub fn sidebar() -> SidebarStrategy {
        SidebarStrategy::new(0.0, 20.0)
    }

    /// Holy-grail strategy: header, footer, left sidebar, main, right sidebar.
    pub fn holy_grail() -> HolyGrailStrategy {
        HolyGrailStrategy::new(0.0, 20.0, 1.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// BoundStrategy — strategy with panels attached, ready to build.
// ---------------------------------------------------------------------------

/// A strategy with panels bound, ready to produce a layout or runtime.
pub struct BoundStrategy {
    kind: StrategyKind,
    panels: Box<[Arc<str>]>,
    tree_override: Option<Layout>,
}

impl BoundStrategy {
    /// Create a new bound strategy.
    pub(crate) fn new(
        kind: StrategyKind,
        panels: Box<[Arc<str>]>,
        tree_override: Option<Layout>,
    ) -> Self {
        Self {
            kind,
            panels,
            tree_override,
        }
    }

    /// Produce a static [`Layout`] from this bound strategy.
    pub fn build(self) -> Result<Layout, PaneError> {
        match self.tree_override {
            Some(layout) => Ok(layout),
            None => {
                let tree = build_tree_for_strategy(&self.kind, &self.panels)?;
                Ok(Layout::from_tree(tree))
            }
        }
    }

    /// Produce a [`LayoutRuntime`] from this bound strategy.
    pub fn into_runtime(self) -> Result<LayoutRuntime, PaneError> {
        match self.tree_override {
            Some(layout) => {
                let tree = LayoutTree::from(layout);
                Ok(LayoutRuntime::from_tree_and_strategy(
                    tree,
                    self.kind,
                    &self.panels,
                ))
            }
            None => LayoutRuntime::from_strategy(self.kind, &self.panels),
        }
    }
}

// ---------------------------------------------------------------------------
// Builder structs — one per strategy family.
// ---------------------------------------------------------------------------

macro_rules! impl_master_ratio_gap {
    ($($Builder:ident),+) => { $(
        impl $Builder {
            /// Set the master panel's share of the viewport (0.0–1.0).
            pub fn master_ratio(mut self, ratio: f32) -> Self {
                self.master_ratio = ratio;
                self
            }

            /// Set the gap between panels.
            pub fn gap(mut self, gap: f32) -> Self {
                self.gap = gap;
                self
            }
        }
    )+ };
}

/// Builder for [`StrategyKind::MasterStack`].
#[derive(Debug, Clone)]
pub struct MasterStackStrategy {
    master_ratio: f32,
    gap: f32,
}

/// Builder for [`StrategyKind::CenteredMaster`].
#[derive(Debug, Clone)]
pub struct CenteredMasterStrategy {
    master_ratio: f32,
    gap: f32,
}

/// Builder for [`StrategyKind::Deck`].
#[derive(Debug, Clone)]
pub struct DeckStrategy {
    master_ratio: f32,
    gap: f32,
}

impl_master_ratio_gap!(MasterStackStrategy, CenteredMasterStrategy, DeckStrategy);

/// Builder for [`StrategyKind::ActivePanel`] (monocle, tabbed, stacked).
#[derive(Debug, Clone)]
pub struct ActivePanelStrategy {
    variant: ActivePanelVariant,
    bar_height: f32,
}

impl ActivePanelStrategy {
    crate::macros::builder_setters!(
        /// Set the bar height (tab bar or title bar height).
        bar_height(height: f32)
    );
}

/// Builder for [`StrategyKind::Window`] (scrollable).
#[derive(Debug, Clone)]
pub struct WindowStrategy {
    size: usize,
    gap: f32,
}

impl WindowStrategy {
    crate::macros::builder_setters!(
        /// Set how many panels the window shows at once.
        size(size: usize);
        /// Set the gap between visible panels.
        gap(gap: f32)
    );
}

/// Builder for [`StrategyKind::BinarySplit`] (dwindle, spiral).
#[derive(Debug, Clone)]
pub struct BinarySplitStrategy {
    spiral: bool,
    ratio: f32,
    gap: f32,
}

impl BinarySplitStrategy {
    crate::macros::builder_setters!(
        /// Set the split ratio at each level.
        ratio(ratio: f32);
        /// Set the gap between panels.
        gap(gap: f32)
    );
}

/// Builder for split (two panels with configurable ratio and direction).
#[derive(Debug, Clone)]
pub struct SplitStrategy {
    ratio: f32,
    gap: f32,
    is_vertical: bool,
}

impl SplitStrategy {
    crate::macros::builder_setters!(
        /// Set the split ratio.
        ratio(ratio: f32);
        /// Set the gap between panels.
        gap(gap: f32)
    );

    crate::macros::builder_flag_setters!(
        /// Use vertical split direction.
        vertical -> is_vertical = true
    );

    /// Convert to a generic [`Strategy`].
    pub fn build(self) -> Strategy {
        Strategy {
            kind: StrategyKind::Sequence {
                direction: match self.is_vertical {
                    true => Direction::Vertical,
                    false => Direction::Horizontal,
                },
                gap: self.gap,
                ratio: Some(self.ratio),
            },
        }
    }

    /// Bind two named panels directly.
    pub fn with_panels(
        self,
        first: impl Into<Arc<str>>,
        second: impl Into<Arc<str>>,
    ) -> BoundStrategy {
        let panels: Box<[Arc<str>]> = Box::from([first.into(), second.into()]);
        let kind = self.build().kind;
        BoundStrategy {
            kind,
            panels,
            tree_override: None,
        }
    }
}
