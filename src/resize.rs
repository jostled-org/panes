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

    let panel_nid = tree
        .node_for_panel(pid)
        .ok_or(PaneError::PanelNotFound(pid))?;
    let parent_nid = resolve_parent(tree, panel_nid)?;

    // Escalate when parent is a TaffyPassthrough whose axis differs from grandparent.
    let (target_nid, siblings) = match should_escalate(tree, parent_nid)? {
        Some((container_nid, grandparent_nid)) => {
            (container_nid, collect_siblings(tree, grandparent_nid)?)
        }
        None => (panel_nid, collect_siblings(tree, parent_nid)?),
    };

    match siblings.iter().any(|s| s.nid == target_nid) {
        true => {}
        false => {
            return Err(PaneError::InvalidMutation(MutationError::SiblingsNotGrow));
        }
    }
    match siblings.len() >= 2 {
        true => {}
        false => {
            return Err(PaneError::InvalidMutation(MutationError::OnlyChild));
        }
    }

    match classify_resize(&siblings) {
        ResizeMode::PureGrow => {
            let new_weights = redistribute_grow(target_nid, delta, &siblings)?;
            apply_grow_weights(tree, &siblings, new_weights)?;
        }
        ResizeMode::FixedAdjust => {
            adjust_fixed(tree, target_nid, delta, &siblings)?;
        }
    }

    Ok(())
}

/// Check if a resize should escalate from a TaffyPassthrough parent to its grandparent.
/// Returns `Some((container_nid, grandparent_nid))` when the container's internal axis
/// differs from the grandparent's axis, meaning the resize crosses the container boundary.
fn should_escalate(
    tree: &LayoutTree,
    parent_nid: NodeId,
) -> Result<Option<(NodeId, NodeId)>, PaneError> {
    let parent_node = tree
        .node(parent_nid)
        .ok_or(PaneError::NodeNotFound(parent_nid))?;
    let parent_horizontal = match parent_node {
        Node::TaffyPassthrough { style, .. } => matches!(
            style.flex_direction,
            taffy::FlexDirection::Row | taffy::FlexDirection::RowReverse
        ),
        _ => return Ok(None),
    };

    let grandparent_nid = match tree.parent(parent_nid)? {
        Some(gp) => gp,
        None => return Ok(None),
    };

    let grandparent_node = tree
        .node(grandparent_nid)
        .ok_or(PaneError::NodeNotFound(grandparent_nid))?;
    let gp_horizontal = match grandparent_node {
        Node::Row { .. } => true,
        Node::Col { .. } => false,
        _ => return Ok(None),
    };

    match parent_horizontal == gp_horizontal {
        true => Ok(None),
        false => Ok(Some((parent_nid, grandparent_nid))),
    }
}

fn validate_delta(delta: f32) -> Result<(), PaneError> {
    match delta.is_finite() {
        true => Ok(()),
        false => Err(PaneError::InvalidConstraint(
            ConstraintError::DeltaNotFinite,
        )),
    }
}

fn resolve_parent(tree: &LayoutTree, nid: NodeId) -> Result<NodeId, PaneError> {
    tree.parent(nid)?
        .ok_or(PaneError::InvalidMutation(MutationError::PanelNoParent))
}

fn apply_grow_weights(
    tree: &mut LayoutTree,
    siblings: &[SiblingInfo],
    weights: Vec<f32>,
) -> Result<(), PaneError> {
    for (sibling, new_grow) in siblings.iter().zip(weights) {
        match &sibling.kind {
            SiblingKind::Panel { pid, constraints } => {
                let updated = Constraints {
                    grow: Some(new_grow),
                    fixed: None,
                    min: constraints.min,
                    max: constraints.max,
                    ..*constraints
                };
                tree.set_constraints(*pid, updated)?;
            }
            SiblingKind::Container => {
                tree.set_node_flex_grow(sibling.nid, new_grow)?;
            }
            SiblingKind::FixedPanel { .. } => {
                return Err(PaneError::InvalidMutation(MutationError::SiblingsNotGrow));
            }
        }
    }
    Ok(())
}

