//! A label must be perceptually closer to its owner than to another object.
use super::{Label, Point, Route, clearance, labels::segment_hits_rect};
use crate::geometry::BoxNode;

pub(super) const MARGIN: f64 = 2.0;

fn point_rect(p: Point, l: &Label) -> f64 {
    let dx = (l.x - p.x).max(0.0).max(p.x - l.x - l.width);
    let dy = (l.y - p.y).max(0.0).max(p.y - l.y - l.height);
    dx.hypot(dy)
}
fn point_segment(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let denominator = dx * dx + dy * dy;
    let t = if denominator < 1e-12 {
        0.0
    } else {
        ((p.x - a.x) * dx + (p.y - a.y) * dy) / denominator
    }
    .clamp(0.0, 1.0);
    (p.x - a.x - t * dx).hypot(p.y - a.y - t * dy)
}
/// Distance from the complete text envelope, not just its centre.
pub(super) fn segment_distance(label: &Label, a: Point, b: Point) -> f64 {
    if segment_hits_rect(a, b, label.x, label.y, label.width, label.height) {
        return 0.0;
    }
    let mut distance = point_rect(a, label).min(point_rect(b, label));
    for (x, y) in [
        (label.x, label.y),
        (label.x + label.width, label.y),
        (label.x, label.y + label.height),
        (label.x + label.width, label.y + label.height),
    ] {
        distance = distance.min(point_segment(Point { x, y }, a, b));
    }
    distance
}
pub(super) fn owner_distance(label: &Label, points: &[Point]) -> f64 {
    points
        .windows(2)
        .map(|p| segment_distance(label, p[0], p[1]))
        .fold(f64::INFINITY, f64::min)
}
fn rect_distance(label: &Label, x: f64, y: f64, w: f64, h: f64) -> f64 {
    let dx = (x - label.x - label.width).max(0.0).max(label.x - x - w);
    let dy = (y - label.y - label.height).max(0.0).max(label.y - y - h);
    dx.hypot(dy)
}
/// Visible foreign objects, prepared once for all label options of a path.
pub(super) struct Objects {
    rectangles: Vec<(f64, f64, f64, f64)>,
    segments: Vec<(Point, Point)>,
}
impl Objects {
    pub(super) fn new(boxes: &[BoxNode]) -> Self {
        let mut result = Self {
            rectangles: Vec::new(),
            segments: Vec::new(),
        };
        for n in boxes {
            if !n.is_container {
                result.rectangles.push((n.x, n.y, n.width, n.height));
                continue;
            }
            // A containing frame is not a filled rectangle. Its silhouette
            // and title remain visible foreign objects.
            result.segments.extend(
                clearance::outlines(n)
                    .into_iter()
                    .map(|(x, y, u, v)| (Point { x, y }, Point { x: u, y: v })),
            );
            if n.c4_boundary {
                let width = n
                    .lines
                    .iter()
                    .take(2)
                    .enumerate()
                    .filter_map(|(i, s)| {
                        crate::text::measure(s, if i == 0 { 18.0 } else { 12.0 }).ok()
                    })
                    .fold(0.0_f64, f64::max)
                    + 16.0;
                result
                    .rectangles
                    .push((n.x, n.y + n.height - 43.0, width, 43.0));
            } else {
                let title = n
                    .lines
                    .iter()
                    .filter_map(|s| crate::text::measure(s, n.font_size).ok())
                    .fold(0.0_f64, f64::max);
                let width = if matches!(
                    n.kind,
                    crate::syntax::NodeKind::Package | crate::syntax::NodeKind::Profile
                ) {
                    (title + n.padding * 2.0 + 12.0).min(n.width - 8.0)
                } else {
                    n.width
                };
                result.rectangles.push((n.x, n.y, width, n.port_start));
            }
        }
        result
    }
    fn clear_region(&self, label: &Label, points: &[Point], previous: &[Route]) -> bool {
        let required = owner_distance(label, points) + MARGIN - 1e-5;
        self.rectangles
            .iter()
            .all(|&(x, y, w, h)| rect_distance(label, x, y, w, h) >= required)
            && self
                .segments
                .iter()
                .all(|&(a, b)| segment_distance(label, a, b) >= required)
            && previous.iter().all(|r| {
                owner_distance(label, &r.points) - r.width / 2.0 >= required
                    && r.label
                        .iter()
                        .chain(&r.end_labels)
                        .all(|l| rect_distance(label, l.x, l.y, l.width, l.height) >= required)
                    && r.qualifiers
                        .iter()
                        .all(|q| rect_distance(label, q.x, q.y, q.width, q.height) >= required)
            })
    }
    pub(super) fn clear(&self, label: &Label, points: &[Point], previous: &[Route]) -> bool {
        // A close corner must not mask a caption centre owned by another box.
        self.clear_region(label, points, previous)
            && self.clear_region(&centre(label), points, previous)
            && previous.iter().all(|r| {
                r.label.as_ref().is_none_or(|old| {
                    [old, &centre(old)].iter().all(|region| {
                        rect_distance(region, label.x, label.y, label.width, label.height)
                            >= owner_distance(region, &r.points) + MARGIN - 1e-5
                    })
                })
            })
    }
}
fn centre(label: &Label) -> Label {
    let mut c = label.clone();
    c.x += label.width / 2.0;
    c.y += label.height / 2.0;
    c.width = 0.0;
    c.height = 0.0;
    c
}
#[cfg(test)]
pub(super) fn clear(
    label: &Label,
    points: &[Point],
    boxes: &[BoxNode],
    previous: &[Route],
) -> bool {
    Objects::new(boxes).clear(label, points, previous)
}

/// Later connections must not steal a label already placed by an earlier pass.
pub(super) fn steals(label: &Label, owner: &[Point], a: Point, b: Point, width: f64) -> bool {
    let centre = centre(label);
    [label, &centre]
        .iter()
        .any(|l| segment_distance(l, a, b) - width / 2.0 < owner_distance(l, owner) + MARGIN - 1e-5)
}

#[cfg(test)]
#[path = "label_ownership_tests.rs"]
mod tests;
