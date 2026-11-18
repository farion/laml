//! Compact underpasses when a shared bridge cannot fit; never erase arrowheads.
use crate::route::{Gap, Point, Route};
fn endpoint(route: &Route, p: Point) -> bool {
    route
        .points
        .first()
        .into_iter()
        .chain(route.points.last())
        .any(|q| (p.x - q.x).hypot(p.y - q.y) < 0.01)
}
fn cuts(route: &Route, p: Point, radius: f64) -> Option<Vec<Gap>> {
    let mut result = vec![];
    for (segment, arm) in route.points.windows(2).enumerate() {
        let (dx, dy) = (arm[1].x - arm[0].x, arm[1].y - arm[0].y);
        let length = dx.hypot(dy);
        if length < 1e-6 {
            continue;
        }
        let along = ((p.x - arm[0].x) * dx + (p.y - arm[0].y) * dy) / length;
        let away = ((p.x - arm[0].x) * dy - (p.y - arm[0].y) * dx).abs() / length;
        if away >= radius {
            continue;
        }
        let half = (radius * radius - away * away).sqrt();
        let lo = (along - half).max(0.0);
        let hi = (along + half).min(length);
        if hi <= lo + 1e-6 {
            continue;
        }
        let head = if route.appearance.end == crate::syntax::EndSymbol::Arrow {
            14.0
        } else {
            3.0
        };
        if segment == 0 && lo < 3.0 || segment + 2 == route.points.len() && hi > length - head {
            return None;
        }
        if route.bridges.iter().any(|b| {
            b.segment == segment && b.distance + b.radius > lo && b.distance - b.radius < hi
        }) {
            return None;
        }
        result.push(Gap {
            segment,
            distance: (lo + hi) / 2.0,
            half: (hi - lo) / 2.0,
        });
    }
    (!result.is_empty()).then_some(result)
}
fn apply(route: &mut Route, cuts: Vec<Gap>) {
    route.gaps.extend(cuts);
    route.gaps.sort_by(|a, b| {
        a.segment
            .cmp(&b.segment)
            .then((a.distance - a.half).total_cmp(&(b.distance - b.half)))
    });
    let mut merged: Vec<Gap> = vec![];
    for gap in route.gaps.drain(..) {
        if let Some(last) = merged.last_mut()
            && last.segment == gap.segment
            && last.distance + last.half >= gap.distance - gap.half
        {
            let lo = last.distance - last.half;
            let hi = (last.distance + last.half).max(gap.distance + gap.half);
            last.distance = (lo + hi) / 2.0;
            last.half = (hi - lo) / 2.0;
        } else {
            merged.push(gap);
        }
    }
    route.gaps = merged;
}
pub(super) fn cut(route: &mut Route, previous: &mut [Route]) -> Result<(), String> {
    for old in previous {
        let visible = if old.bridges.is_empty() && old.gaps.is_empty() {
            vec![old.points.clone()]
        } else {
            crate::svg::route_pick_paths(old, &[], crate::theme::Theme::Plain)?
        };
        let mut intersections = vec![];
        for pair in route.points.windows(2) {
            let (dx, dy) = (pair[1].x - pair[0].x, pair[1].y - pair[0].y);
            for path in &visible {
                for arm in path.windows(2) {
                    let (ox, oy) = (arm[1].x - arm[0].x, arm[1].y - arm[0].y);
                    let det = dx * oy - dy * ox;
                    if det.abs() < 1e-6 {
                        continue;
                    }
                    let (rx, ry) = (arm[0].x - pair[0].x, arm[0].y - pair[0].y);
                    let t = (rx * oy - ry * ox) / det;
                    let u = (rx * dy - ry * dx) / det;
                    if !(-1e-6..=1.0 + 1e-6).contains(&t) || !(-1e-6..=1.0 + 1e-6).contains(&u) {
                        continue;
                    }
                    let p = Point {
                        x: pair[0].x + t * dx,
                        y: pair[0].y + t * dy,
                    };
                    if endpoint(route, p) && endpoint(old, p) {
                        continue;
                    }
                    let radius =
                        ((route.width + old.width) / 2.0 + 3.0) * dx.hypot(dy) * ox.hypot(oy)
                            / det.abs();
                    intersections.push((p, radius));
                }
            }
        }
        for (p, radius) in intersections {
            if old.bridges.is_empty()
                && let Some(gaps) = cuts(old, p, radius)
            {
                apply(old, gaps);
            } else if let Some(gaps) = cuts(route, p, radius) {
                apply(route, gaps);
            } else {
                return Err(format!(
                    "message crossing near an arrowhead needs more clearance (source line {})",
                    route.source_line
                ));
            }
        }
    }
    Ok(())
}
