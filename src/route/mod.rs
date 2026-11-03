//! Bounded first SVG router. Never draws a line through a node or a bare
//! crossing.

use crate::geometry::BoxNode;
use crate::syntax::{Diagram, EndPosition, PortSide};

mod attachments;
mod bridges;
mod detours;
mod edge_style;
mod end_labels;
mod geometry;
mod junction_arms;
mod labels;
mod outline_clearance;
mod parallel;
mod preferred;
mod qualifiers;
mod ranking;
mod route_labels;
mod router;
mod self_loops;
mod side_rules;
use geometry::{center, member_port, port};
use preferred::{forced_port, horizontal_connection, named_port_candidates, normal_at_port};
pub use qualifiers::QualifierBox;
pub use router::route;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
/// Image-space bridge side, independent of relation traversal direction.
/// Predominantly horizontal crossings arch upward; vertical ones arch left.
pub(crate) fn bridge_normal(unit: Point) -> Point {
    let mut normal = Point {
        x: unit.y,
        y: -unit.x,
    };
    if (unit.x.abs() >= unit.y.abs() && normal.y > 0.0)
        || (unit.x.abs() < unit.y.abs() && normal.x > 0.0)
    {
        normal.x = -normal.x;
        normal.y = -normal.y;
    }
    normal
}
#[derive(Clone, Debug)]
pub struct Label {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub angle: f64,
    pub placement: crate::syntax::LabelPlacement,
}
#[derive(Clone, Debug)]
pub struct Route {
    pub points: Vec<Point>,
    pub qualifiers: Vec<QualifierBox>,
    pub bridges: Vec<Bridge>,
    pub gaps: Vec<Gap>,
    pub label: Option<Label>,
    pub end_labels: Vec<Label>,
    pub from: usize,
    pub to: usize,
    pub color: String,
    pub width: f64,
    pub font_size: f64,
    pub bidirectional: bool,
    pub marker_end: bool,
    pub appearance: crate::syntax::EdgeAppearance,
}
#[derive(Clone, Debug)]
pub struct Bridge {
    pub segment: usize,
    pub distance: f64,
    pub radius: f64,
    pub height: f64,
}
#[derive(Clone, Debug)]
pub struct Gap {
    pub segment: usize,
    pub distance: f64,
    pub half: f64,
}
#[derive(Clone, Debug)]
pub struct RouteError {
    pub line: usize,
    pub message: String,
}

