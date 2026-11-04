//! Size UML control bars and input actions from visible activity topology.
use super::BoxNode;
use crate::syntax::{ActivityFlow, ActivityPinDirection, Diagram, FlowDirection, NodeKind};

// A two-arm join (48) fits between object rows (50) with 16 px on both sides.
pub(crate) const PARALLEL_LANE_GAP: f64 = 130.0;

pub(super) fn size_nodes(diagram: &Diagram, boxes: &mut [BoxNode]) {
    if diagram.settings.direction != FlowDirection::LeftToRight {
        return;
    }
    for node in boxes {
        if node.kind == NodeKind::ActivityJoin && node.fixed_height.is_none() {
            let arms = diagram
                .relations
                .iter()
                .filter(|r| r.to == node.id && r.activity_flow == Some(ActivityFlow::Control))
                .count();
            if arms >= 2 {
                node.height = (24.0 + 12.0 * arms as f64).max(48.0);
            }
        }
        if node.kind != NodeKind::ActivityAction || node.fixed_height.is_some() {
            continue; // An authored action height is mandatory.
        }
        let inputs: Vec<_> = node
            .ports
            .iter()
            .filter(|p| p.activity_pin == Some(ActivityPinDirection::Input))
            .collect();
        if inputs.len() < 2 {
            continue;
        }
        let joins = diagram.relations.iter().any(|r| {
            r.to == node.id
                && r.activity_flow == Some(ActivityFlow::Control)
                && diagram
                    .nodes
                    .iter()
                    .any(|from| from.id == r.from && from.kind == NodeKind::ActivityJoin)
        });
        let fed = inputs.iter().all(|port| {
            diagram.relations.iter().any(|r| {
                r.to == port.id
                    && r.activity_flow == Some(ActivityFlow::Object)
                    && diagram
                        .nodes
                        .iter()
                        .any(|from| from.id == r.from && from.kind == NodeKind::ActivityObject)
            })
        });
        if joins && fed {
            let first = node.port_start + node.font_size * 1.25 / 2.0;
            // The pin rows match the minimum separation of two action lanes.
            node.height = node
                .height
                .max(2.0 * first + PARALLEL_LANE_GAP * (inputs.len() - 1) as f64);
        }
    }
}
