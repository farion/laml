//! Revalidate existing paths after changing relation notation, without path search.
use super::{Point, Route, RouteError, marker_clearance, router, side_rules};
use crate::{
    geometry::BoxNode,
    styles::StyleSheet,
    syntax::{Diagram, EndPosition, PortSide},
};
use std::collections::BTreeMap;

/// Reuse only the centre-lines. Labels, crossings, gaps, styles and marker
/// clearance are checked again by the normal router. Caller owns geometry validity.
pub fn replay(
    diagram: &Diagram,
    boxes: &[BoxNode],
    sheet: &StyleSheet,
    previous: &[Route],
    old_diagram: &Diagram,
) -> Result<Vec<Route>, RouteError> {
    let fail = || RouteError {
        line: 0,
        message: "Existing paths need rerouting".into(),
    };
    let visible = crate::geometry::visible_diagram(diagram, sheet)
        .map_err(|(line, message)| RouteError { line, message })?;
    if !visible.attachments.is_empty() {
        return Err(fail());
    }
    let mut seeds = BTreeMap::new();
    for (index, relation) in visible.relations.iter().enumerate() {
        let old = old_diagram
            .relations
            .iter()
            .find(|r| r.line == relation.line)
            .ok_or_else(fail)?;
        let route = previous
            .iter()
            .find(|r| r.source_line == relation.line)
            .ok_or_else(fail)?;
        if !route.qualifiers.is_empty() {
            return Err(fail());
        }
        let mut points = route.points.clone();
        if old.from != relation.from || old.to != relation.to {
            points.reverse();
        }
        if points.len() < 2 {
            return Err(fail());
        }
        for (end, id, point) in [
            (EndPosition::Source, &relation.from, points[0]),
            (EndPosition::Target, &relation.to, *points.last().unwrap()),
        ] {
            if let Some(side) = side_rules::requested(&visible, index, end) {
                let node = boxes.iter().find(|b| b.has_endpoint(id)).ok_or_else(fail)?;
                let fits = match side {
                    PortSide::Left => (point.x - node.x).abs() < 1e-6,
                    PortSide::Right => (point.x - node.x - node.width).abs() < 1e-6,
                    PortSide::Top => (point.y - node.y).abs() < 1e-6,
                    PortSide::Bottom => (point.y - node.y - node.height).abs() < 1e-6,
                };
                if !fits {
                    return Err(fail());
                }
            }
        }
        seeds.insert(relation.line, points);
    }
    let order = (0..visible.relations.len()).collect::<Vec<_>>();
    let routes = router::route_in_order_mode(
        &visible,
        boxes,
        sheet,
        None,
        &order,
        false,
        Some(&seeds),
        &mut super::pass_cache::PassCache::default(),
    )?;
    // Check both traversal orders: a newly enlarged marker must not cover an
    // earlier line, label or node simply because that line was processed first.
    for (index, route) in routes.iter().enumerate() {
        for (other_index, other) in routes.iter().enumerate() {
            if index == other_index {
                continue;
            }
            for pair in other.points.windows(2) {
                if marker_clearance::hits_segment(pair[0], pair[1], route, boxes, true)
                    || marker_clearance::hits_segment(pair[0], pair[1], route, boxes, false)
                {
                    return Err(fail());
                }
            }
            for label in other.label.iter().chain(&other.end_labels) {
                for (a, b) in rect_edges(label.x, label.y, label.width, label.height) {
                    if marker_clearance::hits_segment(a, b, route, boxes, true)
                        || marker_clearance::hits_segment(a, b, route, boxes, false)
                    {
                        return Err(fail());
                    }
                }
            }
        }
        for (node_index, node) in boxes.iter().enumerate() {
            if node_index == route.from || node_index == route.to || node.is_container {
                continue;
            }
            for (a, b) in rect_edges(node.x, node.y, node.width, node.height) {
                if marker_clearance::hits_segment(a, b, route, boxes, true)
                    || marker_clearance::hits_segment(a, b, route, boxes, false)
                {
                    return Err(fail());
                }
            }
        }
    }
    Ok(routes)
}

fn rect_edges(x: f64, y: f64, w: f64, h: f64) -> [(Point, Point); 4] {
    let corners = [
        Point { x, y },
        Point { x: x + w, y },
        Point { x: x + w, y: y + h },
        Point { x, y: y + h },
    ];
    std::array::from_fn(|i| (corners[i], corners[(i + 1) % 4]))
}
