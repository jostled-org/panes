use rustc_hash::{FxHashMap, FxHashSet};

use crate::node::PanelId;
use crate::panel::Constraints;

/// Viewport state for interactive layouts.
#[derive(Debug, Default)]
pub struct ViewportState {
    pub scroll_offset: f32,
    pub active_panel: Option<PanelId>,
    pub collapsed: FxHashSet<PanelId>,
    pub saved_constraints: FxHashMap<PanelId, Constraints>,
}
