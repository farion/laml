//! Keep the control spine axial while giving a lone input object its own row.
//! These are soft preferences: authored layout and collision safety take priority.
use super::{Equality, Minimum, solve_axis, try_soft};
use crate::geometry::BoxNode;
use crate::syntax::{ActivityFlow, Diagram, DiagramType, FlowDirection, NodeKind, RelationDecl};

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
    boxes: &[BoxNode],
    eq_x: &mut Vec<Equality>,
    eq_y: &mut Vec<Equality>,
    x: &[Minimum],
    y: &mut Vec<Minimum>,
    pref_x: &[f64],
    pref_y: &[f64],
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
        let linear = matches!(
            boxes[a].kind,
            NodeKind::ActivityInitial | NodeKind::ActivityAction
        ) && matches!(
            boxes[b].kind,
            NodeKind::ActivityAction | NodeKind::ActivityFinal
        ) && diagram
            .relations
            .iter()
            .filter(|r| r.activity_flow == Some(ActivityFlow::Control) && r.from == relation.from)
            .count()
            == 1
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
}
