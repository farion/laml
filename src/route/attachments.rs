//! Connector to a named relation; its endpoint is a deliberate T-junction, not a bridge.
use super::{Gap, Point, Route, RouteError, bridges, geometry};
use crate::geometry::BoxNode;
use crate::styles::{Element, StyleSheet, Value};
use crate::syntax::{Diagram, EdgeAppearance};

fn near_event(route: &Route, segment: usize, distance: f64) -> bool {
    route.bridges.iter().any(|event| {
        event.segment == segment && (event.distance - distance).abs() < event.radius + 14.0
    }) || route.gaps.iter().any(|event| {
        event.segment == segment && (event.distance - distance).abs() < event.half + 14.0
    })
}

fn only_target_touch(start: Point, anchor: Point, target: &Route) -> bool {
    let hits: Vec<_> = target
        .points
        .windows(2)
        .filter(|pair| geometry::crosses(start, anchor, pair[0], pair[1]))
        .collect();
    if hits.len() != 1 {
        return false;
    }
    let segment = hits[0];
    let (dx, dy) = (anchor.x - start.x, anchor.y - start.y);
    let (ex, ey) = (segment[1].x - segment[0].x, segment[1].y - segment[0].y);
    (dx * ey - dy * ex).abs() > 1e-5 * dx.hypot(dy) * ex.hypot(ey)
}

pub(super) fn route_attachments(
    diagram: &Diagram,
    boxes: &[BoxNode],
    sheet: &StyleSheet,
    routed: &mut Vec<Route>,
) -> Result<(), RouteError> {
    for attachment in &diagram.attachments {
        let from = boxes
            .iter()
            .position(|node| node.id == attachment.source)
            .expect("validated attachment node");
        let target_index = diagram
            .relations
            .iter()
            .position(|relation| relation.name.as_deref() == Some(attachment.target.as_str()))
            .expect("validated target relation");
        let target = &routed[target_index];
        let mut lengths: Vec<_> = target
            .points
            .windows(2)
            .enumerate()
            .map(|(i, pair)| (i, (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y)))
            .collect();
        lengths.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let element = Element::Relation {
            name: attachment.name.as_deref(),
            classes: &attachment.classes,
        };
        let color = match sheet.get(&element, "color").map(|d| &d.value) {
            Some(Value::Color(c)) => c.clone(),
            _ => "#395373".into(),
        };
        let width = match sheet.get(&element, "width").map(|d| &d.value) {
            Some(Value::Number(n)) => *n,
            _ => 2.0,
        };
        let mut chosen = None;
        for (segment, length) in lengths {
            if length < 32.0 {
                continue;
            }
            let (a, b) = (target.points[segment], target.points[segment + 1]);
            for fraction in [0.5, 0.25, 0.75] {
                let distance = length * fraction;
                if near_event(target, segment, distance) {
                    continue;
                }
                let anchor = Point {
                    x: a.x + (b.x - a.x) * fraction,
                    y: a.y + (b.y - a.y) * fraction,
                };
                let start = geometry::port(&boxes[from], anchor);
                let points = vec![start, anchor];
                if !only_target_touch(start, anchor, target)
                    || !geometry::route_is_clear(&points, boxes, routed)
                {
                    continue;
                }
                if let Some(plan) = bridges::plan_except(
                    &points,
                    boxes,
                    routed,
                    from,
                    target.from,
                    width,
                    Some(target_index),
                ) {
                    chosen = Some((points, plan));
                    break;
                }
            }
            if chosen.is_some() {
                break;
            }
        }
        let (points, plan) = chosen.ok_or_else(|| RouteError {
            line: attachment.line,
            message:
                "cannot attach node to named relation without overlapping a node, label, or bridge"
                    .into(),
        })?;
        let target_node = routed[target_index].from;
        let bridges = plan.apply(routed);
        routed.push(Route {
            points,
            bridges,
            gaps: Vec::<Gap>::new(),
            label: None,
            end_labels: Vec::new(),
            from,
            to: target_node,
            color,
            width,
            font_size: 14.0,
            bidirectional: false,
            marker_end: false,
            appearance: EdgeAppearance {
                dashed: true,
                ..Default::default()
            },
        });
    }
    Ok(())
}
