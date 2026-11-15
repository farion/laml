//! Last-resort crossings use a small underpass instead of requiring a bridge hump.
use super::{Gap, Point, Route, bridges::BridgePlan, geometry};

pub(super) fn plan(
    points: &[Point],
    previous: &[Route],
    from: usize,
    to: usize,
    width: f64,
) -> Option<BridgePlan> {
    let mut cuts = Vec::new();
    for pair in points.windows(2) {
        let dx = pair[1].x - pair[0].x;
        let dy = pair[1].y - pair[0].y;
        let len = dx.hypot(dy);
        for (index, route) in previous.iter().enumerate() {
            for old in route.points.windows(2) {
                if !geometry::crosses(pair[0], pair[1], old[0], old[1])
                    || geometry::shared_port_touch(pair, old, points, from, to, route)
                {
                    continue;
                }
                let ox = old[1].x - old[0].x;
                let oy = old[1].y - old[0].y;
                let old_len = ox.hypot(oy);
                let det = dx * oy - dy * ox;
                if len < 1e-6 || old_len < 1e-6 || det.abs() < 1e-6 {
                    return None;
                }
                let t = ((old[0].x - pair[0].x) * oy - (old[0].y - pair[0].y) * ox) / det;
                let center = Point {
                    x: pair[0].x + t * dx,
                    y: pair[0].y + t * dy,
                };
                let radius = ((width + route.width) / 2.0 + 3.0) * len * old_len / det.abs();
                // Cut both arms when the intersection lies at/near an old bend.
                for (segment, arm) in route.points.windows(2).enumerate() {
                    let ax = arm[1].x - arm[0].x;
                    let ay = arm[1].y - arm[0].y;
                    let arm_len = ax.hypot(ay);
                    if arm_len < 1e-6 {
                        continue;
                    }
                    let along = ((center.x - arm[0].x) * ax + (center.y - arm[0].y) * ay) / arm_len;
                    let perpendicular =
                        ((center.x - arm[0].x) * ay - (center.y - arm[0].y) * ax).abs() / arm_len;
                    if perpendicular >= radius {
                        continue;
                    }
                    let half = (radius * radius - perpendicular * perpendicular).sqrt();
                    if along + half <= 0.0 || along - half >= arm_len {
                        continue;
                    }
                    // Never erase an endpoint or its notation to manufacture a crossing.
                    let start =
                        geometry::marker_radius(route.appearance.start, route.width).max(2.0);
                    let end = geometry::marker_radius(route.appearance.end, route.width).max(2.0);
                    if segment == 0 && along - half < start
                        || segment + 2 == route.points.len() && along + half > arm_len - end
                    {
                        return None;
                    }
                    let lo = (along - half).max(0.0);
                    let hi = (along + half).min(arm_len);
                    if route.bridges.iter().any(|b| {
                        b.segment == segment
                            && b.distance + b.radius > lo
                            && b.distance - b.radius < hi
                    }) {
                        return None;
                    }
                    cuts.push((
                        index,
                        Gap {
                            segment,
                            distance: (lo + hi) / 2.0,
                            half: (hi - lo) / 2.0,
                        },
                    ));
                }
            }
        }
    }
    Some(BridgePlan {
        events: Vec::new(),
        cuts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f64, y: f64) -> Point {
        Point { x, y }
    }
    fn old(points: Vec<Point>) -> Route {
        Route {
            source_line: 0,
            points,
            qualifiers: vec![],
            bridges: vec![],
            gaps: vec![],
            label: None,
            end_labels: vec![],
            from: 0,
            to: 1,
            color: "black".into(),
            width: 2.0,
            font_size: 14.0,
            bidirectional: false,
            marker_end: false,
            appearance: Default::default(),
        }
    }
    #[test]
    fn crossing_at_bend_uses_clipped_gaps_on_both_arms_and_renders() {
        let mut previous = vec![old(vec![p(0., 100.), p(100., 100.), p(100., 200.)])];
        let crossing = [p(50., 50.), p(150., 150.)];
        assert!(super::super::bridges::plan(&crossing, &[], &previous, 2, 3, 2.).is_none());
        let plan = plan(&crossing, &previous, 2, 3, 2.).unwrap();
        assert!(plan.cuts.len() >= 2);
        assert!(plan.apply(&mut previous).is_empty());
        assert_eq!(
            previous[0].gaps.len(),
            2,
            "duplicate bend intersections must merge"
        );
        let fragments = crate::svg_paths::fragments(&previous[0]).unwrap();
        assert_eq!(
            fragments.len(),
            2,
            "do not paint a round-capped zero-length dot at the cut bend"
        );
    }
    #[test]
    fn close_crossing_does_not_need_room_for_a_hump() {
        let mut previous = vec![old(vec![p(0., 100.), p(200., 100.)])];
        let crossing = [p(20., 80.), p(20., 120.)];
        assert!(super::super::bridges::plan(&crossing, &[], &previous, 2, 3, 2.).is_none());
        plan(&crossing, &previous, 2, 3, 2.)
            .unwrap()
            .apply(&mut previous);
        assert_eq!(previous[0].gaps.len(), 1);
        crate::svg_paths::fragments(&previous[0]).unwrap();
    }
    #[test]
    fn overlap_and_erasing_an_arrowhead_remain_invalid() {
        let mut previous = vec![old(vec![p(0., 100.), p(200., 100.)])];
        assert!(plan(&[p(40., 100.), p(150., 100.)], &previous, 2, 3, 2.).is_none());
        previous[0].appearance.end = crate::syntax::EndSymbol::Arrow;
        assert!(plan(&[p(195., 80.), p(195., 120.)], &previous, 2, 3, 2.).is_none());
    }
}
