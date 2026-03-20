use std::sync::Arc;

use super::types::{LayoutRuntime, StrategySource, base};
use crate::breakpoint::BreakpointEntry;
use crate::error::{PaneError, TreeError};
use crate::layout::Layout;
use crate::node::{Node, NodeId};
use crate::overlay::{self, OverlayDef};
use crate::sequence::PanelSequence;
use crate::snapshot::{self, LayoutSnapshot, SnapshotSource};
use crate::strategy::StrategyKind;
use crate::tree::LayoutTree;
use crate::viewport::ViewportState;

impl LayoutRuntime {
    /// Create a runtime with direct tree control (no strategy).
    ///
    /// Panels are added via hyprland-style splits: `add_panel` auto-picks
    /// the split direction from the focused panel's aspect ratio.
    /// Use `add_panel_adjacent_with` for explicit direction and constraints,
    /// or `tree_mut()` for direct tree surgery.
    ///
    /// For strategy-managed layouts (presets), use
    /// [`from_strategy`](Self::from_strategy) or a preset's `into_runtime()`.
    pub fn new(tree: LayoutTree) -> Self {
        base(
            tree,
            ViewportState::default(),
            StrategySource::None,
            PanelSequence::default(),
        )
    }

    /// Create a runtime from a strategy and initial panel kinds.
    pub fn from_strategy(strategy: StrategyKind, kinds: &[Arc<str>]) -> Result<Self, PaneError> {
        let mut sequence = PanelSequence::default();
        let mut viewport = ViewportState::default();
        let tree = crate::strategy::build_initial(&strategy, kinds, &mut sequence, &mut viewport)?;
        Ok(base(
            tree,
            viewport,
            StrategySource::Standalone(strategy),
            sequence,
        ))
    }

    /// Create a runtime with direct tree control and a pre-populated sequence.
    ///
    /// Same as [`new`](Self::new) but with an explicit panel sequence for
    /// `focus_next`/`focus_prev` ordering. Use this when you build the tree
    /// yourself and want sequence-aware focus navigation.
    pub fn from_tree_and_sequence(tree: LayoutTree, sequence: PanelSequence) -> Self {
        let focus = sequence.get(0);
        base(
            tree,
            ViewportState {
                focus,
                ..ViewportState::default()
            },
            StrategySource::None,
            sequence,
        )
    }

    /// Create a runtime from a pre-built tree and a strategy.
    /// Populates the sequence by looking up each kind in the tree.
    pub fn from_tree_and_strategy(
        tree: LayoutTree,
        strategy: StrategyKind,
        kinds: &[Arc<str>],
    ) -> Self {
        let mut sequence = PanelSequence::default();
        crate::strategy::populate_sequence_by_kinds(&tree, kinds, &mut sequence);
        let focus = sequence.get(0);
        base(
            tree,
            ViewportState {
                focus,
                ..ViewportState::default()
            },
            StrategySource::Standalone(strategy),
            sequence,
        )
    }

    /// Create a runtime from an adaptive layout with breakpoints.
    ///
    /// The active strategy is borrowed from `breakpoints[active_idx]` — no clone.
    pub(crate) fn from_adaptive(
        kinds: &[Arc<str>],
        breakpoints: Box<[BreakpointEntry]>,
        active_idx: usize,
    ) -> Result<Self, PaneError> {
        let strategy = &breakpoints[active_idx].strategy;
        let mut sequence = PanelSequence::default();
        let mut viewport = ViewportState::default();
        let tree = crate::strategy::build_initial(strategy, kinds, &mut sequence, &mut viewport)?;
        let mut rt = base(tree, viewport, StrategySource::Adaptive, sequence);
        rt.breakpoints = Some(breakpoints);
        rt.active_bp_idx = active_idx;
        Ok(rt)
    }

    /// Capture a serializable snapshot of the current runtime state.
    ///
    /// Strategy runtimes snapshot the recipe (strategy config + panel kinds).
    /// Non-strategy runtimes snapshot the tree topology.
    ///
    /// `TaffyPassthrough` nodes are not serializable and are omitted from tree
    /// snapshots. Returns `SnapshotNoRoot` if the root itself is a passthrough.
    pub fn snapshot(&self) -> Result<LayoutSnapshot, PaneError> {
        let bp_info = self
            .breakpoints
            .as_deref()
            .map(|bps| (bps, self.active_bp_idx));
        snapshot::capture(
            &self.tree,
            self.strategy(),
            &self.sequence,
            &self.viewport,
            &self.overlays,
            bp_info,
        )
    }

