use std::sync::Arc;

use crate::resolver::ResolvedLayout;

/// Result of a single resolve call: the resolved layout for this frame.
#[derive(Clone)]
pub struct Frame {
    layout: Arc<ResolvedLayout>,
}

impl Frame {
    /// Create a new frame from a resolved layout.
    pub(crate) fn new(layout: Arc<ResolvedLayout>) -> Self {
        Self { layout }
    }

    /// The resolved layout for this frame.
    pub fn layout(&self) -> &ResolvedLayout {
        &self.layout
    }

    /// Cheap shared reference to the resolved layout.
    pub fn arc(&self) -> Arc<ResolvedLayout> {
        Arc::clone(&self.layout)
    }
}
