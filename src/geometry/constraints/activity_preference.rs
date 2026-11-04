//! Keep the control spine axial while giving a lone input object its own row.
//! These are soft preferences: authored layout and collision safety take priority.
use super::{Equality, Minimum, solve_axis, try_soft};
use crate::geometry::BoxNode;
use crate::syntax::{ActivityFlow, Diagram, DiagramType, FlowDirection, NodeKind, RelationDecl};

fn branch_lane(node: &BoxNode, y: f64) -> f64 {
    let outputs: Vec<_> = node
        .ports
        .iter()
        .filter(|p| p.activity_pin == Some(crate::syntax::ActivityPinDirection::Output))
        .collect();
    if outputs.len() != 1 {
        return y + node.height / 2.0;
    }
    let row = node
        .ports
        .iter()
        .take_while(|p| p.id != outputs[0].id)
        .filter(|p| {
            matches!(
                p.side,
                crate::syntax::PortSide::Left | crate::syntax::PortSide::Right
            )
        })
        .count();
    y + node.port_start + node.font_size * 1.25 * (row as f64 + 0.5)
}

pub(super) fn prefers_below(
    diagram: &Diagram,
    boxes: &[BoxNode],
    relation: &RelationDecl,
    object: usize,
    action: usize,
) -> bool {
    diagram.settings.diagram_type == DiagramType::Activity
        && diagram.settings.direction == FlowDirection::LeftToRight
        && relation.activity_flow == Some(ActivityFlow::Object)
        && boxes[object].kind == NodeKind::ActivityObject
        && boxes[action].kind == NodeKind::ActivityAction
        && diagram
            .relations
            .iter()
            .filter(|other| {
                other.activity_flow == Some(ActivityFlow::Object)
                    && boxes[action].has_endpoint(&other.to)
            })
            .count()
            == 1
}

