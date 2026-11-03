//! Rank bounded routing candidates by turns, visible crossings, then path length.
//! Feasibility (obstacles, labels, mandatory sides and bridges) remains checked
//! by the normal router before accepting the first candidate in this order.
use super::{Point, Route, geometry, outline_clearance};
use crate::geometry::BoxNode;

fn bends(points: &[Point]) -> usize {
    points
        .windows(3)
        .filter(|triple| {
            let a = triple[0];
            let b = triple[1];
            let c = triple[2];
            let (dx1, dy1) = (b.x - a.x, b.y - a.y);
            let (dx2, dy2) = (c.x - b.x, c.y - b.y);
            let norm = dx1.hypot(dy1) * dx2.hypot(dy2);
            norm > 1e-6
                && ((dx1 * dy2 - dy1 * dx2).abs() > 1e-6 * norm || dx1 * dx2 + dy1 * dy2 < 0.0)
        })
        .count()
}
fn crossings(points: &[Point], routed: &[Route], from: usize, to: usize) -> usize {
    points
        .windows(2)
        .map(|pair| {
            routed
                .iter()
                .map(|route| {
                    route
                        .points
                        .windows(2)
                        .filter(|old| {
                            geometry::crosses(pair[0], pair[1], old[0], old[1])
                                && !geometry::shared_port_touch(pair, old, points, from, to, route)
                        })
                        .count()
                })
                .sum::<usize>()
        })
        .sum()
}
fn length(points: &[Point]) -> f64 {
    points
        .windows(2)
        .map(|pair| (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y))
        .sum()
}
pub(super) fn total(routes: &[Route]) -> (usize, usize, f64) {
    (
        routes.iter().map(|route| route.bridges.len()).sum(),
        routes.iter().map(|route| bends(&route.points)).sum(),
        routes.iter().map(|route| length(&route.points)).sum(),
    )
}

pub(super) fn better_than(ranked: &[Route], stable: &[Route]) -> bool {
    let a = total(ranked);
    let b = total(stable);
    (a.1, a.0) < (b.1, b.0) || (a.1, a.0) == (b.1, b.0) && a.2 < b.2 - 1e-6
}

fn border_hugs(points: &[Point], boxes: &[BoxNode], from: usize, to: usize) -> usize {
    points
        .windows(2)
        .map(|pair| {
            boxes
                .iter()
                .enumerate()
                .filter(|(index, node)| {
                    *index != from
                        && *index != to
                        && outline_clearance::hugs(pair[0], pair[1], node)
                })
                .count()
        })
        .sum()
}

pub(super) fn visibly_clear(routes: &[Route], boxes: &[BoxNode]) -> bool {
    routes
        .iter()
        .all(|route| border_hugs(&route.points, boxes, route.from, route.to) == 0)
}

pub(super) fn order(
    options: &mut [Vec<Point>],
    routed: &[Route],
    boxes: &[BoxNode],
    from: usize,
    to: usize,
) {
    options.sort_by(|a, b| {
        border_hugs(a, boxes, from, to)
            .cmp(&border_hugs(b, boxes, from, to))
            .then_with(|| bends(a).cmp(&bends(b)))
            .then_with(|| crossings(a, routed, from, to).cmp(&crossings(b, routed, from, to)))
            .then_with(|| length(a).total_cmp(&length(b)))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f64, y: f64) -> Point {
        Point { x, y }
    }
    #[test]
    fn ranks_unobstructed_straight_and_short_routes_first() {
        let mut options = vec![
            vec![p(0.0, 0.0), p(0.0, 20.0), p(120.0, 20.0), p(120.0, 0.0)],
            vec![p(0.0, 0.0), p(120.0, 0.0)],
            vec![p(0.0, 0.0), p(0.0, 10.0), p(120.0, 10.0), p(120.0, 0.0)],
        ];
        order(&mut options, &[], &[], 0, 1);
        assert_eq!(options[0].len(), 2);
        assert_eq!(options[1][1].y, 10.0);
    }
}
