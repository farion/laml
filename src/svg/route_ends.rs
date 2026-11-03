//! Visual arrow tips meet the outside of modern GNE port dots, not their centers.
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
    if !theme.modern() || route.points.len() < 2 {
        return (None, None);
    }
    let tip = |node_index: usize, at: Point, towards: Point| {
        let node = boxes.get(node_index)?;
        if node.kind != NodeKind::Generic
            || !node
                .ports
                .iter()
                .any(|port| (port.x - at.x).abs() < 1e-6 && (port.y - at.y).abs() < 1e-6)
        {
            return None;
        }
        let (dx, dy) = (towards.x - at.x, towards.y - at.y);
        let length = dx.hypot(dy);
        if length <= 1e-6 {
            return None;
        }
        // Circle radius 5 + 1 px stroke. A short final segment must never
        // be reversed; the marker geometry remains unmodified in that case.
        if length < 12.0 {
            return None;
        }
        Some(Point {
            x: at.x + dx / length * 6.0,
            y: at.y + dy / length * 6.0,
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
    fn arrow_tips_do_not_move_in_plain_theme() {
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
