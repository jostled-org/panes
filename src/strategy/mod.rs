mod build;
/// Strategy builder types for decoupling layout shape from panel content.
pub mod builder;
mod dashboard;
mod focus;
mod holy_grail;
mod mutate;
mod sidebar;
mod types;

pub use build::build_initial;
pub(crate) use build::{build_tree_for_strategy, populate_sequence_by_kinds};
pub use builder::{
    ActivePanelStrategy, BinarySplitStrategy, BoundStrategy, CenteredMasterStrategy, DeckStrategy,
    MasterStackStrategy, SplitStrategy, Strategy, WindowStrategy,
};
pub use dashboard::DashboardStrategy;
pub use focus::try_apply_focus;
pub use holy_grail::HolyGrailStrategy;
pub(crate) use mutate::collect_kinds_from_sequence;
pub use mutate::{apply_add, apply_move, apply_remove, apply_set_card_span};
pub use sidebar::SidebarStrategy;
pub use types::GridColumnMode;
pub use types::{ActivePanelVariant, CardSpan, SlotDef, StrategyKind};
