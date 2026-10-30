//! Deterministic crossing plan: a later relation receives an arc; the earlier
//! relation receives a gap long enough that the two visible strokes cannot
//! form a bare intersection. Unsupported angles/spacing reject the candidate.

use super::geometry::{crosses, shared_port_touch};
use super::{Bridge, Gap, Label, Point, Route};
use crate::geometry::BoxNode;

pub(super) struct BridgePlan {
    pub events: Vec<(usize, Bridge, Gap)>, // older route index, new bridge, old gap
}

fn cross(a: Point, b: Point) -> f64 {
    a.x * b.y - a.y * b.x
}
fn subtract(a: Point, b: Point) -> Point {
    Point {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}
fn length(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}
fn arc_point(center: Point, direction: Point, radius: f64, height: f64, t: f64) -> Point {
    let normal = Point {
        x: direction.y,
        y: -direction.x,
    };
    Point {
        x: center.x
            + direction.x * (2.0 * radius * (t - 0.5))
            + normal.x * (4.0 * height * t * (1.0 - t)),
        y: center.y
            + direction.y * (2.0 * radius * (t - 0.5))
            + normal.y * (4.0 * height * t * (1.0 - t)),
    }
}
/// The arc, not its original chord, crosses the underlying line. Place the
/// narrow underpass gap at that actual intersection, not at the chord crossing.
fn arc_intersection(
    center: Point,
    direction: Point,
    old_start: Point,
    old_dir: Point,
    radius: f64,
    height: f64,
) -> Point {
    let signed = |t: f64| {
        cross(
            old_dir,
            subtract(arc_point(center, direction, radius, height, t), old_start),
        )
    };
    let mut low = 0.0;
    let mut high = 1.0;
    let positive = signed(low) > 0.0;
    for _ in 0..52 {
        let mid = (low + high) / 2.0;
        if (signed(mid) > 0.0) == positive {
            low = mid;
        } else {
            high = mid;
        }
    }
    arc_point(center, direction, radius, height, (low + high) / 2.0)
}
struct ArcShape {
    center: Point,
    direction: Point,
    radius: f64,
    height: f64,
}
fn gap_half_for_clearance(
    arc: ArcShape,
    old_start: Point,
    old_dir: Point,
    old_len: f64,
    old_distance: f64,
    widths: f64,
) -> f64 {
    let threshold = widths / 2.0 + 2.0;
    let mut half = threshold + 2.0;
    for i in 0..=512 {
        let point = arc_point(
            arc.center,
            arc.direction,
            arc.radius,
            arc.height,
            i as f64 / 512.0,
        );
        let relative = subtract(point, old_start);
        let distance_from_line = cross(old_dir, relative).abs() / old_len;
        if distance_from_line <= threshold {
            let along = (relative.x * old_dir.x + relative.y * old_dir.y) / old_len;
            half = half.max((along - old_distance).abs() + 2.0);
        }
    }
    half + 5.0 // visible whitespace between the two antialiased strokes
}
fn overlaps_region(label: &Label, point: Point, radius: f64) -> bool {
    label.x < point.x + radius
        && label.x + label.width > point.x - radius
        && label.y < point.y + radius
        && label.y + label.height > point.y - radius
}

pub(super) fn plan(
    points: &[Point],
    boxes: &[BoxNode],
    previous: &[Route],
    from: usize,
    to: usize,
    width: f64,
) -> Option<BridgePlan> {
    let mut events: Vec<(usize, Bridge, Gap)> = Vec::new();
    for (segment, pair) in points.windows(2).enumerate() {
        let new_dir = subtract(pair[1], pair[0]);
        let new_len = length(pair[0], pair[1]);
        if new_len < 1e-6 {
            continue;
        }
        for (older_index, route) in previous.iter().enumerate() {
            for (old_segment, old) in route.points.windows(2).enumerate() {
                if !crosses(pair[0], pair[1], old[0], old[1]) {
                    continue;
                }
                if shared_port_touch(pair, old, points, from, to, route) {
                    continue;
                }
                let old_dir = subtract(old[1], old[0]);
                let old_len = length(old[0], old[1]);
                let determinant = cross(new_dir, old_dir);
                if old_len < 1e-6 || determinant.abs() / (new_len * old_len) < 0.5 {
                    return None; // T-junction, collinear overlap or shallow crossing
                }
                let difference = subtract(old[0], pair[0]);
                let t = cross(difference, old_dir) / determinant;
                let u = cross(difference, new_dir) / determinant;
                if !(1e-6..1.0 - 1e-6).contains(&t) || !(1e-6..1.0 - 1e-6).contains(&u) {
                    return None; // A bend or endpoint cannot host a clean bridge.
                }
                let center = Point {
                    x: pair[0].x + t * new_dir.x,
                    y: pair[0].y + t * new_dir.y,
                };
                let radius = (3.0 * width).max(8.0);
                let height = (4.0 * width.max(route.width)).max(8.0);
                let new_distance = t * new_len;
                let unit = Point {
                    x: new_dir.x / new_len,
                    y: new_dir.y / new_len,
                };
                let actual = arc_intersection(center, unit, old[0], old_dir, radius, height);
                let old_distance = (actual.x - old[0].x) * old_dir.x / old_len
                    + (actual.y - old[0].y) * old_dir.y / old_len;
                let half = gap_half_for_clearance(
                    ArcShape {
                        center,
                        direction: unit,
                        radius,
                        height,
                    },
                    old[0],
                    old_dir,
                    old_len,
                    old_distance,
                    width + route.width,
                );
                if new_distance < radius + 24.0
                    || new_len - new_distance < radius + 24.0
                    || old_distance < half + 24.0
                    || old_len - old_distance < half + 24.0
                {
                    return None;
                }
                // No hump or gap may obscure nodes, a label, or an existing
                // bridge/gap. This is conservative; the router can try a detour.
                let region = radius + height + width.max(route.width) + 4.0;
                if boxes.iter().any(|b| {
                    center.x + region > b.x
                        && center.x - region < b.x + b.width
                        && center.y + region > b.y
                        && center.y - region < b.y + b.height
                }) {
                    return None;
                }
                if previous.iter().any(|r| {
                    r.label
                        .as_ref()
                        .is_some_and(|l| overlaps_region(l, center, region))
                }) {
                    return None;
                }
                if route.bridges.iter().any(|b| {
                    b.segment == old_segment
                        && (b.distance - old_distance).abs() < b.radius + half + 8.0
                }) || route.gaps.iter().any(|g| {
                    g.segment == old_segment
                        && (g.distance - old_distance).abs() < g.half + half + 8.0
                }) {
                    return None;
                }
                if events.iter().any(|(index, bridge, gap)| {
                    (bridge.segment == segment
                        && (bridge.distance - new_distance).abs() < bridge.radius + radius + 8.0)
                        || (*index == older_index
                            && gap.segment == old_segment
                            && (gap.distance - old_distance).abs() < gap.half + half + 8.0)
                }) {
                    return None;
                }
                events.push((
                    older_index,
                    Bridge {
                        segment,
                        distance: new_distance,
                        radius,
                        height,
                    },
                    Gap {
                        segment: old_segment,
                        distance: old_distance,
                        half,
                    },
                ));
            }
        }
    }
    Some(BridgePlan { events })
}

impl BridgePlan {
    pub fn avoids_label(&self, points: &[Point], label: &Option<Label>) -> bool {
        let Some(label) = label else {
            return true;
        };
        !self.events.iter().any(|(_, bridge, _)| {
            let pair = &points[bridge.segment..=bridge.segment + 1];
            let length = length(pair[0], pair[1]);
            let t = bridge.distance / length;
            let center = Point {
                x: pair[0].x + t * (pair[1].x - pair[0].x),
                y: pair[0].y + t * (pair[1].y - pair[0].y),
            };
            overlaps_region(label, center, bridge.radius + bridge.height + 4.0)
        })
    }

    pub fn apply(self, routed: &mut [Route]) -> Vec<Bridge> {
        let mut bridges = Vec::new();
        for (older, bridge, gap) in self.events {
            routed[older].gaps.push(gap);
            bridges.push(bridge);
        }
        bridges
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f64, y: f64) -> Point {
        Point { x, y }
    }
    #[test]
    fn perpendicular_center_crossing_belongs_to_later_route() {
        let first = Route {
            points: vec![p(0.0, 100.0), p(200.0, 100.0)],
            bridges: vec![],
            gaps: vec![],
            label: None,
            end_labels: Vec::new(),
            from: 0,
            to: 1,
            color: String::new(),
            width: 2.0,
            font_size: 14.0,
            bidirectional: false,
            marker_end: true,
            appearance: Default::default(),
        };
        let second = vec![p(100.0, 0.0), p(100.0, 200.0)];
        let planned = plan(&second, &[], &[first], 2, 3, 2.0).unwrap();
        assert_eq!(planned.events.len(), 1);
        assert_eq!(planned.events[0].1.segment, 0);
        assert_eq!(planned.events[0].2.segment, 0);
        assert!((planned.events[0].2.distance - 108.0).abs() < 1e-6);
        assert!(planned.events[0].2.half <= 15.0);
    }
    #[test]
    fn close_to_endpoint_is_not_silently_bridged() {
        let first = Route {
            points: vec![p(0.0, 100.0), p(200.0, 100.0)],
            bridges: vec![],
            gaps: vec![],
            label: None,
            end_labels: Vec::new(),
            from: 0,
            to: 1,
            color: String::new(),
            width: 2.0,
            font_size: 14.0,
            bidirectional: false,
            marker_end: true,
            appearance: Default::default(),
        };
        assert!(plan(&[p(100.0, 90.0), p(100.0, 200.0)], &[], &[first], 2, 3, 2.0).is_none());
    }
}
