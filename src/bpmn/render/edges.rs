//! Edge attachment and orthogonal routes.
use super::ports::{Attach, Ports};
use super::{Document, EdgeKind, Geometry};
use crate::bpmn::{Edge, Side};
use std::fmt::Write;

fn stub(a: Attach) -> (f64, f64) {
    match a.side {
        Side::Left => (a.x - 23.0, a.y),
        Side::Right => (a.x + 23.0, a.y),
        Side::Top => (a.x, a.y - 23.0),
        Side::Bottom => (a.x, a.y + 23.0),
    }
}
fn gateway_route(
    from: Option<Attach>,
    to: Option<Attach>,
    source: Attach,
    target: Attach,
) -> Vec<super::orthogonal::Point> {
    // If both sides permit it, a branch needs only one bend. The shared
    // planner checks clearance and supplies a detour when necessary.
    let corner = if from.is_some() && to.is_none() {
        match (source.side, target.side) {
            (Side::Top, Side::Left | Side::Right) if target.y < source.y - 2.0 => {
                Some((source.x, target.y))
            }
            (Side::Bottom, Side::Left | Side::Right) if target.y > source.y + 2.0 => {
                Some((source.x, target.y))
            }
            (Side::Left, Side::Top | Side::Bottom) if target.x < source.x - 2.0 => {
                Some((target.x, source.y))
            }
            (Side::Right, Side::Top | Side::Bottom) if target.x > source.x + 2.0 => {
                Some((target.x, source.y))
            }
            _ => None,
        }
    } else if from.is_none() && to.is_some() {
        match (source.side, target.side) {
            (Side::Left | Side::Right, Side::Top) if source.y < target.y - 2.0 => {
                Some((target.x, source.y))
            }
            (Side::Left | Side::Right, Side::Bottom) if source.y > target.y + 2.0 => {
                Some((target.x, source.y))
            }
            (Side::Top | Side::Bottom, Side::Left) if source.x < target.x - 2.0 => {
                Some((source.x, target.y))
            }
            (Side::Top | Side::Bottom, Side::Right) if source.x > target.x + 2.0 => {
                Some((source.x, target.y))
            }
            _ => None,
        }
    } else {
        None
    };
    if let Some(corner) = corner {
        return vec![(source.x, source.y), corner, (target.x, target.y)];
    }
    let (s, t) = (stub(source), stub(target));
    let vertical = |side| matches!(side, Side::Top | Side::Bottom);
    let mut points = vec![(source.x, source.y), s];
    match (vertical(source.side), vertical(target.side)) {
        (true, false) => points.push((t.0, s.1)),
        (false, true) => points.push((s.0, t.1)),
        (true, true) => {
            let mid = (s.1 + t.1) / 2.0;
            points.extend([(s.0, mid), (t.0, mid)]);
        }
        (false, false) => {
            let mid = (s.0 + t.0) / 2.0;
            points.extend([(mid, s.1), (mid, t.1)]);
        }
    }
    points.extend([t, (target.x, target.y)]);
    points
}
fn path_with_bridges(
    points: &[super::orthogonal::Point],
    bridges: &[(usize, super::orthogonal::Point)],
    radius: f64,
) -> (String, Vec<String>) {
    let corners: Vec<_> = (1..points.len().saturating_sub(1))
        .map(|i| rounded_corner(points, bridges, i, radius))
        .collect();
    let mut d = format!("M {:.1} {:.1}", points[0].0, points[0].1);
    let mut masks = Vec::new();
    for (segment, leg) in points.windows(2).enumerate() {
        let (a, b) = (leg[0], leg[1]);
        let length = (b.0 - a.0).abs() + (b.1 - a.1).abs();
        if length < 0.1 {
            continue;
        }
        let (dx, dy) = ((b.0 - a.0) / length, (b.1 - a.1) / length);
        let mut crossings: Vec<_> = bridges
            .iter()
            .filter(|(i, _)| *i == segment)
            .map(|(_, p)| *p)
            .collect();
        crossings.sort_by(|p, q| {
            ((p.0 - a.0) * dx + (p.1 - a.1) * dy).total_cmp(&((q.0 - a.0) * dx + (q.1 - a.1) * dy))
        });
        for p in crossings {
            let pre = (p.0 - dx * 6.0, p.1 - dy * 6.0);
            let post = (p.0 + dx * 6.0, p.1 + dy * 6.0);
            // Bridge humps are stable in image coordinates, not traversal order:
            // horizontal crossings arc upward; vertical crossings arc left.
            let control = if dx.abs() > dy.abs() {
                (p.0, p.1 - 15.0)
            } else {
                (p.0 - 15.0, p.1)
            };
            let arc = format!(
                "M {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1}",
                pre.0, pre.1, control.0, control.1, post.0, post.1
            );
            masks.push(arc);
            write!(
                d,
                " L {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1}",
                pre.0, pre.1, control.0, control.1, post.0, post.1
            )
            .unwrap();
        }
        if let Some((before, after)) = corners.get(segment).copied().flatten() {
            write!(
                d,
                " L {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1}",
                before.0, before.1, b.0, b.1, after.0, after.1
            )
            .unwrap();
        } else {
            write!(d, " L {:.1} {:.1}", b.0, b.1).unwrap();
        }
    }
    (d, masks)
}
fn rounded_corner(
    points: &[super::orthogonal::Point],
    bridges: &[(usize, super::orthogonal::Point)],
    index: usize,
    radius: f64,
) -> Option<(super::orthogonal::Point, super::orthogonal::Point)> {
    if radius <= 0.0 {
        return None;
    }
    let (a, b, c) = (points[index - 1], points[index], points[index + 1]);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (ex, ey) = (c.0 - b.0, c.1 - b.1);
    let before = dx.hypot(dy);
    let after = ex.hypot(ey);
    if before < 1e-6 || after < 1e-6 || (dx * ey - dy * ex).abs() < 1e-5 * before * after {
        return None;
    }
    let last_bridge = bridges
        .iter()
        .filter(|(i, _)| *i == index - 1)
        .map(|(_, p)| (p.0 - a.0).hypot(p.1 - a.1) + 6.0)
        .fold(0.0_f64, f64::max);
    let first_bridge = bridges
        .iter()
        .filter(|(i, _)| *i == index)
        .map(|(_, p)| (p.0 - b.0).hypot(p.1 - b.1) - 6.0)
        .fold(after, f64::min);
    let r = radius
        .min(before / 2.0)
        .min(after / 2.0)
        .min(before - last_bridge)
        .min(first_bridge);
    (r >= 1.0).then_some((
        (b.0 - dx / before * r, b.1 - dy / before * r),
        (b.0 + ex / after * r, b.1 + ey / after * r),
    ))
}
pub(super) fn edge(
    svg: &mut String,
    doc: &Document,
    geom: &Geometry,
    e: &Edge,
    index: usize,
    ports: &Ports,
    previous: &mut Vec<super::orthogonal::Prior>,
) -> Result<(), String> {
    let ai = doc.nodes.iter().position(|n| n.id == e.from).unwrap();
    let bi = doc.nodes.iter().position(|n| n.id == e.to).unwrap();
    let gateway = |kind| {
        matches!(
            kind,
            crate::bpmn::Kind::Exclusive
                | crate::bpmn::Kind::Parallel
                | crate::bpmn::Kind::Inclusive
                | crate::bpmn::Kind::EventBased
        )
    };
    // Stage one selects free, forward-facing node ports; stage two finds an
    // obstacle-free path for each pair. Compare actual route costs, including
    // bends, instead of locking in the first node side before path planning.
    let mut best: Option<(super::orthogonal::Plan, Attach, Attach)> = None;
    let mut error = None;
    let sources = super::ports::alternatives::for_end(doc, geom, ports, index, true);
    let targets = super::ports::alternatives::for_end(doc, geom, ports, index, false);
    for source in sources {
        for &target in &targets {
            let from = gateway(doc.nodes[ai].kind).then_some(source);
            let to = gateway(doc.nodes[bi].kind).then_some(target);
            let preferred = if e.kind == EdgeKind::Sequence && (from.is_some() || to.is_some()) {
                Some(gateway_route(from, to, source, target))
            } else {
                None
            };
            match super::orthogonal::route(doc, geom, e, source, target, preferred, previous) {
                Ok(plan)
                    if best.as_ref().is_none_or(|(current, _, _)| {
                        plan.crossings < current.crossings
                            || (plan.crossings == current.crossings
                                && plan.cost < current.cost - 0.01)
                    }) =>
                {
                    best = Some((plan, source, target))
                }
                Err(reason) => error = Some(reason),
                _ => (),
            }
        }
    }
    let (plan, source, _target) = best.ok_or_else(|| {
        error.unwrap_or_else(|| {
            format!(
                "no obstacle-free orthogonal BPMN route for {} -> {}",
                e.from, e.to
            )
        })
    })?;
    let bridges = super::orthogonal::bridges(&plan.points, previous);
    let (path, masks) = path_with_bridges(
        &plan.points,
        &bridges,
        if doc.theme.modern() { 9.0 } else { 0.0 },
    );
    for mask in masks {
        writeln!(
            svg,
            "<path d=\"{mask}\" fill=\"none\" stroke=\"#f8fbff\" stroke-width=\"6\"/>"
        )
        .unwrap();
    }
    let (dash, marker) = match e.kind {
        EdgeKind::Sequence => ("", "solid-arrow"),
        EdgeKind::Message => ("stroke-dasharray=\"8 5\"", "open-arrow"),
        EdgeKind::Data => ("stroke-dasharray=\"2 5\"", "open-arrow"),
        EdgeKind::Association => ("stroke-dasharray=\"4 4\"", ""),
    };
    let arrow = if marker.is_empty() {
        String::new()
    } else {
        format!("marker-end=\"url(#{marker})\"")
    };
    writeln!(
        svg,
        "<path d=\"{path}\" fill=\"none\" stroke=\"#263548\" stroke-width=\"1.7\" {dash} {arrow}/>"
    )
    .unwrap();
    if e.kind == EdgeKind::Message {
        writeln!(
            svg,
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"4\" fill=\"white\" stroke=\"#263548\"/>",
            source.x, source.y
        )
        .unwrap();
    }
    previous.push(super::orthogonal::Prior {
        points: plan.points,
        kind: e.kind,
        from: e.from.clone(),
        to: e.to.clone(),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::path_with_bridges;
    #[test]
    fn modern_route_rounds_the_bend_but_plain_keeps_it_sharp() {
        let points = [(0.0, 0.0), (60.0, 0.0), (60.0, 80.0)];
        assert_eq!(
            path_with_bridges(&points, &[], 9.0).0,
            "M 0.0 0.0 L 51.0 0.0 Q 60.0 0.0 60.0 9.0 L 60.0 80.0"
        );
        assert_eq!(
            path_with_bridges(&points, &[], 0.0).0,
            "M 0.0 0.0 L 60.0 0.0 L 60.0 80.0"
        );
    }
    #[test]
    fn bridges_arc_up_or_left_regardless_of_flow_direction() {
        for points in [[(0.0, 50.0), (100.0, 50.0)], [(100.0, 50.0), (0.0, 50.0)]] {
            assert!(
                path_with_bridges(&points, &[(0, (50.0, 50.0))], 0.0)
                    .0
                    .contains("Q 50.0 35.0")
            );
        }
        for points in [[(50.0, 0.0), (50.0, 100.0)], [(50.0, 100.0), (50.0, 0.0)]] {
            assert!(
                path_with_bridges(&points, &[(0, (50.0, 50.0))], 0.0)
                    .0
                    .contains("Q 35.0 50.0")
            );
        }
    }
}
