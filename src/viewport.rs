use rustc_hash::{FxHashMap, FxHashSet};

use crate::node::PanelId;
use crate::panel::Constraints;

/// Viewport state for interactive layouts.
#[derive(Debug, Default)]
pub struct ViewportState {
    /// Current horizontal scroll offset.
    pub scroll_offset: f32,
    /// The currently focused panel, if any.
    pub focus: Option<PanelId>,
    /// First visible panel index for Window/ActivePanel strategies.
    pub window_start: usize,
    /// Panels that have been collapsed to zero size.
    pub collapsed: FxHashSet<PanelId>,
    /// Original constraints saved before collapsing.
    pub saved_constraints: FxHashMap<PanelId, Constraints>,
}
