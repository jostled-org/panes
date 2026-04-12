use std::sync::Arc;

/// The role a decoration panel plays in the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecorationRole {
    /// Tab-bar chrome for a tabbed preset.
    Tab,
    /// Title-bar chrome for a stacked preset.
    Title,
}

/// Metadata linking a decoration panel to the content panel it decorates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DecorationMeta {
    pub role: DecorationRole,
    pub content_kind: Arc<str>,
}
