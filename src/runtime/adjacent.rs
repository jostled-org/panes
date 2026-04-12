use std::sync::Arc;

use super::placement::Placement;
use super::types::{LayoutRuntime, strategy_ref};
use crate::error::{MutationError, PaneError};
use crate::node::{Node, NodeId, PanelId};
use crate::panel::Axis;
use crate::tree::LayoutTree;

impl LayoutRuntime {
    /// Pick a split direction from the focused panel's aspect ratio.
    /// Splits the longer axis: wider -> horizontal, taller -> vertical.
    /// Falls back to horizontal if no layout is cached or no panel is focused.
    pub(crate) fn auto_axis(&self) -> Axis {
        let rect = self
            .viewport
            .focus
            .and_then(|pid| self.previous.as_ref()?.get(pid));
        match rect {
            Some(r) if r.h > r.w => Axis::Col,
            _ => Axis::Row,
        }
    }

    /// Add a panel adjacent to the currently focused panel with full control.
    ///
    /// With a strategy: delegates to strategy rebuild (axis/constraints
    /// ignored, placement controls sequence position).
    ///
    /// Without a strategy: works directly on tree topology. If `axis`
    /// matches the parent container's axis, the new panel is inserted as a
    /// sibling. If it conflicts, the focused panel is wrapped in a new
    /// sub-container.
    pub fn add_panel_adjacent_with(
        &mut self,
        kind: Arc<str>,
        axis: Axis,
        constraints: crate::Constraints,
        placement: Placement,
    ) -> Result<PanelId, PaneError> {
        let pid = match strategy_ref(&self.strategy_source, &self.breakpoints, self.active_bp_idx) {
            Some(strategy) => {
                let index = self.placement_to_index(placement);
                crate::strategy::apply_add(
                    strategy,
                    &mut self.tree,
                    &mut self.sequence,
                    &mut self.viewport,
                    kind,
                    index,
                )
            }
            None => self.add_panel_adjacent_no_strategy(kind, axis, constraints, placement),
        };
        self.invalidate_topology();
        pid
    }

    fn add_panel_adjacent_no_strategy(
        &mut self,
        kind: Arc<str>,
        axis: Axis,
        constraints: crate::Constraints,
        placement: Placement,
    ) -> Result<PanelId, PaneError> {
        let focused = self
            .focused()
            .ok_or(PaneError::InvalidMutation(MutationError::NoFocusedPanel))?;
        let (focused_nid, parent_id, focused_idx, parent_axis) =
            find_focused_position(&self.tree, focused)?;

        let (new_pid, new_nid) = self.tree.add_panel(kind, constraints)?;

        let insert_result = match (parent_axis == axis, placement) {
            (true, Placement::Before) => self.tree.insert_child_at(parent_id, focused_idx, new_nid),
            (true, Placement::After | Placement::End) => {
                self.tree
                    .insert_child_at(parent_id, focused_idx + 1, new_nid)
            }
            (false, Placement::End | Placement::After) => wrap_in_container(
                &mut self.tree,
                parent_id,
                focused_nid,
                focused_idx,
                new_nid,
                axis,
                Placement::After,
            ),
            (false, Placement::Before) => wrap_in_container(
                &mut self.tree,
                parent_id,
                focused_nid,
                focused_idx,
                new_nid,
                axis,
                Placement::Before,
            ),
        };

        if let Err(error) = insert_result {
            self.tree.remove_panel(new_pid)?;
            return Err(error);
        }

        let seq_idx = match (self.sequence.index_of(focused), placement) {
            (Some(idx), Placement::Before) => idx,
            (Some(idx), Placement::After) => idx + 1,
            (_, Placement::End) | (None, _) => self.sequence.len(),
        };
        self.sequence.insert(seq_idx, new_pid);

        self.viewport.focus = Some(new_pid);
        Ok(new_pid)
    }
}

fn find_focused_position(
    tree: &LayoutTree,
    focused: PanelId,
) -> Result<(NodeId, NodeId, usize, Axis), PaneError> {
    let focused_nid = tree
        .node_for_panel(focused)
        .ok_or(PaneError::PanelNotFound(focused))?;
    let parent_id = tree
        .parent(focused_nid)?
        .ok_or(PaneError::InvalidMutation(MutationError::FocusedNoParent))?;
    let parent_axis = parent_axis(tree, parent_id)?;
    let focused_idx = tree
        .children(parent_id)?
        .iter()
        .position(|&c| c == focused_nid)
        .ok_or(PaneError::PanelNotFound(focused))?;
    Ok((focused_nid, parent_id, focused_idx, parent_axis))
}

fn parent_axis(tree: &LayoutTree, parent_id: NodeId) -> Result<Axis, PaneError> {
    let node = tree.node(parent_id).ok_or(PaneError::InvalidMutation(
        MutationError::ParentNotContainer,
    ))?;
    match node {
        Node::Panel { .. } => Err(PaneError::InvalidMutation(
            MutationError::ParentNotContainer,
        )),
        _ => Ok(crate::compiler::axis_of(node)),
    }
}

fn wrap_in_container(
    tree: &mut LayoutTree,
    parent_id: NodeId,
    focused_nid: NodeId,
    focused_idx: usize,
    new_nid: NodeId,
    axis: Axis,
    placement: Placement,
) -> Result<(), PaneError> {
    tree.detach(focused_nid);
    let children = match placement {
        Placement::Before => vec![new_nid, focused_nid],
        Placement::After | Placement::End => vec![focused_nid, new_nid],
    };
    let c = match axis {
        Axis::Row => tree.add_row(0.0, children)?,
        Axis::Col => tree.add_col(0.0, children)?,
    };
    if let Err(error) = tree.insert_child_at(parent_id, focused_idx, c) {
        rollback_wrapped_container(tree, parent_id, focused_idx, focused_nid, c)?;
        return Err(error);
    }

    Ok(())
}

fn rollback_wrapped_container(
    tree: &mut LayoutTree,
    parent_id: NodeId,
    focused_idx: usize,
    focused_nid: NodeId,
    container_id: NodeId,
) -> Result<(), PaneError> {
    tree.remove_orphan_node(container_id)?;
    let focused_still_attached = tree.children(parent_id)?.get(focused_idx).copied();
    match focused_still_attached == Some(focused_nid) {
        true => {
            tree.restore_parent(focused_nid, parent_id);
            Ok(())
        }
        false => tree.insert_child_at(parent_id, focused_idx, focused_nid),
    }
}
