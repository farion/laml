use super::labels::segment_hits_rect;
use super::{Point, Route};
use crate::geometry::{BoxNode, NodeShape};
fn compact_radius(node: &BoxNode) -> Option<f64> {
    match node.kind {
        crate::syntax::NodeKind::Association | crate::syntax::NodeKind::Junction => Some(12.0),
        crate::syntax::NodeKind::Lollipop | crate::syntax::NodeKind::Socket => Some(10.0),
        crate::syntax::NodeKind::StateInitial | crate::syntax::NodeKind::ActivityInitial => {
            Some(9.0)
        }
        crate::syntax::NodeKind::StateFinal
        | crate::syntax::NodeKind::StateChoice
        | crate::syntax::NodeKind::ActivityFinal
        | crate::syntax::NodeKind::ActivityFlowFinal
        | crate::syntax::NodeKind::ActivityDecision
        | crate::syntax::NodeKind::ActivityMerge => Some(12.0),
        _ => None,
    }
}
pub(super) fn center(node: &BoxNode) -> Point {
    Point {
        x: node.x + compact_radius(node).unwrap_or(node.width / 2.0),
        y: node.y
            + if node.kind == crate::syntax::NodeKind::UseCaseActor {
                crate::geometry::usecase::FIGURE_CENTER_Y
            } else {
                node.height / 2.0
            },
    }
}
pub(super) fn member_port(node: &BoxNode, member: &str, towards: Point) -> Option<Point> {
    if let Some(port) = node.ports.iter().find(|port| port.id == member) {
        return Some(Point {
            x: port.x,
            y: port.y,
        });
    }
    let line_height = node.font_size * 1.25;
    let mut y = node.y + node.padding + node.top_band + line_height * node.lines.len() as f64 + 4.0;
    for (part, row) in node.body.iter().zip(&node.body_styles) {
        match part {
            crate::syntax::NodePart::Separator => y += 8.0,
            crate::syntax::NodePart::Member { .. } => {
                if node.matches_member(part, member) {
                    return Some(Point {
                        x: if towards.x >= node.x + node.width / 2.0 {
                            node.x + node.width
                        } else {
                            node.x
                        },
                        y: y + row.font_size * 1.25 / 2.0,
                    });
                }
                y += row.font_size * 1.25;
            }
        }
    }
    None
}
fn note_fold(node: &BoxNode) -> f64 {
    if node.note_card {
        0.0
    } else {
        12.0_f64.min(node.width / 4.0).min(node.height / 4.0)
    }
}

pub(super) fn port(node: &BoxNode, towards: Point) -> Point {
    if node.kind == crate::syntax::NodeKind::Package
        && !node.c4_boundary
        && let Some(hit) = super::package_outline::port(node, towards)
    {
        return hit;
    }
    let c = center(node);
    let dx = towards.x - c.x;
    let dy = towards.y - c.y;
    let (rx, ry) = if node.kind == crate::syntax::NodeKind::UseCaseActor {
        (
            crate::geometry::usecase::DOCK_RX,
            crate::geometry::usecase::DOCK_RY,
        )
    } else if let Some(radius) = compact_radius(node) {
        (radius, radius)
    } else {
        ((node.width / 2.0).max(1e-9), (node.height / 2.0).max(1e-9))
    };
    let scale = match node.shape {
        _ if node.kind == crate::syntax::NodeKind::UseCaseActor => {
            ((dx / rx).powi(2) + (dy / ry).powi(2)).sqrt()
        }
        NodeShape::Rectangle | NodeShape::Note => (dx.abs() / rx).max(dy.abs() / ry),
        NodeShape::Ellipse => ((dx / rx).powi(2) + (dy / ry).powi(2)).sqrt(),
        NodeShape::Diamond => dx.abs() / rx + dy.abs() / ry,
    }
    .max(1e-9);
    let mut t = 1.0 / scale;
    if node.shape == NodeShape::Note && dx - dy > 1e-9 {
        let boundary = node.x + node.width - note_fold(node) - node.y;
        let diagonal = (boundary - c.x + c.y) / (dx - dy);
        if diagonal > 0.0 {
            t = t.min(diagonal);
        }
    }
    Point {
        x: c.x + dx * t,
        y: c.y + dy * t,
    }
}
fn ellipse_intersection(a: Point, b: Point, c: Point, rx: f64, ry: f64) -> bool {
    let (ax, ay) = ((a.x - c.x) / rx, (a.y - c.y) / ry);
    let (dx, dy) = ((b.x - a.x) / rx, (b.y - a.y) / ry);
    let denominator = dx * dx + dy * dy;
    let t = if denominator < 1e-12 {
        0.0
    } else {
        (-(ax * dx + ay * dy) / denominator).clamp(0.0, 1.0)
    };
    (ax + t * dx).powi(2) + (ay + t * dy).powi(2) < 1.0 - 1e-5
}

