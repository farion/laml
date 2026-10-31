//! Prevent an orthogonal stroke from visually merging with a rectangular node edge.
use super::Point;
use crate::geometry::{BoxNode, NodeShape};

pub(super) fn hugs(a: Point, b: Point, node: &BoxNode) -> bool {
    if node.shape != NodeShape::Rectangle {
        return false;
    }
    let overlap = |a: f64, b: f64, low: f64, high: f64| {
        a.min(b).max(low + 1.0) + 2.0 < a.max(b).min(high - 1.0)
    };
    let close = |value: f64, edge: f64| (value - edge).abs() < 4.0;
    if (a.y - b.y).abs() < 1e-6 && overlap(a.x, b.x, node.x, node.x + node.width) {
        return close(a.y, node.y) || close(a.y, node.y + node.height);
    }
    if (a.x - b.x).abs() < 1e-6 && overlap(a.y, b.y, node.y, node.y + node.height) {
        return close(a.x, node.x) || close(a.x, node.x + node.width);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn near_parallel_deployment_strokes_do_not_merge_with_box_edges() {
        let diagram =
            crate::parser::parse("settings {\ntype: deployment\n}\nsemantics {\nartifact App\n}\n")
                .unwrap();
        let boxes =
            crate::geometry::place(&diagram, &crate::styles::StyleSheet::default()).unwrap();
        let n = &boxes[0];
        let a = Point {
            x: n.x + 10.0,
            y: n.y - 0.5,
        };
        let b = Point {
            x: n.x + n.width - 10.0,
            y: a.y,
        };
        assert!(hugs(a, b, n));
        assert!(!hugs(
            Point { y: n.y - 5.0, ..a },
            Point { y: n.y - 5.0, ..b },
            n
        ));
        let outbound = Point {
            x: n.x + n.width / 2.0,
            y: n.y - 20.0,
        };
        assert!(!hugs(Point { y: n.y, ..outbound }, outbound, n));
    }
}
