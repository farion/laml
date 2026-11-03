//! Visual arrow tips meet the visible edge of port dots or squares.
//! Routing stays anchored at the port center for deterministic obstacle geometry.
use crate::geometry::BoxNode;
use crate::route::{Point, Route};
use crate::syntax::{EndSymbol, NodeKind};
use crate::theme::Theme;

pub(super) fn marker_tips(
    route: &Route,
    boxes: &[BoxNode],
    theme: Theme,
) -> (Option<Point>, Option<Point>) {
    if route.points.len() < 2 {
        return (None, None);
    }
    let tip = |node_index: usize, at: Point, towards: Point| {
        let node = boxes.get(node_index)?;
        node.ports
            .iter()
            .find(|port| (port.x - at.x).abs() < 1e-6 && (port.y - at.y).abs() < 1e-6)?;
        let (dx, dy) = (towards.x - at.x, towards.y - at.y);
        let length = dx.hypot(dy);
        if length <= 1e-6 {
            return None;
        }
        let (ux, uy) = (dx / length, dy / length);
        let offset = if theme.modern() && node.kind == NodeKind::Generic {
            // Modern generic ports are circles with radius 5 and a 2px stroke.
            6.0
        } else {
            // Other ports are 10px squares with the node's border width.
            // For angled segments, intersect the square's outer contour.
            (5.0 + node.border_width / 2.0) / ux.abs().max(uy.abs())
        };
        // Preserve the route direction if a final segment is unusually short.
        if length <= offset + 1e-6 {
            return None;
        }
        Some(Point {
            x: at.x + ux * offset,
            y: at.y + uy * offset,
        })
    };
    let start = (route.appearance.start == EndSymbol::Arrow)
        .then(|| tip(route.from, route.points[0], route.points[1]))
        .flatten();
    let last = route.points.len() - 1;
    let end = (route.appearance.end == EndSymbol::Arrow)
        .then(|| tip(route.to, route.points[last], route.points[last - 1]))
        .flatten();
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    // End-to-end geometry and SVG marker tests live in tests/theme_details.rs.
    #[test]
    fn arrow_tips_do_not_move_without_a_matching_named_port() {
        let route = Route {
            points: vec![Point { x: 0.0, y: 0.0 }, Point { x: 20.0, y: 0.0 }],
            qualifiers: vec![],
            bridges: vec![],
            gaps: vec![],
            label: None,
            end_labels: vec![],
            from: 0,
            to: 0,
            color: String::new(),
            width: 2.0,
            font_size: 14.0,
            bidirectional: false,
            marker_end: true,
            appearance: Default::default(),
        };
        assert_eq!(marker_tips(&route, &[], Theme::Plain), (None, None));
    }
}
