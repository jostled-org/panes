use crate::error::{ConstraintError, MutationError, PaneError};
use crate::node::{Node, NodeId, PanelId};
use crate::panel::Constraints;
use crate::tree::LayoutTree;

pub(crate) fn resize_boundary(
    tree: &mut LayoutTree,
    pid: PanelId,
    delta: f32,
) -> Result<(), PaneError> {
    validate_delta(delta)?;
    if delta.abs() < f32::EPSILON {
        return Ok(());
    }

    let parent_nid = resolve_parent(tree, pid)?;
    let siblings = collect_grow_siblings(tree, parent_nid)?;
    let new_weights = redistribute_grow(pid, delta, &siblings)?;

    for (sibling, new_grow) in siblings.iter().zip(new_weights) {
        let updated = Constraints {
            grow: Some(new_grow),
            min: sibling.constraints.min,
            max: sibling.constraints.max,
            ..Constraints::default()
        };
        tree.set_constraints(sibling.pid, updated)?;
    }

    Ok(())
}

fn validate_delta(delta: f32) -> Result<(), PaneError> {
    match delta.is_finite() {
        true => Ok(()),
        false => Err(PaneError::InvalidConstraint(
            ConstraintError::DeltaNotFinite,
        )),
    }
}

fn resolve_parent(tree: &LayoutTree, pid: PanelId) -> Result<NodeId, PaneError> {
    let nid = tree
        .node_for_panel(pid)
        .ok_or(PaneError::PanelNotFound(pid))?;
    let parent_nid = tree
        .parent(nid)?
        .ok_or(PaneError::InvalidMutation(MutationError::PanelNoParent))?;
    match tree.children(parent_nid)?.len() < 2 {
        true => Err(PaneError::InvalidMutation(MutationError::OnlyChild)),
        false => Ok(parent_nid),
    }
}

fn redistribute_grow(
    target_pid: PanelId,
    delta: f32,
    siblings: &[SiblingInfo],
) -> Result<Vec<f32>, PaneError> {
    const EPSILON: f32 = 0.001;

    let total_grow: f32 = siblings.iter().map(|s| s.grow).sum();
    let target = siblings
        .iter()
        .find(|s| s.pid == target_pid)
        .ok_or(PaneError::PanelNotFound(target_pid))?;
    let current_share = target.grow / total_grow;

    // Panel already dominates — no room to redistribute.
    match current_share >= 1.0 - EPSILON {
        true => return Ok(siblings.iter().map(|s| s.grow).collect()),
        false => {}
    }

    let max_share = 1.0 - EPSILON * (siblings.len() - 1) as f32;
    let new_share = (current_share + delta).clamp(EPSILON, max_share);
    let scale = (1.0 - new_share) / (1.0 - current_share);

    Ok(siblings
        .iter()
        .map(|s| match s.pid == target_pid {
            true => new_share * total_grow,
            false => (s.grow * scale).max(EPSILON),
        })
        .collect())
}

struct SiblingInfo {
    pid: PanelId,
    grow: f32,
    constraints: Constraints,
}

fn collect_grow_siblings(
    tree: &LayoutTree,
    parent_nid: NodeId,
) -> Result<Vec<SiblingInfo>, PaneError> {
    let children = tree.children(parent_nid)?;
    let mut siblings = Vec::with_capacity(children.len());
    for &child_nid in children {
        let Some(Node::Panel {
            id, constraints, ..
        }) = tree.node(child_nid)
        else {
            return Err(PaneError::InvalidMutation(MutationError::SiblingsNotPanels));
        };
        let grow = match (constraints.grow, constraints.fixed) {
            (Some(g), _) => g,
            (None, None) => 1.0,
            (None, Some(_)) => {
                return Err(PaneError::InvalidMutation(MutationError::SiblingsNotGrow));
            }
        };
        siblings.push(SiblingInfo {
            pid: *id,
            grow,
            constraints: *constraints,
        });
    }
    Ok(siblings)
}
