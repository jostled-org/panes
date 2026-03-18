use std::sync::Arc;

use crate::error::PaneError;
use crate::layout::Layout;
use crate::runtime::LayoutRuntime;
use crate::tree::LayoutTree;

use super::build::build_tree_for_strategy;
use super::{ActivePanelVariant, CardSpan, Direction, GridColumnMode, SlotDef, StrategyKind};

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
    DashboardStrategy,
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
        let panels: Vec<Arc<str>> = panels.into_iter().map(Into::into).collect();
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

    /// Columns strategy. Deprecated — use [`Strategy::dashboard`] instead.
    #[deprecated(since = "0.12.0", note = "use Strategy::dashboard() instead")]
    #[allow(deprecated)]
    pub fn columns() -> ColumnsStrategy {
        ColumnsStrategy {
            columns: GridColumnMode::Fixed(0),
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

    /// Grid strategy. Deprecated — use [`Strategy::dashboard`] instead.
    #[deprecated(since = "0.12.0", note = "use Strategy::dashboard() instead")]
    #[allow(deprecated)]
    pub fn grid(columns: usize) -> ColumnGridStrategy {
        ColumnGridStrategy {
            columns: GridColumnMode::Fixed(columns),
            gap: 0.0,
        }
    }

    /// Dashboard strategy: CSS-grid layout with per-card column spans.
    pub fn dashboard() -> DashboardStrategy {
        DashboardStrategy {
            columns: GridColumnMode::Fixed(4),
            gap: 0.0,
        }
    }

    /// Sidebar strategy: fixed-width sidebar with grow content.
    pub fn sidebar() -> SidebarStrategy {
        SidebarStrategy {
            gap: 0.0,
            sidebar_width: 20.0,
        }
    }

    /// Holy-grail strategy: header, footer, left sidebar, main, right sidebar.
    pub fn holy_grail() -> HolyGrailStrategy {
        HolyGrailStrategy {
            gap: 0.0,
            sidebar_width: 20.0,
            header_height: 1.0,
            footer_height: 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// BoundStrategy — strategy with panels attached, ready to build.
// ---------------------------------------------------------------------------

/// A strategy with panels bound, ready to produce a layout or runtime.
pub struct BoundStrategy {
    kind: StrategyKind,
    panels: Vec<Arc<str>>,
    tree_override: Option<Layout>,
}

impl BoundStrategy {
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
    /// Set the bar height (tab bar or title bar height).
    pub fn bar_height(mut self, height: f32) -> Self {
        self.bar_height = height;
        self
    }
}

/// Builder for [`StrategyKind::Window`] (scrollable).
#[derive(Debug, Clone)]
pub struct WindowStrategy {
    size: usize,
    gap: f32,
}

impl WindowStrategy {
    /// Set how many panels the window shows at once.
    pub fn size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    /// Set the gap between visible panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

/// Builder for [`StrategyKind::BinarySplit`] (dwindle, spiral).
#[derive(Debug, Clone)]
pub struct BinarySplitStrategy {
    spiral: bool,
    ratio: f32,
    gap: f32,
}

impl BinarySplitStrategy {
    /// Set the split ratio at each level.
    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio;
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

/// Builder for columns strategy. Deprecated — use [`DashboardStrategy`] instead.
#[deprecated(since = "0.12.0", note = "use Strategy::dashboard() instead")]
#[derive(Debug, Clone)]
pub struct ColumnsStrategy {
    columns: GridColumnMode,
    gap: f32,
}

/// Backward-compatible alias.
#[deprecated(since = "0.12.0", note = "use DashboardStrategy instead")]
#[allow(deprecated)]
pub type SequenceStrategy = ColumnsStrategy;

#[allow(deprecated)]
impl ColumnsStrategy {
    /// Set a fixed number of columns. When 0 (default), uses panel count.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = GridColumnMode::Fixed(columns);
        self
    }

    /// Use responsive `repeat(auto-fill, minmax(min_width, 1fr))` columns.
    pub fn auto_fill(mut self, min_width: f32) -> Self {
        self.columns = GridColumnMode::AutoFill { min_width };
        self
    }

    /// Use responsive `repeat(auto-fit, minmax(min_width, 1fr))` columns.
    pub fn auto_fit(mut self, min_width: f32) -> Self {
        self.columns = GridColumnMode::AutoFit { min_width };
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Convert to a generic [`Strategy`].
    pub fn build(self) -> Strategy {
        self.into_dashboard().build()
    }

    fn into_dashboard(self) -> DashboardStrategy {
        DashboardStrategy {
            columns: self.columns,
            gap: self.gap,
        }
    }

    /// Bind panels directly.
    pub fn with_panels(
        self,
        panels: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> BoundStrategy {
        let panels: Vec<Arc<str>> = panels.into_iter().map(Into::into).collect();
        let resolved = match self.columns {
            GridColumnMode::Fixed(0) => GridColumnMode::Fixed(panels.len()),
            other => other,
        };
        let d = DashboardStrategy {
            columns: resolved,
            gap: self.gap,
        };
        d.with_panels(panels)
    }
}

#[allow(deprecated)]
impl From<ColumnsStrategy> for Strategy {
    fn from(builder: ColumnsStrategy) -> Self {
        builder.build()
    }
}

#[allow(deprecated)]
impl From<ColumnGridStrategy> for Strategy {
    fn from(builder: ColumnGridStrategy) -> Self {
        builder.build()
    }
}

/// Builder for split (two panels with configurable ratio and direction).
#[derive(Debug, Clone)]
pub struct SplitStrategy {
    ratio: f32,
    gap: f32,
    is_vertical: bool,
}

impl SplitStrategy {
    /// Set the split ratio.
    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio;
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Use vertical split direction.
    pub fn vertical(mut self) -> Self {
        self.is_vertical = true;
        self
    }

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
        let panels = vec![first.into(), second.into()];
        let kind = self.build().kind;
        BoundStrategy {
            kind,
            panels,
            tree_override: None,
        }
    }
}

/// Builder for grid strategies. Deprecated — use [`DashboardStrategy`] instead.
#[deprecated(since = "0.12.0", note = "use Strategy::dashboard() instead")]
#[derive(Debug, Clone)]
pub struct ColumnGridStrategy {
    columns: GridColumnMode,
    gap: f32,
}

#[allow(deprecated)]
impl ColumnGridStrategy {
    /// Use responsive `repeat(auto-fill, minmax(min_width, 1fr))` columns.
    pub fn auto_fill(mut self, min_width: f32) -> Self {
        self.columns = GridColumnMode::AutoFill { min_width };
        self
    }

    /// Use responsive `repeat(auto-fit, minmax(min_width, 1fr))` columns.
    pub fn auto_fit(mut self, min_width: f32) -> Self {
        self.columns = GridColumnMode::AutoFit { min_width };
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Convert to a generic [`Strategy`].
    pub fn build(self) -> Strategy {
        self.into_dashboard().build()
    }

    /// Bind panels directly.
    pub fn with_panels(
        self,
        panels: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> BoundStrategy {
        self.into_dashboard().with_panels(panels)
    }

    fn into_dashboard(self) -> DashboardStrategy {
        DashboardStrategy {
            columns: self.columns,
            gap: self.gap,
        }
    }
}

// ---------------------------------------------------------------------------
// Dashboard builder
// ---------------------------------------------------------------------------

/// Builder for dashboard strategies.
#[derive(Debug, Clone)]
pub struct DashboardStrategy {
    columns: GridColumnMode,
    gap: f32,
}

impl DashboardStrategy {
    /// Set a fixed number of columns.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = GridColumnMode::Fixed(columns);
        self
    }

    /// Use responsive `repeat(auto-fill, minmax(min_width, 1fr))` columns.
    pub fn auto_fill(mut self, min_width: f32) -> Self {
        self.columns = GridColumnMode::AutoFill { min_width };
        self
    }

    /// Use responsive `repeat(auto-fit, minmax(min_width, 1fr))` columns.
    pub fn auto_fit(mut self, min_width: f32) -> Self {
        self.columns = GridColumnMode::AutoFit { min_width };
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Bind cards with explicit column spans.
    pub fn with_cards(
        self,
        cards: impl IntoIterator<Item = (impl Into<Arc<str>>, impl Into<CardSpan>)>,
    ) -> BoundStrategy {
        let cards: Vec<(Arc<str>, CardSpan)> = cards
            .into_iter()
            .map(|(k, s)| (k.into(), s.into()))
            .collect();
        let panels: Vec<Arc<str>> = cards.iter().map(|(k, _)| Arc::clone(k)).collect();
        let spans: Arc<[CardSpan]> = cards.iter().map(|(_, s)| *s).collect();
        let kind = self.to_strategy_kind(spans);
        BoundStrategy {
            kind,
            panels,
            tree_override: None,
        }
    }

    /// Bind panels with all spans defaulting to 1.
    pub fn with_panels(
        self,
        panels: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> BoundStrategy {
        let panels: Vec<Arc<str>> = panels.into_iter().map(Into::into).collect();
        let spans: Arc<[CardSpan]> = vec![CardSpan::Columns(1); panels.len()].into();
        let kind = self.to_strategy_kind(spans);
        BoundStrategy {
            kind,
            panels,
            tree_override: None,
        }
    }

    /// Convert to a generic [`Strategy`] with no cards bound yet.
    /// The resulting strategy will have empty spans — bind panels via
    /// [`Strategy::with_panels`] (all spans default to 1).
    pub fn build(self) -> Strategy {
        let spans: Arc<[CardSpan]> = Arc::from([]);
        Strategy {
            kind: self.to_strategy_kind(spans),
        }
    }

    fn to_strategy_kind(&self, spans: Arc<[CardSpan]>) -> StrategyKind {
        match self.columns {
            GridColumnMode::Fixed(columns) => StrategyKind::Dashboard {
                columns,
                gap: self.gap,
                spans,
            },
            GridColumnMode::AutoFill { min_width } => StrategyKind::DashboardAutoFill {
                min_width,
                gap: self.gap,
                spans,
            },
            GridColumnMode::AutoFit { min_width } => StrategyKind::DashboardAutoFit {
                min_width,
                gap: self.gap,
                spans,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Slotted builder (sidebar, holy_grail)
// ---------------------------------------------------------------------------

/// Builder for slotted strategies (sidebar, holy-grail).
#[derive(Debug, Clone)]
pub struct SlottedStrategy {
    gap: f32,
    sidebar_width: f32,
    header_height: f32,
    footer_height: f32,
}

impl SlottedStrategy {
    /// Set the sidebar width.
    pub fn sidebar_width(mut self, width: f32) -> Self {
        self.sidebar_width = width;
        self
    }

    /// Set the header height (holy-grail only).
    pub fn header_height(mut self, height: f32) -> Self {
        self.header_height = height;
        self
    }

    /// Set the footer height (holy-grail only).
    pub fn footer_height(mut self, height: f32) -> Self {
        self.footer_height = height;
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

// ---------------------------------------------------------------------------
// SidebarStrategy — focused builder for sidebar layouts.
// ---------------------------------------------------------------------------

/// Builder for sidebar strategy: fixed-width sidebar + grow content.
#[derive(Debug, Clone)]
pub struct SidebarStrategy {
    gap: f32,
    sidebar_width: f32,
}

impl SidebarStrategy {
    /// Set the sidebar width.
    pub fn sidebar_width(mut self, width: f32) -> Self {
        self.sidebar_width = width;
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Bind sidebar panels: fixed-width sidebar + grow content.
    pub fn with_panels(
        self,
        sidebar: impl Into<Arc<str>>,
        content: impl Into<Arc<str>>,
    ) -> BoundStrategy {
        let sidebar: Arc<str> = sidebar.into();
        let content: Arc<str> = content.into();
        let slots: Arc<[SlotDef]> = vec![
            SlotDef {
                kind: Arc::clone(&sidebar),
                constraints: crate::panel::fixed(self.sidebar_width),
            },
            SlotDef {
                kind: Arc::clone(&content),
                constraints: crate::panel::grow(1.0),
            },
        ]
        .into();
        let kind = StrategyKind::Slotted {
            slots,
            gap: self.gap,
            direction: Direction::Horizontal,
        };
        BoundStrategy {
            kind,
            panels: vec![sidebar, content],
            tree_override: None,
        }
    }
}

// ---------------------------------------------------------------------------
// HolyGrailStrategy — focused builder for holy-grail layouts.
// ---------------------------------------------------------------------------

/// Builder for holy-grail strategy: header, footer, left sidebar, main, right sidebar.
#[derive(Debug, Clone)]
pub struct HolyGrailStrategy {
    gap: f32,
    sidebar_width: f32,
    header_height: f32,
    footer_height: f32,
}

impl HolyGrailStrategy {
    /// Set the sidebar width.
    pub fn sidebar_width(mut self, width: f32) -> Self {
        self.sidebar_width = width;
        self
    }

    /// Set the header height.
    pub fn header_height(mut self, height: f32) -> Self {
        self.header_height = height;
        self
    }

    /// Set the footer height.
    pub fn footer_height(mut self, height: f32) -> Self {
        self.footer_height = height;
        self
    }

    /// Set the gap between panels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Bind holy-grail panels: header, footer, left sidebar, main, right sidebar.
    pub fn with_panels(
        self,
        header: impl Into<Arc<str>>,
        footer: impl Into<Arc<str>>,
        left: impl Into<Arc<str>>,
        main: impl Into<Arc<str>>,
        right: impl Into<Arc<str>>,
    ) -> Result<BoundStrategy, PaneError> {
        let header: Arc<str> = header.into();
        let footer: Arc<str> = footer.into();
        let left: Arc<str> = left.into();
        let main_kind: Arc<str> = main.into();
        let right: Arc<str> = right.into();

        let slots: Arc<[SlotDef]> = vec![
            SlotDef {
                kind: Arc::clone(&header),
                constraints: crate::panel::fixed(self.header_height),
            },
            SlotDef {
                kind: Arc::clone(&left),
                constraints: crate::panel::fixed(self.sidebar_width),
            },
            SlotDef {
                kind: Arc::clone(&main_kind),
                constraints: crate::panel::grow(1.0),
            },
            SlotDef {
                kind: Arc::clone(&right),
                constraints: crate::panel::fixed(self.sidebar_width),
            },
            SlotDef {
                kind: Arc::clone(&footer),
                constraints: crate::panel::fixed(self.footer_height),
            },
        ]
        .into();
        let kind = StrategyKind::Slotted {
            slots,
            gap: self.gap,
            direction: Direction::Vertical,
        };

        let layout = crate::preset::HolyGrail::new(
            Arc::clone(&header),
            Arc::clone(&footer),
            Arc::clone(&left),
            Arc::clone(&main_kind),
            Arc::clone(&right),
        )
        .header_height(self.header_height)
        .footer_height(self.footer_height)
        .sidebar_width(self.sidebar_width)
        .gap(self.gap)
        .build()?;

        let panels = vec![header, left, main_kind, right, footer];
        Ok(BoundStrategy {
            kind,
            panels,
            tree_override: Some(layout),
        })
    }
}

impl SlottedStrategy {
    /// Bind sidebar panels: fixed-width sidebar + grow content.
    pub fn with_sidebar_panels(
        self,
        sidebar: impl Into<Arc<str>>,
        content: impl Into<Arc<str>>,
    ) -> BoundStrategy {
        SidebarStrategy {
            gap: self.gap,
            sidebar_width: self.sidebar_width,
        }
        .with_panels(sidebar, content)
    }

    /// Bind holy-grail panels: header, footer, left sidebar, main, right sidebar.
    pub fn with_holy_grail_panels(
        self,
        header: impl Into<Arc<str>>,
        footer: impl Into<Arc<str>>,
        left: impl Into<Arc<str>>,
        main: impl Into<Arc<str>>,
        right: impl Into<Arc<str>>,
    ) -> Result<BoundStrategy, PaneError> {
        HolyGrailStrategy {
            gap: self.gap,
            sidebar_width: self.sidebar_width,
            header_height: self.header_height,
            footer_height: self.footer_height,
        }
        .with_panels(header, footer, left, main, right)
    }
}
