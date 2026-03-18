mod build;
/// Strategy builder types for decoupling layout shape from panel content.
pub mod builder;
mod focus;
mod mutate;
mod types;

pub use build::build_initial;
pub(crate) use build::{build_tree_for_strategy, populate_sequence_by_kinds};
#[allow(deprecated)]
pub use builder::{
    ActivePanelStrategy, BinarySplitStrategy, BoundStrategy, CenteredMasterStrategy,
    ColumnGridStrategy, ColumnsStrategy, DashboardStrategy, DeckStrategy, HolyGrailStrategy,
    MasterStackStrategy, SequenceStrategy, SidebarStrategy, SlottedStrategy, SplitStrategy,
    Strategy, WindowStrategy,
};
pub use focus::try_apply_focus;
pub(crate) use mutate::collect_kinds_from_sequence;
pub use mutate::{apply_add, apply_move, apply_remove};
pub(crate) use types::GridColumnMode;
pub use types::{ActivePanelVariant, CardSpan, Direction, SlotDef, StrategyKind};