    /// Restore a runtime from a snapshot.
    ///
    /// Strategy snapshots rebuild through the preset builder.
    /// Tree snapshots rebuild via the layout builder.
    pub fn from_snapshot(snap: LayoutSnapshot) -> Result<Self, PaneError> {
        let mut rt = match snap.source() {
            SnapshotSource::Strategy { strategy, panels } => {
                let sk = StrategyKind::from(strategy);
                let kinds: Box<[Arc<str>]> = panels.iter().map(|s| Arc::from(&**s)).collect();
                Self::from_strategy(sk, &kinds)?
            }
            SnapshotSource::Tree { root } => {
                let tree = snapshot::snapshot_to_tree(root)?;
                let mut seq = PanelSequence::default();
                collect_panels_depth_first(&tree, &mut seq);
                Self::from_tree_and_sequence(tree, seq)
            }
            SnapshotSource::Adaptive { breakpoints, .. } if breakpoints.is_empty() => {
                return Err(PaneError::InvalidTree(TreeError::NoBreakpoints));
            }
            SnapshotSource::Adaptive {
                breakpoints,
                panels,
                active_index,
            } => {
                let kinds: Box<[Arc<str>]> = panels.iter().map(|s| Arc::from(&**s)).collect();
                let bp_entries: Box<[BreakpointEntry]> = breakpoints
                    .iter()
                    .map(|sb| BreakpointEntry {
                        min_width: sb.min_width,
                        strategy: StrategyKind::from(&sb.strategy),
                    })
                    .collect::<Vec<_>>()
                    .into();
                let active = (*active_index).min(bp_entries.len().saturating_sub(1));
                Self::from_adaptive(&kinds, bp_entries, active)?
            }
        };

        // Restore focus by kind — best-effort: the panel exists in the tree
        // via panels_by_kind, so focus() should always succeed.
        if let Some(&pid) = snap
            .focused()
            .and_then(|kind| rt.tree.panels_by_kind(kind).first())
        {
            let _ = rt.focus(pid);
        }

        // Restore collapsed by kind
        for kind in snap.collapsed() {
            if let Some(&pid) = rt.tree.panels_by_kind(kind).first() {
                rt.toggle_collapsed(pid)?;
            }
        }

        restore_overlays(&mut rt, snap.into_overlays())?;

        Ok(rt)
    }
}

impl From<LayoutTree> for LayoutRuntime {
    fn from(tree: LayoutTree) -> Self {
        Self::new(tree)
    }
}

impl From<Layout> for LayoutRuntime {
    fn from(layout: Layout) -> Self {
        Self::new(LayoutTree::from(layout))
    }
}

/// Restore overlay definitions from snapshot data.
fn restore_overlays(
    rt: &mut LayoutRuntime,
    overlays: Vec<overlay::SnapshotOverlay>,
) -> Result<(), PaneError> {
    for snap_overlay in overlays {
        let kind: Arc<str> = Arc::from(snap_overlay.kind);
        let id = rt.overlay_gen.next_id()?;
        let def = OverlayDef {
            id,
            kind: Arc::clone(&kind),
            anchor: snap_overlay.anchor,
            width: snap_overlay.width,
            height: snap_overlay.height,
            visible: snap_overlay.visible,
        };
        let idx = rt.overlays.len();
        rt.overlays.push(def);
        rt.overlay_index.insert(kind, idx);
    }
    Ok(())
}

/// Collect all panel IDs from the tree in depth-first order.
fn collect_panels_depth_first(tree: &LayoutTree, seq: &mut PanelSequence) {
    let Some(root) = tree.root() else { return };
    collect_panels_recursive(tree, root, seq);
}

fn collect_panels_recursive(tree: &LayoutTree, nid: NodeId, seq: &mut PanelSequence) {
    let Some(node) = tree.node(nid) else { return };
    match node {
        Node::Panel { id, .. } => seq.push(*id),
        _ => {
            for &child in node.children() {
                collect_panels_recursive(tree, child, seq);
            }
        }
    }
}