#[allow(clippy::too_many_arguments)] // Soft placement must check both axes against their hard constraints.
pub(super) fn apply(
    diagram: &Diagram,
    original: &Diagram,
    boxes: &[BoxNode],
    eq_x: &mut Vec<Equality>,
    eq_y: &mut Vec<Equality>,
    x: &mut Vec<Minimum>,
    y: &mut Vec<Minimum>,
    pref_x: &[f64],
    pref_y: &mut [f64],
) {
    if diagram.settings.diagram_type != DiagramType::Activity
        || diagram.settings.direction != FlowDirection::LeftToRight
    {
        return;
    }
    let owner = |id: &str| boxes.iter().position(|node| node.has_endpoint(id)).unwrap();
    for relation in &diagram.relations {
        if relation.activity_flow != Some(ActivityFlow::Control) {
            continue;
        }
        let (a, b) = (owner(&relation.from), owner(&relation.to));
        let outgoing: Vec<_> = diagram
            .relations
            .iter()
            .filter(|r| r.activity_flow == Some(ActivityFlow::Control) && r.from == relation.from)
            .collect();
        let straight_chain = matches!(
            boxes[a].kind,
            NodeKind::ActivityInitial | NodeKind::ActivityAction
        ) && matches!(
            boxes[b].kind,
            NodeKind::ActivityAction | NodeKind::ActivityDecision | NodeKind::ActivityFinal
        ) && outgoing.len() == 1;
        let linear = straight_chain
            && diagram
                .relations
                .iter()
                .filter(|r| r.activity_flow == Some(ActivityFlow::Control) && r.to == relation.to)
                .count()
                == 1;
        if !linear {
            continue;
        }
        let target_offset = if boxes[a].kind == NodeKind::ActivityInitial
            && boxes[b].kind == NodeKind::ActivityAction
            && boxes[b]
                .ports
                .iter()
                .any(|p| p.activity_pin == Some(crate::syntax::ActivityPinDirection::Input))
        {
            // Control enters in the clear upper band; the lower left edge is
            // occupied by the input pin and its flow arrowhead.
            boxes[b].radius + 2.0
        } else {
            boxes[b].height / 2.0
        };
        eq_y.push(Equality {
            a,
            b,
            delta: boxes[a].height / 2.0 - target_offset,
            line: 0,
        });
        if solve_axis(boxes.len(), eq_y, y, pref_y).is_err() {
            eq_y.pop();
        }
    }
    // A lone output pin and its object node share one horizontal flow lane.
    // This preserves the branch order through the downstream input pins.
    for relation in &diagram.relations {
        if relation.activity_flow != Some(ActivityFlow::Object) {
            continue;
        }
        let (action, object) = (owner(&relation.from), owner(&relation.to));
        if boxes[action].kind != NodeKind::ActivityAction
            || boxes[object].kind != NodeKind::ActivityObject
        {
            continue;
        }
        let outputs: Vec<_> = boxes[action]
            .ports
            .iter()
            .filter(|p| p.activity_pin == Some(crate::syntax::ActivityPinDirection::Output))
            .collect();
        if outputs.len() != 1 {
            continue;
        }
        let pin = outputs[0];
        let row = boxes[action]
            .ports
            .iter()
            .take_while(|p| p.id != pin.id)
            .filter(|p| {
                matches!(
                    p.side,
                    crate::syntax::PortSide::Left | crate::syntax::PortSide::Right
                )
            })
            .count();
        eq_y.push(Equality {
            a: action,
            b: object,
            delta: boxes[action].port_start + boxes[action].font_size * 1.25 * (row as f64 + 0.5)
                - boxes[object].height / 2.0,
            line: 0,
        });
        if solve_axis(boxes.len(), eq_y, y, pref_y).is_err() {
            eq_y.pop();
        }
    }
    // Keep input objects level with the particular pins they feed. The
    // scoped graph uses node IDs, so resolve member IDs from the source.
    for relation in &original.relations {
        if relation.activity_flow != Some(ActivityFlow::Object) {
            continue;
        }
        let Some(object) = boxes
            .iter()
            .position(|node| node.id == relation.from && node.kind == NodeKind::ActivityObject)
        else {
            continue;
        };
        let Some(action) = boxes
            .iter()
            .position(|node| node.ports.iter().any(|p| p.id == relation.to))
        else {
            continue;
        };
        let action_box = &boxes[action];
        let inputs: Vec<_> = action_box
            .ports
            .iter()
            .filter(|p| p.activity_pin == Some(crate::syntax::ActivityPinDirection::Input))
            .collect();
        if inputs.len() < 2 {
            continue;
        }
        let first = action_box.port_start + action_box.font_size * 1.25 / 2.0;
        if action_box.height
            < 2.0 * first + crate::geometry::activity::PARALLEL_LANE_GAP * (inputs.len() - 1) as f64
                - 1e-6
        {
            continue;
        }
        let Some(row) = inputs.iter().position(|p| p.id == relation.to) else {
            continue;
        };
        eq_y.push(Equality {
            a: object,
            b: action,
            delta: boxes[object].height / 2.0
                - first
                - crate::geometry::activity::PARALLEL_LANE_GAP * row as f64,
            line: 0,
        });
        if solve_axis(boxes.len(), eq_y, y, pref_y).is_err() {
            eq_y.pop();
        }
    }
    for relation in &diagram.relations {
        let (object, action) = (owner(&relation.from), owner(&relation.to));
        if !prefers_below(diagram, boxes, relation, object, action) {
            continue;
        }
        eq_x.push(Equality {
            a: object,
            b: action,
            delta: (boxes[object].width - boxes[action].width) / 2.0,
            line: 0,
        });
        if solve_axis(boxes.len(), eq_x, x, pref_x).is_err() {
            eq_x.pop();
        }
        try_soft(
            boxes.len(),
            eq_y,
            y,
            Minimum {
                a: action,
                b: object,
                distance: boxes[action].height + 24.0,
                line: 0,
            },
            pref_y,
        );
    }
    // Objects feeding different pins of one action belong in one column.
    for (index, first) in diagram.relations.iter().enumerate() {
        if first.activity_flow != Some(ActivityFlow::Object) {
            continue;
        }
        let a = owner(&first.from);
        let target = owner(&first.to);
        if boxes[a].kind != NodeKind::ActivityObject
            || boxes[target].kind != NodeKind::ActivityAction
        {
            continue;
        }
        for second in diagram.relations.iter().skip(index + 1) {
            if second.activity_flow != Some(ActivityFlow::Object) || owner(&second.to) != target {
                continue;
            }
            let b = owner(&second.from);
            if b == a || boxes[b].kind != NodeKind::ActivityObject {
                continue;
            }
            eq_x.push(Equality {
                a,
                b,
                delta: (boxes[a].width - boxes[b].width) / 2.0,
                line: 0,
            });
            if solve_axis(boxes.len(), eq_x, x, pref_x).is_err() {
                eq_x.pop();
            }
        }
    }
    // A join belongs between its incoming control branches, after the
    // column of object nodes. This leaves a corridor for data flows to the
    // input pins and keeps the outgoing control flow outside that corridor.
    for (join, node) in boxes.iter().enumerate() {
        if node.kind != NodeKind::ActivityJoin {
            continue;
        }
        let branches: Vec<_> = diagram
            .relations
            .iter()
            .filter(|r| r.activity_flow == Some(ActivityFlow::Control) && owner(&r.to) == join)
            .map(|r| owner(&r.from))
            .collect();
        if branches.len() < 2 {
            continue;
        }
        for pair in branches.windows(2) {
            let (upper, lower) = (pair[0], pair[1]);
            try_soft(
                boxes.len(),
                eq_y,
                y,
                Minimum {
                    a: upper,
                    b: lower,
                    distance: crate::geometry::activity::PARALLEL_LANE_GAP
                        + branch_lane(&boxes[upper], 0.0)
                        - branch_lane(&boxes[lower], 0.0),
                    line: 0,
                },
                pref_y,
            );
        }
        if let Ok(ys) = solve_axis(boxes.len(), eq_y, y, pref_y) {
            let midpoint = branches
                .iter()
                .map(|&branch| branch_lane(&boxes[branch], ys[branch]))
                .sum::<f64>()
                / branches.len() as f64;
            pref_y[join] = midpoint - node.height / 2.0;
        }
        for relation in &diagram.relations {
            if relation.activity_flow != Some(ActivityFlow::Object) {
                continue;
            }
            let (producer, object) = (owner(&relation.from), owner(&relation.to));
            if !branches.contains(&producer) || boxes[object].kind != NodeKind::ActivityObject {
                continue;
            }
            try_soft(
                boxes.len(),
                eq_x,
                x,
                Minimum {
                    a: object,
                    b: join,
                    distance: boxes[object].width + 68.0,
                    line: 0,
                },
                pref_x,
            );
        }
    }
}