fn adjust_fixed(
    tree: &mut LayoutTree,
    target_nid: NodeId,
    delta: f32,
    siblings: &[SiblingInfo],
) -> Result<(), PaneError> {
    let target_idx = siblings
        .iter()
        .position(|s| s.nid == target_nid)
        .ok_or(PaneError::NodeNotFound(target_nid))?;

    match &siblings[target_idx].kind {
        SiblingKind::FixedPanel { pid, constraints } => {
            let new_fixed = clamp_fixed(constraints.fixed.unwrap_or(0.0) + delta, constraints);
            let updated = Constraints {
                fixed: Some(new_fixed),
                grow: None,
                min: constraints.min,
                max: constraints.max,
                ..*constraints
            };
            tree.set_constraints(*pid, updated)
        }
        SiblingKind::Panel { .. } | SiblingKind::Container => {
            let (_, neighbor) = find_nearest_fixed(target_idx, siblings)?;
            let SiblingKind::FixedPanel { pid, constraints } = &neighbor.kind else {
                return Err(PaneError::InvalidMutation(MutationError::SiblingsNotGrow));
            };
            let new_fixed = clamp_fixed(constraints.fixed.unwrap_or(0.0) - delta, constraints);
            let updated = Constraints {
                fixed: Some(new_fixed),
                grow: None,
                min: constraints.min,
                max: constraints.max,
                ..*constraints
            };
            tree.set_constraints(*pid, updated)
        }
    }
}

fn clamp_fixed(value: f32, constraints: &Constraints) -> f32 {
    let floor = constraints.min.unwrap_or(1.0).max(1.0);
    let ceiling = constraints.max.unwrap_or(f32::MAX);
    value.clamp(floor, ceiling)
}

fn find_nearest_fixed(
    target_idx: usize,
    siblings: &[SiblingInfo],
) -> Result<(usize, &SiblingInfo), PaneError> {
    let left = (0..target_idx).rev().find(|&i| siblings[i].is_fixed());
    let right = ((target_idx + 1)..siblings.len()).find(|&i| siblings[i].is_fixed());
    match (left, right) {
        (Some(i), _) | (None, Some(i)) => Ok((i, &siblings[i])),
        (None, None) => Err(PaneError::InvalidMutation(MutationError::SiblingsNotGrow)),
    }
}

fn redistribute_grow(
    target_nid: NodeId,
    delta: f32,
    siblings: &[SiblingInfo],
) -> Result<Vec<f32>, PaneError> {
    const EPSILON: f32 = 0.001;

    let total_grow: f32 = siblings.iter().map(|s| s.grow).sum();
    let target = siblings
        .iter()
        .find(|s| s.nid == target_nid)
        .ok_or(PaneError::NodeNotFound(target_nid))?;
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
        .map(|s| match s.nid == target_nid {
            true => new_share * total_grow,
            false => (s.grow * scale).max(EPSILON),
        })
        .collect())
}

enum SiblingKind {
    Panel {
        pid: PanelId,
        constraints: Constraints,
    },
    FixedPanel {
        pid: PanelId,
        constraints: Constraints,
    },
    Container,
}

struct SiblingInfo {
    nid: NodeId,
    grow: f32,
    kind: SiblingKind,
}

impl SiblingInfo {
    fn is_fixed(&self) -> bool {
        matches!(self.kind, SiblingKind::FixedPanel { .. })
    }
}

enum ResizeMode {
    PureGrow,
    FixedAdjust,
}

fn classify_resize(siblings: &[SiblingInfo]) -> ResizeMode {
    match siblings.iter().any(SiblingInfo::is_fixed) {
        true => ResizeMode::FixedAdjust,
        false => ResizeMode::PureGrow,
    }
}

fn panel_sibling(nid: NodeId, pid: PanelId, constraints: Constraints) -> SiblingInfo {
    let (grow, kind) = match (constraints.grow, constraints.fixed) {
        (Some(g), _) => (g, SiblingKind::Panel { pid, constraints }),
        (None, None) => (1.0, SiblingKind::Panel { pid, constraints }),
        (None, Some(_)) => (0.0, SiblingKind::FixedPanel { pid, constraints }),
    };
    SiblingInfo { nid, grow, kind }
}

fn collect_siblings(tree: &LayoutTree, parent_nid: NodeId) -> Result<Vec<SiblingInfo>, PaneError> {
    let children = tree.children(parent_nid)?;
    let mut siblings = Vec::with_capacity(children.len());
    for &child_nid in children {
        let node = tree
            .node(child_nid)
            .ok_or(PaneError::NodeNotFound(child_nid))?;
        match node {
            Node::Panel {
                id, constraints, ..
            } => siblings.push(panel_sibling(child_nid, *id, *constraints)),
            Node::TaffyPassthrough { style, .. } if style.flex_grow > 0.0 => {
                siblings.push(SiblingInfo {
                    nid: child_nid,
                    grow: style.flex_grow,
                    kind: SiblingKind::Container,
                });
            }
            Node::TaffyPassthrough { .. } => {} // skip zero-grow decorations
            Node::Row { .. }
            | Node::Col { .. }
            | Node::Grid { .. }
            | Node::GridItemWrapper { .. } => {
                return Err(PaneError::InvalidMutation(MutationError::SiblingsNotPanels));
            }
        }
    }
    Ok(siblings)
}
