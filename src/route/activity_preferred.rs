//! Notation-aware preferred routes for UML activity control and object flows.
use super::{Point, geometry::center, preferred::forced_port};
use crate::geometry::BoxNode;
use crate::syntax::PortSide;

pub(super) fn activity_fork_to_action(a: &BoxNode, b: &BoxNode) -> Option<Vec<Point>> {
    use crate::syntax::NodeKind;
    if a.kind != NodeKind::ActivityFork
        || b.kind != NodeKind::ActivityAction
        || a.x + a.width >= b.x
    {
        return None;
    }
    let y = center(b).y;
    let start_y = y.clamp(a.y + 12.0, a.y + a.height - 12.0);
    let start = Point {
        x: a.x + a.width,
        y: start_y,
    };
    let end = Point { x: b.x, y };
    if (start_y - y).abs() < 1e-6 {
        return Some(vec![start, end]);
    }
    let x = (start.x + end.x) / 2.0;
    Some(vec![start, Point { x, y: start_y }, Point { x, y }, end])
}

pub(super) fn activity_action_to_join(a: &BoxNode, b: &BoxNode) -> Option<Vec<Point>> {
    use crate::syntax::NodeKind;
    if a.kind != NodeKind::ActivityAction
        || b.kind != NodeKind::ActivityJoin
        || a.x + a.width >= b.x
    {
        return None;
    }
    let action_center = center(a).y;
    let join_center = center(b).y;
    let upper = action_center < join_center;
    let start = Point {
        x: center(a).x,
        y: if upper { a.y + a.height } else { a.y },
    };
    let lane = if upper {
        b.y + 10.0
    } else {
        b.y + b.height - 10.0
    };
    if (lane - start.y) * (if upper { 1.0 } else { -1.0 }) <= 1.0 {
        return None;
    }
    Some(vec![
        start,
        Point {
            x: start.x,
            y: lane,
        },
        Point { x: b.x, y: lane },
    ])
}

pub(super) fn activity_output_to_object(
    a: &BoxNode,
    b: &BoxNode,
    member: &str,
) -> Option<Vec<Point>> {
    use crate::syntax::{ActivityPinDirection, NodeKind};
    if a.kind != NodeKind::ActivityAction
        || b.kind != NodeKind::ActivityObject
        || a.x + a.width >= b.x
    {
        return None;
    }
    let pin = a
        .ports
        .iter()
        .find(|p| p.id == member && p.activity_pin == Some(ActivityPinDirection::Output))?;
    let middle = b.y + b.height / 2.0;
    ((pin.y - middle).abs() < 1e-4).then_some(vec![
        Point { x: pin.x, y: pin.y },
        Point { x: b.x, y: middle },
    ])
}

pub(super) fn activity_decision_to_fork(a: &BoxNode, b: &BoxNode) -> Option<Vec<Point>> {
    use crate::syntax::NodeKind;
    if a.kind != NodeKind::ActivityDecision
        || b.kind != NodeKind::ActivityFork
        || a.x + a.width >= b.x
        || a.y + a.height >= b.y
    {
        return None;
    }
    // Leave the lower-right slope: the exact bottom vertex may sit within the
    // incoming control arrowhead's reserved footprint.
    let c = center(a);
    let start = Point {
        x: c.x + 5.0,
        y: c.y + 7.0,
    };
    let end = forced_port(b, PortSide::Left);
    Some(vec![
        start,
        Point {
            x: start.x,
            y: end.y,
        },
        end,
    ])
}

pub(super) fn activity_control_entry(a: &BoxNode, b: &BoxNode) -> Option<Vec<Point>> {
    use crate::syntax::{ActivityPinDirection, NodeKind};
    if a.kind != NodeKind::ActivityInitial
        || b.kind != NodeKind::ActivityAction
        || !b
            .ports
            .iter()
            .any(|p| p.activity_pin == Some(ActivityPinDirection::Input))
        || a.x + a.width >= b.x
    {
        return None;
    }
    let y = b.y + b.radius + 2.0;
    let start = forced_port(a, PortSide::Right);
    ((start.y - y).abs() < 1e-4).then_some(vec![start, Point { x: b.x, y }])
}

// A join between two object lanes reaches an action above them from its
// bottom, without cutting across the object flows entering its left pins.
pub(super) fn activity_join_to_action(a: &BoxNode, b: &BoxNode) -> Option<Vec<Point>> {
    use crate::syntax::NodeKind;
    if a.kind != NodeKind::ActivityJoin
        || b.kind != NodeKind::ActivityAction
        || a.x + a.width >= b.x
        || a.y + a.height / 2.0 <= b.y + b.height + 8.0
    {
        return None;
    }
    let start = forced_port(a, PortSide::Right);
    let end = forced_port(b, PortSide::Bottom);
    Some(vec![
        start,
        Point {
            x: end.x,
            y: start.y,
        },
        end,
    ])
}

pub(super) fn activity_object_to_input(
    a: &BoxNode,
    b: &BoxNode,
    member: &str,
) -> Option<Vec<Point>> {
    use crate::syntax::{ActivityPinDirection, NodeKind};
    if a.kind != NodeKind::ActivityObject || b.kind != NodeKind::ActivityAction {
        return None;
    }
    let pin = b
        .ports
        .iter()
        .find(|p| p.id == member && p.activity_pin == Some(ActivityPinDirection::Input))?;
    if a.x + a.width + 48.0 < b.x {
        let start = forced_port(a, PortSide::Right);
        if (start.y - pin.y).abs() < 1e-4 {
            return Some(vec![start, Point { x: pin.x, y: pin.y }]);
        }
        // Multiple object rows get distinct orthogonal lanes approaching the
        // action's input pins from the left, rather than crossing diagonals.
        let row = b
            .ports
            .iter()
            .take_while(|p| p.id != pin.id)
            .filter(|p| p.activity_pin == Some(ActivityPinDirection::Input))
            .count();
        let start = forced_port(a, PortSide::Right);
        let x = start.x + 24.0 + 20.0 * row as f64;
        if x < pin.x - 24.0 {
            return Some(vec![
                start,
                Point { x, y: start.y },
                Point { x, y: pin.y },
                Point { x: pin.x, y: pin.y },
            ]);
        }
    }
    if a.y < b.y + b.height + 8.0 {
        return None;
    }
    let start = Point {
        x: a.x,
        y: a.y + a.height / 2.0,
    };
    let x = a.x.min(b.x) - 24.0;
    Some(vec![
        start,
        Point { x, y: start.y },
        Point { x, y: pin.y },
        Point { x: pin.x, y: pin.y },
    ])
}
