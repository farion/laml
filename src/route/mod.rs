//! Bounded first SVG router. Never draws a line through a node or a bare
//! crossing.

use crate::geometry::BoxNode;
use crate::styles::{Element, StyleSheet, Value};
use crate::syntax::{Diagram, LabelPlacement, RelationDecl};

mod bridges;
mod end_labels;
mod geometry;
mod labels;
mod parallel;
mod self_loops;
use geometry::{center, member_port, port, route_is_clear};
use labels::{label_along, label_inline, label_near};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
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
    let mut results = vec![vec![port(a, cb), port(b, ca)]];
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
    results
        .into_iter()
        .filter_map(|mut points| {
            let first_horizontal = (points[0].y - points[1].y).abs() < 1e-6;
            let last = points.len() - 1;
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
                member_port(a, id, points[1])?
            } else {
                parallel::shift_port(a, points[0], start_offset)?
            };
            let end = if let Some(id) = target_member {
                member_port(b, id, points[last - 1])?
            } else {
                parallel::shift_port(b, points[last], end_offset)?
            };
            points[0] = start;
            points[last] = end;
            // Shifting a boundary port must shift its adjacent elbow too.
            // Otherwise an exterior detour gains short diagonal stubs.
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
                _ => {}
            }
            Some(points)
        })
        .collect()
}
fn style(relation: &RelationDecl, sheet: &StyleSheet) -> (String, f64, f64) {
    let element = Element::Relation {
        name: relation.name.as_deref(),
        classes: &relation.classes,
    };
    let number = |key, default| match sheet.get(&element, key).map(|d| &d.value) {
        Some(Value::Number(n)) => *n,
        _ => default,
    };
    let color = match sheet.get(&element, "color").map(|d| &d.value) {
        Some(Value::Color(c)) => c.clone(),
        _ => "#395373".into(),
    };
    (color, number("width", 2.0), number("font-size", 14.0))
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
                .position(|b| {
                    b.id == id || b.body.iter().any(|part| {
            matches!(part, crate::syntax::NodePart::Member { id: Some(member), .. } if member == id)
        })
                })
                .expect("parsed relation endpoint")
        };
        let from = owner(&relation.from);
        let to = owner(&relation.to);
        if from == to && (relation.from != boxes[from].id || relation.to != boxes[to].id) {
            return Err(RouteError {
                line: relation.line,
                message:
                    "member-to-member relation within one node needs a separate exterior route"
                        .into(),
            });
        }
        let (color, width, font_size) = style(relation, sheet);
        let options = if from == to {
            let rank = diagram.relations[..index]
                .iter()
                .filter(|r| r.from == relation.from && r.to == relation.to)
                .count();
            if rank >= 4 {
                return Err(RouteError {
                    line: relation.line,
                    message: "more than four self-relations need additional distinct loop ports"
                        .into(),
                });
            }
            self_loops::candidates(&boxes[from], rank)
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
        let shown_label = relation.display_label();
        let label_rule = diagram
            .relation_layout
            .iter()
            .find(|rule| relation.name.as_deref() == Some(rule.name.as_str()));
        let placement = label_rule.map_or(diagram.settings.relation_label, |rule| rule.placement);
        let label_line = label_rule.map_or(relation.line, |rule| rule.line);
        let mut choice = None;
        let mut label_error = None;
        for points in options {
            if !route_is_clear(&points, boxes, &routed) {
                continue;
            }
            let Some(plan) = bridges::plan(&points, boxes, &routed, from, to, width) else {
                continue;
            };
            let labeled = match shown_label.as_deref() {
                None => Ok((None, None)),
                Some(text) => match placement {
                    LabelPlacement::Near => {
                        label_near(&points, text, font_size, width, boxes, &routed)
                            .map(|label| (label, None))
                    }
                    LabelPlacement::Along => {
                        label_along(&points, text, font_size, width, boxes, &routed)
                            .map(|label| (label, None))
                    }
                    LabelPlacement::Inline => label_inline(
                        &points, text, font_size, width, boxes, &routed,
                    )
                    .map(|result| match result {
                        Some((label, gap)) => (Some(label), Some(gap)),
                        None => (None, None),
                    }),
                },
            };
            let (label, label_gap) = match labeled {
                Ok(pair) => pair,
                Err(error) => {
                    label_error = Some(error);
                    continue;
                }
            };
            if label_gap.as_ref().is_some_and(|gap| {
                plan.events.iter().any(|(_, bridge, _)| {
                    bridge.segment == gap.segment
                        && (bridge.distance - gap.distance).abs() < bridge.radius + gap.half + 4.0
                })
            }) {
                label_error = Some("inline label gap conflicts with required bridge".into());
                continue;
            }
            if !plan.avoids_label(&points, &label) {
                label_error = Some("relation label conflicts with required bridge".into());
                continue;
            }
            let end_labels = match end_labels::place(
                relation,
                &points,
                font_size,
                boxes,
                &routed,
                label.as_ref(),
            ) {
                Ok(labels) => labels,
                Err(message) => {
                    label_error = Some(message);
                    continue;
                }
            };
            choice = Some((points, label, end_labels, label_gap, plan));
            break;
        }
        let (points,label,end_labels,label_gap,plan) = choice.ok_or_else(|| RouteError { line: if label_error.is_some() {label_line} else {relation.line},
            message: label_error.unwrap_or_else(|| "bounded router cannot avoid nodes/relations or make a safe bridge for the selected ports".into()) })?;
        let new_bridges = plan.apply(&mut routed);
        routed.push(Route {
            points,
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
    Ok(routed)
}

#[cfg(test)]
mod candidate_tests {
    use super::*;
    use crate::geometry::place;
    use crate::parser::parse;

    #[test]
    fn shifted_ports_keep_bent_detours_orthogonal_on_both_axes() {
        for rule in [
            "A < B : align=center; gap=220",
            "A above B : align=center; gap=220",
        ] {
            let source = format!("semantics {{\nA\nB\n}}\nlayout {{\n{rule}\n}}\n");
            let diagram = parse(&source).unwrap();
            let boxes = place(&diagram, &StyleSheet::default()).unwrap();
            let options = candidates(&diagram, 0, 0, 1, &boxes, &[], 12.0);
            let valid: Vec<_> = options
                .iter()
                .filter(|points| route_is_clear(points, &boxes, &[]))
                .collect();
            assert!(valid.iter().any(|points| points.len() == 4), "{rule}");
            for points in valid {
                assert!(
                    points
                        .windows(2)
                        .all(|pair| (pair[0].x - pair[1].x).abs() < 1e-6
                            || (pair[0].y - pair[1].y).abs() < 1e-6),
                    "{rule}: {points:?}"
                );
            }
        }
    }
}