pub(super) fn intersection(a: Point, b: Point, node: &BoxNode) -> bool {
    if node.kind == crate::syntax::NodeKind::UseCaseActor {
        let feet = crate::geometry::usecase::FOOT_Y;
        // Both docking and collision use the same invisible ellipse; the
        // caption remains a separate obstacle below the figure.
        return ellipse_intersection(
            a,
            b,
            center(node),
            crate::geometry::usecase::DOCK_RX,
            crate::geometry::usecase::DOCK_RY,
        ) || segment_hits_rect(
            a,
            b,
            node.x,
            node.y + feet + 7.0,
            node.width,
            (node.height - feet - 7.0).max(0.0),
        );
    }
    if node.kind == crate::syntax::NodeKind::Package && !node.c4_boundary {
        return super::package_outline::intersects(a, b, node);
    }
    let c = center(node);
    let rx = if let Some(radius) = compact_radius(node) {
        radius
    } else {
        (node.width / 2.0).max(1e-9)
    };
    let ry = if let Some(radius) = compact_radius(node) {
        radius
    } else {
        (node.height / 2.0).max(1e-9)
    };
    match node.shape {
        NodeShape::Ellipse => {
            if matches!(
                node.kind,
                crate::syntax::NodeKind::Lollipop | crate::syntax::NodeKind::Socket
            ) && segment_hits_rect(a, b, node.x + 26.0, node.y, node.width - 26.0, node.height)
            {
                return true;
            }
            return ellipse_intersection(a, b, c, rx, ry);
        }
        NodeShape::Diamond => {
            if compact_radius(node).is_some()
                && !node.label.is_empty()
                && segment_hits_rect(a, b, node.x + 30.0, node.y, node.width - 30.0, node.height)
            {
                return true;
            }
            let mut candidates = vec![0.0, 1.0];
            for (start, delta, target) in [(a.x, b.x - a.x, c.x), (a.y, b.y - a.y, c.y)] {
                if delta.abs() > 1e-9 {
                    let t = (target - start) / delta;
                    if (0.0..=1.0).contains(&t) {
                        candidates.push(t);
                    }
                }
            }
            return candidates.into_iter().any(|t| {
                let x = a.x + (b.x - a.x) * t;
                let y = a.y + (b.y - a.y) * t;
                (x - c.x).abs() / rx + (y - c.y).abs() / ry < 1.0 - 1e-5
            });
        }
        NodeShape::Rectangle | NodeShape::Note => {}
    }
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
    if node.shape == NodeShape::Note {
        let boundary = node.x + node.width - note_fold(node) - node.y;
        let start = a.x - a.y - boundary;
        let delta = b.x - a.x - b.y + a.y;
        if delta.abs() < 1e-9 {
            if start >= -1e-5 {
                return false;
            }
        } else {
            let limit = (-1e-5 - start) / delta;
            if delta > 0.0 {
                high = high.min(limit);
            } else {
                low = low.max(limit);
            }
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
pub(super) fn marker_radius(symbol: crate::syntax::EndSymbol, stroke: f64) -> f64 {
    use crate::syntax::EndSymbol;
    (match symbol {
        EndSymbol::None => return 0.0,
        EndSymbol::Arrow => 12.0,
        EndSymbol::Triangle | EndSymbol::FilledTriangle => 14.0,
        EndSymbol::OpenDiamond | EndSymbol::FilledDiamond => 18.0,
    }) + stroke / 2.0
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
        if boxes.iter().any(|node| {
            let within = |p: Point| {
                p.x > node.x
                    && p.x < node.x + node.width
                    && p.y > node.y
                    && p.y < node.y + node.height
            };
            let own_container =
                node.is_container && (within(points[0]) || within(*points.last().unwrap()));
            if own_container {
                if node.c4_boundary {
                    let caption_width = node
                        .lines
                        .iter()
                        .take(2)
                        .enumerate()
                        .filter_map(|(i, line)| {
                            crate::text::measure(line, if i == 0 { 18.0 } else { 12.0 }).ok()
                        })
                        .fold(0.0_f64, f64::max)
                        + 16.0;
                    return super::labels::segment_hits_rect(
                        pair[0],
                        pair[1],
                        node.x,
                        node.y + node.height - 43.0,
                        caption_width,
                        43.0,
                    );
                }
                let title_width = node
                    .lines
                    .iter()
                    .filter_map(|line| crate::text::measure(line, node.font_size).ok())
                    .fold(0.0_f64, f64::max);
                let tab_width = if !matches!(
                    node.kind,
                    crate::syntax::NodeKind::Package | crate::syntax::NodeKind::Profile
                ) {
                    node.width
                } else {
                    (title_width + node.padding * 2.0 + 12.0).min(node.width - 8.0)
                };
                let tab_height = if !matches!(
                    node.kind,
                    crate::syntax::NodeKind::Package | crate::syntax::NodeKind::Profile
                ) {
                    node.port_start
                } else {
                    node.padding + node.font_size * 1.25 * node.lines.len() as f64 + 4.0
                };
                super::labels::segment_hits_rect(
                    pair[0], pair[1], node.x, node.y, tab_width, tab_height,
                )
            } else {
                intersection(pair[0], pair[1], node)
                    || (matches!(node.kind, crate::syntax::NodeKind::Package) && !node.c4_boundary
                        || node.deployment)
                        && super::outline_clearance::hugs(pair[0], pair[1], node)
            }
        }) {
            return false;
        }
        for route in previous {
            if super::marker_clearance::hits_segment(pair[0], pair[1], route, boxes, true)
                || super::marker_clearance::hits_segment(pair[0], pair[1], route, boxes, false)
                || route.qualifiers.iter().any(|qualifier| {
                    segment_hits_rect(
                        pair[0],
                        pair[1],
                        qualifier.x,
                        qualifier.y,
                        qualifier.width,
                        qualifier.height,
                    )
                })
                || route.bridges.iter().any(|bridge| {
                    let segment = &route.points[bridge.segment..=bridge.segment + 1];
                    let length = (segment[1].x - segment[0].x).hypot(segment[1].y - segment[0].y);
                    let t = bridge.distance / length;
                    let center = Point {
                        x: segment[0].x + t * (segment[1].x - segment[0].x),
                        y: segment[0].y + t * (segment[1].y - segment[0].y),
                    };
                    let reach = bridge.radius + bridge.height + route.width;
                    segment_hits_rect(
                        pair[0],
                        pair[1],
                        center.x - reach,
                        center.y - reach,
                        2.0 * reach,
                        2.0 * reach,
                    )
                })
                || route
                    .label
                    .iter()
                    .chain(route.end_labels.iter())
                    .any(|label| {
                        segment_hits_rect(
                            pair[0],
                            pair[1],
                            label.x,
                            label.y,
                            label.width,
                            label.height,
                        )
                    })
            {
                return false;
            }
            // Route/route crossings are examined by the bridge planner.
            // They cannot silently pass through this preliminary check.
        }
    }
    true
}
#[cfg(test)]
#[path = "geometry_tests.rs"]
mod tests;
