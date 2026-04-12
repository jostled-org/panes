mod active_panel;
pub(crate) mod catalog;
mod centered_master;
mod common;
mod dashboard;
mod deck;
mod dwindle;
mod holy_grail;
mod master_stack;
mod monocle;
mod scrollable;
mod sidebar;
mod spiral;
mod split;
mod stacked;
mod tabbed;

pub use active_panel::ActivePanelPreset;
pub use centered_master::CenteredMaster;
pub use dashboard::Dashboard;
pub use deck::Deck;
pub use dwindle::Dwindle;
pub use holy_grail::HolyGrail;
pub use master_stack::MasterStack;
pub use monocle::Monocle;
pub use scrollable::Scrollable;
pub use sidebar::Sidebar;
pub use spiral::Spiral;
pub use split::Split;
pub use stacked::Stacked;
pub use tabbed::Tabbed;

pub use catalog::{PanelInputKind, PresetInfo};

pub(crate) use common::collect_kinds;
pub(crate) use common::impl_preset;
pub(super) use common::{add_active_hidden_panels, add_panels, build_single, col_style, row_style};
pub(crate) use common::{
    grid_item_style, simple_grid_style, validate_grid_columns, validate_grid_span,
};
pub(crate) use common::{
    validate_active, validate_f32_param, validate_kinds, validate_share_param,
};
