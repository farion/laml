//! Deterministic BPMN attachment routing. Candidates are ranked by clearance,
//! shared-segment avoidance, crossings, bends and length (in that order).
use super::ports::Attach;
use super::{Document, Geometry};
use crate::bpmn::{Edge, EdgeKind, Kind, Side};

pub(super) type Point = (f64, f64);
pub(super) struct Prior {
    pub points: Vec<Point>,
    pub kind: EdgeKind,
    pub from: String,
}
pub(super) struct Plan {
    pub points: Vec<Point>,
    pub label: Point,
}
fn stub(p: Attach, length: f64) -> Point {
    match p.side {
        Side::Left => (p.x - length, p.y),
        Side::Right => (p.x + length, p.y),
        Side::Top => (p.x, p.y - length),
        Side::Bottom => (p.x, p.y + length),
    }
}
fn compact(mut points: Vec<Point>) -> Vec<Point> {
    points.dedup_by(|a, b| (a.0 - b.0).abs() < 0.05 && (a.1 - b.1).abs() < 0.05);
    let mut i = 1;
    while i + 1 < points.len() {
        if ((points[i - 1].0 - points[i].0).abs() < 0.05
            && (points[i].0 - points[i + 1].0).abs() < 0.05
            || (points[i - 1].1 - points[i].1).abs() < 0.05
                && (points[i].1 - points[i + 1].1).abs() < 0.05)
            && (points[i].0 - points[i - 1].0) * (points[i + 1].0 - points[i].0)
                + (points[i].1 - points[i - 1].1) * (points[i + 1].1 - points[i].1)
                > 0.0
        {
            points.remove(i);
        } else {
            i += 1;
        }
    }
    points
}
fn hits_box(a: Point, b: Point, rect: super::super::geometry::BoxAt, margin: f64) -> bool {
    let (left, right) = (
        rect.x - rect.w / 2.0 - margin,
        rect.x + rect.w / 2.0 + margin,
    );
    let (top, bottom) = (
        rect.y - rect.h / 2.0 - margin,
        rect.y + rect.h / 2.0 + margin,
    );
    if (a.0 - b.0).abs() < 0.05 {
        a.0 > left && a.0 < right && a.1.max(b.1) > top && a.1.min(b.1) < bottom
    } else if (a.1 - b.1).abs() < 0.05 {
        a.1 > top && a.1 < bottom && a.0.max(b.0) > left && a.0.min(b.0) < right
    } else {
        true
    }
}
fn crossing(a: Point, b: Point, c: Point, d: Point) -> Option<Point> {
    if (a.0 - b.0).abs() < 0.05 && (c.1 - d.1).abs() < 0.05 {
        let p = (a.0, c.1);
        (p.1 > a.1.min(b.1) + 9.0
            && p.1 < a.1.max(b.1) - 9.0
            && p.0 > c.0.min(d.0) + 9.0
            && p.0 < c.0.max(d.0) - 9.0)
            .then_some(p)
    } else if (a.1 - b.1).abs() < 0.05 && (c.0 - d.0).abs() < 0.05 {
        let p = (c.0, a.1);
        (p.0 > a.0.min(b.0) + 9.0
            && p.0 < a.0.max(b.0) - 9.0
            && p.1 > c.1.min(d.1) + 9.0
            && p.1 < c.1.max(d.1) - 9.0)
            .then_some(p)
    } else {
        None
    }
}
fn overlap(a: Point, b: Point, c: Point, d: Point, margin: f64) -> bool {
    if (a.0 - b.0).abs() < 0.05 && (c.0 - d.0).abs() < 0.05 {
        (a.0 - c.0).abs() < margin
            && a.1.max(b.1).min(c.1.max(d.1)) - a.1.min(b.1).max(c.1.min(d.1)) > 7.0
    } else if (a.1 - b.1).abs() < 0.05 && (c.1 - d.1).abs() < 0.05 {
        (a.1 - c.1).abs() < margin
            && a.0.max(b.0).min(c.0.max(d.0)) - a.0.min(b.0).max(c.0.min(d.0)) > 7.0
    } else {
        false
    }
}
fn score(
    doc: &Document,
    geom: &Geometry,
    e: &Edge,
    points: &[Point],
    previous: &[Prior],
) -> Option<f64> {
    let mut cost = 0.0;
    for triple in points.windows(3) {
        let first = (triple[1].0 - triple[0].0, triple[1].1 - triple[0].1);
        let second = (triple[2].0 - triple[1].0, triple[2].1 - triple[1].1);
        if first.0 * second.0 + first.1 * second.1 < -0.1
            && (first.0 * second.1 - first.1 * second.0).abs() < 0.1
        {
            return None;
        }
    }
    for (segment, leg) in points.windows(2).enumerate() {
        let (a, b) = (leg[0], leg[1]);
        if (a.0 - b.0).abs() > 0.05 && (a.1 - b.1).abs() > 0.05 {
            return None;
        }
        cost += (a.0 - b.0).abs() + (a.1 - b.1).abs();
        if e.kind == EdgeKind::Message
            && doc.direction != crate::syntax::FlowDirection::TopToBottom
            && (a.1 - b.1).abs() < 0.05
        {
            for pool in &geom.pools {
                if a.1 > pool.y + 30.0
                    && a.1 < pool.y + pool.h - 5.0
                    && [pool.x, pool.x + pool.w]
                        .iter()
                        .any(|&edge| a.0.min(b.0) < edge - 1.0 && a.0.max(b.0) > edge + 1.0)
                {
                    cost += 1500.0;
                }
            }
        }
        for (i, node) in doc.nodes.iter().enumerate() {
            if node.kind == crate::bpmn::Kind::Boundary
                && node.id != e.from
                && node.id != e.to
                && doc.edges.iter().any(|edge| {
                    edge.from == node.id && edge.kind == crate::bpmn::EdgeKind::Sequence
                })
                && (a.0 - b.0).abs() < 0.05
                && (a.0 - geom.nodes[i].x).abs() < 17.0
                && a.1.max(b.1) > geom.nodes[i].y + geom.nodes[i].h / 2.0 + 2.0
                && a.1.min(b.1) < geom.nodes[i].y + 350.0
            {
                return None;
            }
            if (segment == 0
                && (node.id == e.from
                    || doc.nodes.iter().find(|n| n.id == e.from).is_some_and(|n| {
                        n.kind == Kind::Boundary && n.attached.as_deref() == Some(node.id.as_str())
                    })))
                || (segment == points.len() - 2 && node.id == e.to)
            {
                continue;
            }
            if node.kind == Kind::Subprocess
                && doc.nodes.iter().any(|n| {
                    (n.id == e.from || n.id == e.to)
                        && n.parent.as_deref() == node.id.rsplit('.').next()
                })
            {
                continue;
            }
            if hits_box(
                a,
                b,
                geom.nodes[i],
                if node.id == e.from || node.id == e.to {
                    1.0
                } else {
                    7.0
                },
            ) {
                return None;
            }
        }
        for prior in previous {
            for old in prior.points.windows(2) {
                let margin = if matches!(e.kind, EdgeKind::Data | EdgeKind::Association)
                    && matches!(prior.kind, EdgeKind::Data | EdgeKind::Association)
                {
                    23.0
                } else if segment == 0
                    && prior.from == e.from
                    && e.kind == EdgeKind::Sequence
                    && doc.nodes.iter().find(|n| n.id == e.from).is_some_and(|n| {
                        matches!(
                            n.kind,
                            Kind::Exclusive | Kind::Parallel | Kind::Inclusive | Kind::EventBased
                        )
                    })
                {
                    5.0
                } else {
                    10.0
                };
                if overlap(a, b, old[0], old[1], margin) {
                    return None;
                }
                if crossing(a, b, old[0], old[1]).is_some() {
                    cost += 120.0;
                }
            }
        }
    }
    cost += (points.len().saturating_sub(2)) as f64 * 24.0;
    Some(cost)
}
fn label(doc: &Document, geom: &Geometry, points: &[Point], name: &str) -> Point {
    let text_width = crate::text::measure(name, 12.0).unwrap_or(40.0);
    let mut options: Vec<(f64, Point)> = Vec::new();
    for leg in points.windows(2) {
        let (a, b) = (leg[0], leg[1]);
        let horiz = (a.1 - b.1).abs() < 0.05;
        let length = if horiz {
            (a.0 - b.0).abs()
        } else {
            (a.1 - b.1).abs()
        };
        if length < (text_width + 20.0).max(45.0) {
            continue;
        }
        for fraction in [0.25, 0.5, 0.75] {
            let mid = (a.0 + (b.0 - a.0) * fraction, a.1 + (b.1 - a.1) * fraction);
            let candidate = if horiz {
                (mid.0, mid.1 - 10.0)
            } else {
                (mid.0 + text_width / 2.0 + 13.0, mid.1)
            };
            let label_box = super::super::geometry::BoxAt {
                x: candidate.0,
                y: candidate.1 - 5.0,
                w: text_width + 8.0,
                h: 18.0,
            };
            if doc.nodes.iter().enumerate().any(|(i, _)| {
                let q = geom.nodes[i];
                (q.x - label_box.x).abs() < (q.w + label_box.w) / 2.0 + 5.0
                    && (q.y - label_box.y).abs() < (q.h + label_box.h) / 2.0 + 5.0
            }) {
                continue;
            }
            let near_border = geom.pools.iter().any(|pool| {
                [pool.y, pool.y + 30.0, pool.y + pool.h]
                    .iter()
                    .any(|&y| (candidate.1 - y).abs() < 22.0)
                    || pool
                        .bands
                        .iter()
                        .any(|band| (candidate.1 - band.y).abs() < 22.0)
            });
            let score = length
                - (if near_border { 1000.0 } else { 0.0 })
                - (if horiz { 0.0 } else { 15.0 })
                - (fraction - 0.5_f64).abs() * 5.0;
            options.push((score, candidate));
        }
    }
    options
        .into_iter()
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map_or_else(
            || {
                let a = points[0];
                let b = points[points.len() - 1];
                ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0 - 10.0)
            },
            |(_, point)| point,
        )
}
pub(super) fn route(
    doc: &Document,
    geom: &Geometry,
    e: &Edge,
    source: Attach,
    target: Attach,
    preferred: Option<Vec<Point>>,
    previous: &[Prior],
) -> Result<Plan, String> {
    let short = |id: &str| {
        if doc.nodes.iter().find(|n| n.id == id).is_some_and(|n| {
            matches!(
                n.kind,
                crate::bpmn::Kind::Annotation | crate::bpmn::Kind::Data | crate::bpmn::Kind::Store
            )
        }) {
            12.0
        } else {
            22.0
        }
    };
    let (s, t) = (stub(source, short(&e.from)), stub(target, short(&e.to)));
    let first = (source.x, source.y);
    let last = (target.x, target.y);
    let mut candidates: Vec<Vec<Point>> = Vec::new();
    if let Some(path) = preferred {
        candidates.push(path);
    }
    candidates.push(vec![first, s, t, last]);
    candidates.push(vec![first, s, (s.0, t.1), t, last]);
    candidates.push(vec![first, s, (t.0, s.1), t, last]);
    let mut xs = vec![
        (s.0 + t.0) / 2.0,
        s.0 - 30.0,
        s.0 + 30.0,
        t.0 - 30.0,
        t.0 + 30.0,
    ];
    let mut ys = vec![
        (s.1 + t.1) / 2.0,
        s.1 - 30.0,
        s.1 + 30.0,
        t.1 - 30.0,
        t.1 + 30.0,
    ];
    for (i, node) in doc.nodes.iter().enumerate() {
        if node.id == e.from || node.id == e.to {
            continue;
        }
        let b = geom.nodes[i];
        xs.extend([b.x - b.w / 2.0 - 18.0, b.x + b.w / 2.0 + 18.0]);
        ys.extend([b.y - b.h / 2.0 - 18.0, b.y + b.h / 2.0 + 18.0]);
    }
    for pool in &geom.pools {
        ys.push(pool.y + pool.h + 17.0);
        ys.push(pool.y - 17.0);
        for band in &pool.bands {
            ys.push(band.y + band.h - 17.0);
        }
    }
    for x in xs {
        candidates.push(vec![first, s, (x, s.1), (x, t.1), t, last]);
        for y in [
            s.1 - 18.0,
            s.1 + 9.0,
            s.1 + 12.0,
            s.1 + 18.0,
            s.1 - 30.0,
            s.1 + 30.0,
        ] {
            candidates.push(vec![first, s, (s.0, y), (x, y), (x, t.1), t, last]);
        }
    }
    for &y in &ys {
        candidates.push(vec![first, s, (s.0, y), (t.0, y), t, last]);
        for x in [s.0 - 18.0, s.0 + 18.0, s.0 - 30.0, s.0 + 30.0] {
            candidates.push(vec![first, s, (x, s.1), (x, y), (t.0, y), t, last]);
        }
    }
    let best = candidates
        .into_iter()
        .enumerate()
        .filter_map(|(i, points)| {
            let points = compact(points);
            score(doc, geom, e, &points, previous).map(|cost| (cost + i as f64 * 0.001, points))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, p)| p)
        .ok_or_else(|| {
            format!(
                "no obstacle-free orthogonal BPMN route for {} -> {}",
                e.from, e.to
            )
        })?;
    let label = e
        .label
        .as_deref()
        .map(|name| label(doc, geom, &best, name))
        .unwrap_or((0.0, 0.0));
    Ok(Plan {
        points: best,
        label,
    })
}
pub(super) fn bridges(points: &[Point], previous: &[Prior]) -> Vec<(usize, Point)> {
    let mut result = Vec::new();
    for (segment, leg) in points.windows(2).enumerate() {
        for prior in previous {
            for old in prior.points.windows(2) {
                if let Some(p) = crossing(leg[0], leg[1], old[0], old[1])
                    && !result.iter().any(|(s, q): &(usize, Point)| {
                        *s == segment && (q.0 - p.0).abs() + (q.1 - p.1).abs() < 15.0
                    })
                {
                    result.push((segment, p));
                }
            }
        }
    }
    result
}