fn candidates(
    diagram: &Diagram,
    index: usize,
    from: usize,
    to: usize,
    boxes: &[BoxNode],
    previous: &[Route],
    offset: f64,
) -> Vec<Vec<Point>> {
    let a = &boxes[from];
    let b = &boxes[to];
    let source_member = diagram
        .relations
        .get(index)
        .and_then(|relation| (relation.from != a.id).then_some(relation.from.as_str()));
    let target_member = diagram
        .relations
        .get(index)
        .and_then(|relation| (relation.to != b.id).then_some(relation.to.as_str()));
    let ca = center(a);
    let cb = center(b);
    let source_side = side_rules::requested(diagram, index, EndPosition::Source);
    let target_side = side_rules::requested(diagram, index, EndPosition::Target);
    let named_port = |node: &BoxNode, id: Option<&str>| {
        id.and_then(|id| node.ports.iter().find(|port| port.id == id))
            .map(|port| {
                (
                    Point {
                        x: port.x,
                        y: port.y,
                    },
                    port.side,
                )
            })
    };
    // UML assembly and delegation connectors obey their own owner-boundary
    // notation; normal exit here applies to generic named boundary ports.
    let generic_ports = diagram.settings.diagram_type == crate::syntax::DiagramType::Generic;
    let source_port = generic_ports
        .then(|| named_port(a, source_member))
        .flatten();
    let target_port = generic_ports
        .then(|| named_port(b, target_member))
        .flatten();
    let mut results = if source_port.is_some() || target_port.is_some() {
        named_port_candidates(a, b, source_port, target_port)
    } else {
        horizontal_connection(a, b)
            .filter(|_| {
                source_side.is_none()
                    && target_side.is_none()
                    && source_member.is_none()
                    && target_member.is_none()
            })
            .into_iter()
            .collect()
    };
    if diagram.settings.diagram_type == crate::syntax::DiagramType::Class
        && source_side.is_none()
        && target_side.is_none()
    {
        results.extend(junction_arms::candidates(a, b));
    }
    let preferred_count = results.len();
    results.push(vec![port(a, cb), port(b, ca)]);
    for bend in [Point { x: cb.x, y: ca.y }, Point { x: ca.x, y: cb.y }] {
        results.push(vec![port(a, bend), bend, port(b, bend)]);
    }
    let min_x = boxes.iter().map(|b| b.x).fold(ca.x.min(cb.x), f64::min) - 32.0;
    let max_x = boxes
        .iter()
        .map(|b| b.x + b.width)
        .fold(ca.x.max(cb.x), f64::max)
        + 32.0;
    let min_y = boxes.iter().map(|b| b.y).fold(ca.y.min(cb.y), f64::min) - 32.0;
    let max_y = boxes
        .iter()
        .map(|b| b.y + b.height)
        .fold(ca.y.max(cb.y), f64::max)
        + 32.0;
    for x in [min_x, max_x] {
        let wa = Point { x, y: ca.y };
        let wb = Point { x, y: cb.y };
        results.push(vec![port(a, wa), wa, wb, port(b, wb)]);
    }
    for y in [min_y, max_y] {
        let wa = Point { x: ca.x, y };
        let wb = Point { x: cb.x, y };
        results.push(vec![port(a, wa), wa, wb, port(b, wb)]);
    }
    if source_side == Some(PortSide::Top) && target_side == Some(PortSide::Bottom) {
        for x in [min_x, max_x] {
            results.push(vec![
                forced_port(a, PortSide::Top),
                Point { x: ca.x, y: min_y },
                Point { x, y: min_y },
                Point { x, y: max_y },
                Point { x: cb.x, y: max_y },
                forced_port(b, PortSide::Bottom),
            ]);
        }
    }
    if source_side.is_some() || target_side.is_some() {
        for y in [min_y, max_y] {
            for (sx, tx) in [(min_x, max_x), (max_x, min_x)] {
                results.push(vec![
                    port(a, Point { x: sx, y: ca.y }),
                    Point { x: sx, y: ca.y },
                    Point { x: sx, y },
                    Point { x: tx, y },
                    Point { x: tx, y: cb.y },
                    port(b, Point { x: tx, y: cb.y }),
                ]);
            }
        }
    }
    results.extend(
        detours::corridors(a, b, boxes)
            .into_iter()
            .filter(|points| (source_side.is_none() && target_side.is_none()) || points.len() == 5),
    );
    results
        .into_iter()
        .enumerate()
        .filter_map(|(option, mut points)| {
            let last = points.len() - 1;
            if let Some(side) = source_side {
                points[0] = forced_port(a, side);
            }
            if let Some(side) = target_side {
                points[last] = forced_port(b, side);
            }
            let first_horizontal = (points[0].y - points[1].y).abs() < 1e-6;
            let last_horizontal = (points[last - 1].y - points[last].y).abs() < 1e-6;
            let start_offset = if source_member.is_none()
                && parallel::has_competing_port(diagram, index, boxes, from, points[0], previous)
            {
                offset
            } else {
                0.0
            };
            let end_offset = if target_member.is_none()
                && parallel::has_competing_port(diagram, index, boxes, to, points[last], previous)
            {
                offset
            } else {
                0.0
            };
            let start = if let Some(id) = source_member {
                member_port(a, id, side_rules::member_towards(a, source_side, points[1]))?
            } else {
                parallel::shift_port(a, points[0], start_offset)?
            };
            let end = if let Some(id) = target_member {
                member_port(
                    b,
                    id,
                    side_rules::member_towards(b, target_side, points[last - 1]),
                )?
            } else {
                parallel::shift_port(b, points[last], end_offset)?
            };
            points[0] = start;
            points[last] = end;
            // Shifting a boundary port must shift its adjacent elbow too.
            // Otherwise an exterior detour gains short diagonal stubs.
            if option >= preferred_count {
                match points.len() {
                    3 if first_horizontal && !last_horizontal => {
                        points[1] = Point {
                            x: end.x,
                            y: start.y,
                        };
                    }
                    3 if !first_horizontal && last_horizontal => {
                        points[1] = Point {
                            x: start.x,
                            y: end.y,
                        };
                    }
                    3 if first_horizontal && last_horizontal => points[1].y = start.y,
                    3 => points[1].x = start.x,
                    4 if first_horizontal && last_horizontal => {
                        points[1].y = start.y;
                        points[2].y = end.y;
                    }
                    4 if !first_horizontal && !last_horizontal => {
                        points[1].x = start.x;
                        points[2].x = end.x;
                    }
                    5 => {
                        if matches!(source_side, Some(PortSide::Top | PortSide::Bottom)) {
                            points[1] = Point {
                                x: start.x,
                                y: points[2].y,
                            };
                        } else {
                            points[1].y = start.y;
                        }
                        if matches!(target_side, Some(PortSide::Left | PortSide::Right)) {
                            points[3] = Point {
                                x: points[2].x,
                                y: end.y,
                            };
                        } else {
                            points[3].x = end.x;
                        }
                    }
                    _ => {}
                }
            }
            if source_port.is_some_and(|(_, side)| !normal_at_port(&points, side, true))
                || target_port.is_some_and(|(_, side)| !normal_at_port(&points, side, false))
            {
                return None;
            }
            Some(points)
        })
        .collect()
}
