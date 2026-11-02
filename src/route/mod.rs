//! Bounded first SVG router. Never draws a line through a node or a bare
//! crossing.

use crate::geometry::BoxNode;
use crate::styles::StyleSheet;
use crate::syntax::{Diagram, EndPosition, PortSide};

mod attachments;
mod bridges;
mod detours;
mod edge_style;
mod end_labels;
mod geometry;
mod labels;
mod outline_clearance;
mod parallel;
mod preferred;
mod qualifiers;
mod route_labels;
mod self_loops;
mod side_rules;
use geometry::{center, member_port, port, route_is_clear};
use preferred::{forced_port, horizontal_connection, named_port_candidates, normal_at_port};
pub use qualifiers::QualifierBox;

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
pub fn route(
    diagram: &Diagram,
    boxes: &[BoxNode],
    sheet: &StyleSheet,
) -> Result<Vec<Route>, RouteError> {
    let visible = crate::geometry::visible_diagram(diagram, sheet)
        .map_err(|(line, message)| RouteError { line, message })?;
    let diagram = &visible;
    let mut routed = Vec::new();
    for (index, relation) in diagram.relations.iter().enumerate() {
        let owner = |id: &str| {
            boxes
                .iter()
                .position(|b| b.has_endpoint(id))
                .expect("parsed relation endpoint")
        };
        let from = owner(&relation.from);
        let to = owner(&relation.to);
        if from == to && (relation.from != boxes[from].id || relation.to != boxes[to].id) {
            return Err(RouteError {
                line: relation.line,
                message: format!(
                    "relation '{}' ({} -> {}) connects two members of one node; use separate nodes or an exterior self-relation",
                    relation.name.as_deref().unwrap_or("unnamed"),
                    relation.from,
                    relation.to
                ),
            });
        }
        let (color, width, font_size) = edge_style::style(relation, sheet);
        if matches!(
            diagram.settings.diagram_type,
            crate::syntax::DiagramType::Composite | crate::syntax::DiagramType::Profile
        ) && (color == "none" || width < 1.0)
        {
            return Err(RouteError {
                line: relation.line,
                message: "composite connector stroke must remain visible".into(),
            });
        }
        let options = if from == to {
            let rank = diagram.relations[..index]
                .iter()
                .filter(|r| r.from == relation.from && r.to == relation.to)
                .count();
            if rank >= 4 {
                return Err(RouteError {
                    line: relation.line,
                    message: format!(
                        "relation '{}' ({} -> {}): more than four self-relations need distinct loop ports; use fewer loops or separate nodes",
                        relation.name.as_deref().unwrap_or("unnamed"),
                        relation.from,
                        relation.to
                    ),
                });
            }
            side_rules::loop_options(
                diagram,
                index,
                &boxes[from],
                self_loops::candidates(&boxes[from], rank),
            )
        } else {
            let offset =
                parallel::offset(diagram, index, boxes, from, to, sheet).map_err(|message| {
                    RouteError {
                        line: relation.line,
                        message,
                    }
                })?;
            candidates(diagram, index, from, to, boxes, &routed, offset)
        };
        let label_rule = diagram
            .relation_layout
            .iter()
            .find(|rule| relation.name.as_deref() == Some(rule.name.as_str()));
        let end_rules = [
            crate::syntax::EndPosition::Source,
            crate::syntax::EndPosition::Target,
        ]
        .map(|end| {
            diagram.relation_end_layout.iter().find(|rule| {
                relation.name.as_deref() == Some(rule.name.as_str()) && rule.end == end
            })
        });
        let placement = label_rule.map_or(diagram.settings.relation_label, |rule| rule.placement);
        let label_line = label_rule.map_or(relation.line, |rule| rule.line);
        let mut choice = None;
        let mut label_error = None;
        for mut points in options {
            let qualifier_boxes = match qualifiers::attach(
                relation,
                &mut points,
                boxes,
                from,
                to,
                font_size,
                &routed,
            ) {
                Ok(boxes) => boxes,
                Err(message) => {
                    label_error = Some(message);
                    continue;
                }
            };
            if !route_is_clear(&points, boxes, &routed) {
                continue;
            }
            let Some(plan) = bridges::plan(&points, boxes, &routed, from, to, width) else {
                continue;
            };
            if qualifier_boxes
                .iter()
                .any(|q| !plan.avoids_region(&points, q.x, q.y, q.width, q.height))
            {
                label_error = Some("qualifier conflicts with required bridge".into());
                continue;
            }
            let (label, end_labels, label_gap) = match route_labels::select(
                relation,
                &points,
                boxes,
                &routed,
                &plan,
                &qualifier_boxes,
                placement,
                font_size,
                width,
                end_rules,
            ) {
                Ok(labels) => labels,
                Err(message) => {
                    label_error = Some(message);
                    continue;
                }
            };
            choice = Some((points, qualifier_boxes, label, end_labels, label_gap, plan));
            break;
        }
        let (points, qualifier_boxes, label, end_labels, label_gap, plan) =
            choice.ok_or_else(|| {
                let label_failed = label_error.is_some();
                let reason = label_error.unwrap_or_else(|| {
                    "no safe path around nodes, other connections, and crossings".into()
                });
                let identity = relation.name.as_deref().unwrap_or("unnamed");
                let advice = if let Some(name) = relation.name.as_deref() {
                    format!(
                        "Increase the gap between the connected elements or set relation[{name}].source : side=left (or choose a suitable target side)"
                    )
                } else {
                    "Increase the gap between the connected elements or name the relation to select its attachment sides".into()
                };
                RouteError {
                    line: if label_failed && label_rule.is_some() {
                        label_line
                    } else {
                        relation.line
                    },
                    message: format!(
                        "relation '{identity}' ({} -> {}): {reason}. {advice}",
                        relation.from, relation.to
                    ),
                }
            })?;
        let new_bridges = plan.apply(&mut routed);
        routed.push(Route {
            points,
            qualifiers: qualifier_boxes,
            bridges: new_bridges,
            gaps: label_gap.into_iter().collect(),
            label,
            end_labels,
            from,
            to,
            color,
            width,
            font_size,
            bidirectional: relation.kind == crate::RelationKind::Bidirectional,
            marker_end: relation.kind != crate::RelationKind::Undirected,
            appearance: relation.appearance,
        });
    }
    attachments::route_attachments(diagram, boxes, sheet, &mut routed)?;
    Ok(routed)
}
