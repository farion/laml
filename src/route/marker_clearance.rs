//! Decision entries use the visible arrow footprint so other branches can
//! depart from the top or bottom of the same diamond.
use super::{Point, Route, geometry::marker_radius, labels::segment_hits_rect};
use crate::geometry::BoxNode;
use crate::syntax::{EndSymbol, NodeKind};

pub(super) fn hits_segment(
    a: Point,
    b: Point,
    route: &Route,
    boxes: &[BoxNode],
    at_start: bool,
) -> bool {
    let (tip, neighbour, symbol) = if at_start {
        (route.points[0], route.points[1], route.appearance.start)
    } else {
        let last = route.points.len() - 1;
        (
            route.points[last],
            route.points[last - 1],
            route.appearance.end,
        )
    };
    if symbol == EndSymbol::None {
        return false;
    }
    let endpoint = if at_start { route.from } else { route.to };
    // Retain conservative clearance for other notations and endpoints.
    if symbol != EndSymbol::Arrow
        || boxes.get(endpoint).is_none_or(|node| {
            !matches!(
                node.kind,
                NodeKind::ActivityDecision | NodeKind::ActivityMerge | NodeKind::StateChoice
            )
        })
    {
        let radius = marker_radius(symbol, route.width);
        return segment_hits_rect(
            a,
            b,
            tip.x - radius,
            tip.y - radius,
            2.0 * radius,
            2.0 * radius,
        );
    }
    // An open arrowhead extends *behind* its tip along the final segment,
    // not in every direction around the node. Preserve a padded footprint
    // without blocking a different flow leaving the opposite side of a fork.
    let length = (tip.x - neighbour.x).hypot(tip.y - neighbour.y);
    if length < 1e-6 {
        return false;
    }
    let (ux, uy) = (
        (tip.x - neighbour.x) / length,
        (tip.y - neighbour.y) / length,
    );
    let depth = 12.0;
    let half_width = 6.0 + route.width / 2.0;
    let corners = [
        (tip.x + uy * half_width, tip.y - ux * half_width),
        (tip.x - uy * half_width, tip.y + ux * half_width),
        (
            tip.x - ux * depth + uy * half_width,
            tip.y - uy * depth - ux * half_width,
        ),
        (
            tip.x - ux * depth - uy * half_width,
            tip.y - uy * depth + ux * half_width,
        ),
    ];
    let (min_x, max_x) = corners
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |bounds, point| {
            (bounds.0.min(point.0), bounds.1.max(point.0))
        });
    let (min_y, max_y) = corners
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |bounds, point| {
            (bounds.0.min(point.1), bounds.1.max(point.1))
        });
    segment_hits_rect(
        a,
        b,
        min_x - 1.0,
        min_y - 1.0,
        max_x - min_x + 2.0,
        max_y - min_y + 2.0,
    )
}
