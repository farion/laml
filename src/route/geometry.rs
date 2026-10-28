use super::labels::segment_hits_rect;
use super::{Point, Route};
use crate::geometry::BoxNode;

pub(super) fn center(node: &BoxNode) -> Point {
    Point {
        x: node.x + node.width / 2.0,
        y: node.y + node.height / 2.0,
    }
}
pub(super) fn port(node: &BoxNode, towards: Point) -> Point {
    let c = center(node);
    let dx = towards.x - c.x;
    let dy = towards.y - c.y;
    let scale = ((dx.abs() / (node.width / 2.0)).max(dy.abs() / (node.height / 2.0))).max(1e-9);
    Point {
        x: c.x + dx / scale,
        y: c.y + dy / scale,
    }
}
pub(super) fn intersection(a: Point, b: Point, node: &BoxNode) -> bool {
    // Open interior; touching a boundary at a valid port is allowed.
    let left = node.x + 1e-5;
    let right = node.x + node.width - 1e-5;
    let top = node.y + 1e-5;
    let bottom = node.y + node.height - 1e-5;
    if left >= right || top >= bottom {
        return false;
    }
    let mut low: f64 = 0.0;
    let mut high: f64 = 1.0;
    for (start, delta, min, max) in [(a.x, b.x - a.x, left, right), (a.y, b.y - a.y, top, bottom)] {
        if delta.abs() < 1e-9 {
            if start <= min || start >= max {
                return false;
            }
        } else {
            let t1 = (min - start) / delta;
            let t2 = (max - start) / delta;
            low = low.max(t1.min(t2));
            high = high.min(t1.max(t2));
        }
    }
    low < high - 1e-8 && high > 1e-8 && low < 1.0 - 1e-8
}
pub(super) fn crosses(a: Point, b: Point, c: Point, d: Point) -> bool {
    fn cross(a: Point, b: Point, c: Point) -> f64 {
        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
    }
    let (p, q, r, s) = (
        cross(a, b, c),
        cross(a, b, d),
        cross(c, d, a),
        cross(c, d, b),
    );
    if p.abs() < 1e-7 && q.abs() < 1e-7 {
        // Reject collinear overlap, not a single shared endpoint.
        let (u, v, w, z) = if (a.x - b.x).abs() > (a.y - b.y).abs() {
            (a.x, b.x, c.x, d.x)
        } else {
            (a.y, b.y, c.y, d.y)
        };
        return u.min(v).max(w.min(z)) < u.max(v).min(w.max(z)) - 1e-6;
    }
    p * q <= 1e-7
        && r * s <= 1e-7
        && a.x.min(b.x) <= c.x.max(d.x) + 1e-7
        && c.x.min(d.x) <= a.x.max(b.x) + 1e-7
        && a.y.min(b.y) <= c.y.max(d.y) + 1e-7
        && c.y.min(d.y) <= a.y.max(b.y) + 1e-7
}
fn same(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6
}
pub(super) fn shared_port_touch(
    pair: &[Point],
    old: &[Point],
    points: &[Point],
    from: usize,
    to: usize,
    route: &Route,
) -> bool {
    for common in [from, to] {
        let new_port = if common == from {
            points[0]
        } else {
            *points.last().unwrap()
        };
        let old_port = if common == route.from {
            route.points[0]
        } else if common == route.to {
            *route.points.last().unwrap()
        } else {
            continue;
        };
        if same(new_port, old_port)
            && pair.iter().any(|&p| same(p, new_port))
            && old.iter().any(|&p| same(p, old_port))
        {
            let np = if same(pair[0], new_port) {
                pair[1]
            } else {
                pair[0]
            };
            let op = if same(old[0], old_port) {
                old[1]
            } else {
                old[0]
            };
            let determinant = (np.x - new_port.x) * (op.y - old_port.y)
                - (np.y - new_port.y) * (op.x - old_port.x);
            if determinant.abs() > 1e-6 {
                return true;
            }
        }
    }
    false
}
pub(super) fn route_is_clear(points: &[Point], boxes: &[BoxNode], previous: &[Route]) -> bool {
    for pair in points.windows(2) {
        if boxes
            .iter()
            .any(|node| intersection(pair[0], pair[1], node))
        {
            return false;
        }
        for route in previous {
            if route.label.as_ref().is_some_and(|label| {
                segment_hits_rect(
                    pair[0],
                    pair[1],
                    label.x,
                    label.y,
                    label.width,
                    label.height,
                )
            }) {
                return false;
            }
            // Route/route crossings are examined by the bridge planner.
            // They cannot silently pass through this preliminary check.
        }
    }
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f64, y: f64) -> Point {
        Point { x, y }
    }
    #[test]
    fn t_junction_is_a_forbidden_unbridged_intersection() {
        assert!(crosses(p(0.0, 5.0), p(10.0, 5.0), p(5.0, 0.0), p(5.0, 5.0)));
    }
    #[test]
    fn collinear_overdraw_is_rejected() {
        assert!(crosses(
            p(0.0, 0.0),
            p(10.0, 0.0),
            p(5.0, 0.0),
            p(15.0, 0.0)
        ));
        assert!(!crosses(
            p(0.0, 0.0),
            p(10.0, 0.0),
            p(10.0, 0.0),
            p(15.0, 0.0)
        ));
    }
}
