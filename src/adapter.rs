use std::sync::Arc;

use crate::diff::LayoutDiff;
use crate::overlay::{AnchorFailure, OverlayId};
use crate::resolver::ResolvedLayout;
use crate::runtime::{Frame, LayoutRuntime};

/// Shared adapter-frame shell for renderer backends.
///
/// Captures the runtime-vs-stateless duality that every adapter crate needs:
/// runtime frames borrow the [`LayoutRuntime`] for diff access, while stateless
/// frames own a [`ResolvedLayout`] directly. Renderer-specific rect conversion
/// and helpers remain local to each adapter crate.
pub struct AdapterFrame<'a> {
    kind: AdapterFrameKind<'a>,
}

enum AdapterFrameKind<'a> {
    Runtime { frame: Frame, rt: &'a LayoutRuntime },
    Stateless { resolved: ResolvedLayout },
}

impl<'a> AdapterFrame<'a> {
    /// Build from a runtime resolve result.
    pub fn from_runtime(frame: Frame, rt: &'a LayoutRuntime) -> Self {
        Self {
            kind: AdapterFrameKind::Runtime { frame, rt },
        }
    }

    /// Build from a stateless resolve result.
    pub fn from_stateless(resolved: ResolvedLayout) -> Self {
        Self {
            kind: AdapterFrameKind::Stateless { resolved },
        }
    }

    /// Access the resolved layout regardless of variant.
    pub fn resolved(&self) -> &ResolvedLayout {
        match &self.kind {
            AdapterFrameKind::Runtime { frame, .. } => frame.layout(),
            AdapterFrameKind::Stateless { resolved } => resolved,
        }
    }

    /// Layout diff (panel IDs, not rects). `None` for stateless resolves.
    pub fn diff(&self) -> Option<LayoutDiff<'_>> {
        match &self.kind {
            AdapterFrameKind::Runtime { rt, .. } => Some(rt.last_diff()),
            AdapterFrameKind::Stateless { .. } => None,
        }
    }

    /// Raw panes [`Frame`]. `None` for stateless resolves.
    pub fn inner(&self) -> Option<&Frame> {
        match &self.kind {
            AdapterFrameKind::Runtime { frame, .. } => Some(frame),
            AdapterFrameKind::Stateless { .. } => None,
        }
    }

    /// Overlays that failed to anchor during the most recent resolve.
    pub fn overlay_failures(&self) -> &[(OverlayId, Arc<str>, AnchorFailure)] {
        self.resolved().overlay_failures()
    }
}
