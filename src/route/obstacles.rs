//! Prepare notation-aware obstacle geometry once per relation, not per candidate.
use super::{Point, Route, clearance, geometry};
use crate::{
    geometry::{BoxNode, NodeShape},
    syntax::NodeKind,
};

pub(super) struct Obstacles {
    solids: Vec<BoxNode>,
    outlines: Vec<(f64, f64, f64, f64)>,
}

impl Obstacles {
    pub(super) fn new(boxes: &[BoxNode], from: usize, to: usize) -> Self {
        let mut result = Self {
            solids: Vec::new(),
            outlines: Vec::new(),
        };
        let inside = |i: usize, n: &BoxNode| {
            let b = &boxes[i];
            b.x > n.x
                && b.y > n.y
                && b.x + b.width < n.x + n.width
                && b.y + b.height < n.y + n.height
        };
        for (i, n) in boxes.iter().enumerate() {
            let endpoint = i == from || i == to;
            let owned = n.is_container && (inside(from, n) || inside(to, n));
            if (endpoint || n.is_container)
                && matches!(n.shape, NodeShape::Rectangle | NodeShape::Note)
            {
                result.outlines.extend(clearance::outlines(n));
            }
            let mut rect = |x, y, width, height| {
                let mut r = n.clone();
                r.x = x;
                r.y = y;
                r.width = width;
                r.height = height;
                r.kind = NodeKind::Generic;
                r.shape = NodeShape::Rectangle;
                r.deployment = false;
                result.solids.push(r);
            };
            if owned {
                if n.c4_boundary {
                    let w = n
                        .lines
                        .iter()
                        .take(2)
                        .enumerate()
                        .filter_map(|(i, s)| {
                            crate::text::measure(s, if i == 0 { 18.0 } else { 12.0 }).ok()
                        })
                        .fold(0.0_f64, f64::max)
                        + 16.0;
                    rect(n.x, n.y + n.height - 43.0, w, 43.0);
                } else if matches!(n.kind, NodeKind::Package | NodeKind::Profile) {
                    let title = n
                        .lines
                        .iter()
                        .filter_map(|s| crate::text::measure(s, n.font_size).ok())
                        .fold(0.0_f64, f64::max);
                    rect(
                        n.x,
                        n.y,
                        (title + n.padding * 2.0 + 12.0).min(n.width - 8.0),
                        n.padding + n.font_size * 1.25 * n.lines.len() as f64 + 4.0,
                    );
                } else {
                    rect(n.x, n.y, n.width, n.port_start);
                }
            } else if n.kind == NodeKind::Package && !n.c4_boundary {
                if let Ok((w, h)) = crate::geometry::package_outline::tab(n) {
                    let gap = if endpoint { 0.0 } else { clearance::GAP - 1e-5 };
                    rect(n.x - gap, n.y - gap, w + 2.0 * gap, h + 2.0 * gap);
                    rect(
                        n.x - gap,
                        n.y + h - gap,
                        n.width + 2.0 * gap,
                        n.height - h + 2.0 * gap,
                    );
                }
            } else if !endpoint
                && !n.is_container
                && matches!(n.shape, NodeShape::Rectangle | NodeShape::Note)
            {
                let gap = clearance::GAP - 1e-5;
                rect(
                    n.x - gap,
                    n.y - gap,
                    n.width + 2.0 * gap,
                    n.height + 2.0 * gap,
                );
            } else {
                result.solids.push(n.clone());
            }
        }
        result
    }
    pub(super) fn clear(&self, points: &[Point], previous: &[Route]) -> bool {
        if points
            .windows(2)
            .any(|p| (p[1].x - p[0].x).hypot(p[1].y - p[0].y) < clearance::GAP - 1e-5)
        {
            return false;
        }
        self.clear_segments(points, previous)
    }
    // Search-grid subdivisions may be short; only the final simplified path
    // needs the minimum segment length check above.
    pub(super) fn clear_segments(&self, points: &[Point], previous: &[Route]) -> bool {
        points.windows(2).all(|p| {
            !self
                .solids
                .iter()
                .any(|n| geometry::intersection(p[0], p[1], n))
                && self
                    .outlines
                    .iter()
                    .all(|&edge| clearance::away(p[0], p[1], edge))
        }) && geometry::route_is_clear(points, &[], previous)
            && points.windows(2).all(|p| {
                previous.iter().all(|r| {
                    r.points.windows(2).all(|q| {
                        let overlap = |a: f64, b: f64, c: f64, d: f64| {
                            a.min(b).max(c.min(d)) < a.max(b).min(c.max(d)) - 8.0
                        };
                        if (p[0].y - p[1].y).abs() < 1e-5 && (q[0].y - q[1].y).abs() < 1e-5 {
                            (p[0].y - q[0].y).abs() >= clearance::GAP - 1e-5
                                || !overlap(p[0].x, p[1].x, q[0].x, q[1].x)
                        } else if (p[0].x - p[1].x).abs() < 1e-5 && (q[0].x - q[1].x).abs() < 1e-5 {
                            (p[0].x - q[0].x).abs() >= clearance::GAP - 1e-5
                                || !overlap(p[0].y, p[1].y, q[0].y, q[1].y)
                        } else {
                            true
                        }
                    })
                })
            })
    }
}