// Give a decision's upper branch a visible vertical departure before the
// control flow turns toward an action. Move its entire existing alignment
// group, so preceding nodes keep their authored/soft axial relationships.
pub(super) fn separate_short_decision_stem(
    diagram: &Diagram,
    boxes: &[BoxNode],
    eq_y: &[Equality],
    y: &[Minimum],
    positions: &[f64],
    preferred: &mut [f64],
) -> bool {
    if diagram.settings.diagram_type != DiagramType::Activity
        || diagram.settings.direction != FlowDirection::LeftToRight
    {
        return false;
    }
    let owner = |id: &str| boxes.iter().position(|node| node.has_endpoint(id)).unwrap();
    for relation in &diagram.relations {
        if relation.activity_flow != Some(ActivityFlow::Control) {
            continue;
        }
        let (decision, target) = (owner(&relation.from), owner(&relation.to));
        if boxes[decision].kind != NodeKind::ActivityDecision
            || boxes[target].kind != NodeKind::ActivityAction
        {
            continue;
        }
        let target_center = positions[target] + boxes[target].height / 2.0;
        let decision_center = positions[decision] + boxes[decision].height / 2.0;
        if target_center >= decision_center - 2.0 {
            continue; // Not an upper branch.
        }
        let stem = positions[decision] - target_center;
        let minimum = boxes[decision].height;
        if stem >= minimum - 0.01 {
            continue;
        }
        let mut connected = vec![false; boxes.len()];
        connected[decision] = true;
        loop {
            let mut changed = false;
            for equality in eq_y {
                if connected[equality.a] && !connected[equality.b] {
                    connected[equality.b] = true;
                    changed = true;
                }
                if connected[equality.b] && !connected[equality.a] {
                    connected[equality.a] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let anchor = connected.iter().position(|member| *member).unwrap();
        let saved = preferred[anchor];
        preferred[anchor] += minimum - stem;
        if solve_axis(boxes.len(), eq_y, y, preferred).is_ok_and(|new_positions| {
            new_positions[decision] - new_positions[target] - boxes[target].height / 2.0
                >= minimum - 0.01
        }) {
            return true;
        }
        preferred[anchor] = saved; // Mandatory constraints leave no room to separate.
    }
    false
}

// Collision resolution can separate branch actions after their initial
// positions were chosen. Recenter the free join on those final branch lanes.
pub(super) fn recenter_joins(
    diagram: &Diagram,
    boxes: &[BoxNode],
    ys: &[f64],
    preferred: &mut [f64],
) -> bool {
    if diagram.settings.diagram_type != DiagramType::Activity
        || diagram.settings.direction != FlowDirection::LeftToRight
    {
        return false;
    }
    let owner = |id: &str| boxes.iter().position(|node| node.has_endpoint(id)).unwrap();
    let mut changed = false;
    for (join, node) in boxes.iter().enumerate() {
        if node.kind != NodeKind::ActivityJoin
            || diagram
                .layout
                .iter()
                .any(|rule| rule.first == node.id || rule.second == node.id)
        {
            continue;
        }
        let branches: Vec<_> = diagram
            .relations
            .iter()
            .filter(|r| r.activity_flow == Some(ActivityFlow::Control) && owner(&r.to) == join)
            .map(|r| owner(&r.from))
            .collect();
        if branches.len() < 2 {
            continue;
        }
        let midpoint = branches
            .iter()
            .map(|&branch| branch_lane(&boxes[branch], ys[branch]))
            .sum::<f64>()
            / branches.len() as f64;
        let target = midpoint - node.height / 2.0;
        if (preferred[join] - target).abs() > 0.01 {
            preferred[join] = target;
            changed = true;
        }
    }
    changed
}
