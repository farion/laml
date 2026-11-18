//! Obstacle-aware grid search after the fast, aesthetic candidates are exhausted.
//! Paths have no fixed bend limit. Crossing count is a cost, never a prohibition.
use super::{Point, Route, geometry, obstacles::Obstacles, orthogonal, side_rules};
use crate::{
    geometry::BoxNode,
    syntax::{Diagram, DiagramType, EndPosition, PortSide},
};
use std::{cmp::Reverse, collections::BinaryHeap};

#[derive(Clone, Copy)]
struct Dock {
    port: Point,
    stub: Point,
}

#[allow(clippy::too_many_arguments)] // A dock depends on the same endpoints and obstacles as its route.
fn docks(
    d: &Diagram,
    index: usize,
    boxes: &[BoxNode],
    node: usize,
    other: usize,
    source: bool,
    previous: &[Route],
    check: &Obstacles,
    terminal_gap: f64,
) -> Vec<Dock> {
    let rel = &d.relations[index];
    let id = if source { &rel.from } else { &rel.to };
    let b = &boxes[node];
    let member = (id != &b.id).then_some(id.as_str());
    let required = side_rules::requested(
        d,
        index,
        if source {
            EndPosition::Source
        } else {
            EndPosition::Target
        },
    );
    let named_side = member
        .and_then(|id| b.ports.iter().find(|p| p.id == id))
        .map(|p| p.side);
    let mut result = Vec::new();
    let width = rel
        .display_label()
        .and_then(|s| crate::text::measure(&s, 14.0).ok())
        .unwrap_or(0.0);
    for side in [
        PortSide::Right,
        PortSide::Left,
        PortSide::Bottom,
        PortSide::Top,
    ] {
        if required.is_some_and(|s| s != side) || named_side.is_some_and(|s| s != side) {
            continue;
        }
        let ports = if member.is_some() && named_side.is_none() {
            if !matches!(side, PortSide::Left | PortSide::Right) {
                continue;
            }
            geometry::member_port(
                b,
                id,
                side_rules::member_towards(b, Some(side), geometry::center(&boxes[other])),
            )
            .into_iter()
            .collect()
        } else {
            orthogonal::endpoints(
                b,
                member,
                Some(side),
                side,
                true,
                geometry::center(&boxes[other]),
            )
        };
        for port in ports {
            for distance in [terminal_gap, 32.0, 64.0, 128.0, width + terminal_gap + 32.0] {
                let distance = distance * super::container_ends::polarity(b, &boxes[other]);
                let mut stub = port;
                match side {
                    PortSide::Left => stub.x -= distance,
                    PortSide::Right => stub.x += distance,
                    PortSide::Top => stub.y -= distance,
                    PortSide::Bottom => stub.y += distance,
                }
                if check.clear(&[port, stub], previous) {
                    result.push(Dock { port, stub });
                }
            }
        }
    }
    result
}
fn sorted(values: &mut Vec<f64>) {
    values.sort_by(f64::total_cmp);
    values.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
}
fn simplify(points: Vec<Point>) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::new();
    for p in points {
        if out
            .last()
            .is_some_and(|q| (q.x - p.x).abs() + (q.y - p.y).abs() < 1e-6)
        {
            continue;
        }
        while out.len() >= 2 {
            let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
            if ((a.x - b.x).abs() < 1e-6 && (b.x - p.x).abs() < 1e-6
                || (a.y - b.y).abs() < 1e-6 && (b.y - p.y).abs() < 1e-6)
                && (b.x - a.x) * (p.x - b.x) + (b.y - a.y) * (p.y - b.y) >= 0.0
            {
                out.pop();
            } else {
                break;
            }
        }
        out.push(p);
    }
    out
}

fn simple(points: &[Point]) -> bool {
    if points.windows(3).any(|p| {
        (p[1].x - p[0].x) * (p[2].x - p[1].x) + (p[1].y - p[0].y) * (p[2].y - p[1].y) < -1e-6
    }) {
        return false;
    }
    for (i, a) in points.windows(2).enumerate() {
        if points
            .windows(2)
            .skip(i + 2)
            .any(|b| geometry::crosses(a[0], a[1], b[0], b[1]))
        {
            return false;
        }
    }
    true
}

pub(super) fn candidates(
    d: &Diagram,
    index: usize,
    boxes: &[BoxNode],
    previous: &[Route],
    terminal_gap: f64,
) -> Vec<Vec<Point>> {
    if !matches!(
        d.settings.diagram_type,
        DiagramType::Generic | DiagramType::Package
    ) {
        return Vec::new();
    }
    let rel = &d.relations[index];
    let (Some(from), Some(to)) = (
        boxes.iter().position(|b| b.has_endpoint(&rel.from)),
        boxes.iter().position(|b| b.has_endpoint(&rel.to)),
    ) else {
        return Vec::new();
    };
    if from == to {
        return Vec::new();
    }
    let check = Obstacles::new(boxes, from, to);
    let starts = docks(
        d,
        index,
        boxes,
        from,
        to,
        true,
        previous,
        &check,
        terminal_gap,
    );
    let ends = docks(
        d,
        index,
        boxes,
        to,
        from,
        false,
        previous,
        &check,
        terminal_gap,
    );
    if starts.is_empty() || ends.is_empty() {
        return Vec::new();
    }
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for b in boxes {
        for distance in [16.0, 32.0, 64.0] {
            xs.extend([b.x - distance, b.x + b.width + distance]);
            ys.extend([b.y - distance, b.y + b.height + distance]);
        }
    }
    for r in previous {
        for p in &r.points {
            xs.extend([p.x - 16.0, p.x + 16.0]);
            ys.extend([p.y - 16.0, p.y + 16.0]);
        }
    }
    for dock in starts.iter().chain(&ends) {
        xs.push(dock.stub.x);
        ys.push(dock.stub.y);
    }
    sorted(&mut xs);
    sorted(&mut ys);
    let (nx, ny) = (xs.len(), ys.len());
    // This is a resource guard, not a small fixed number of route candidates.
    // A million grid positions already requires millions of search states.
    if nx.saturating_mul(ny) > 1_000_000 {
        return Vec::new();
    }
    let at = |p: Point| {
        ys.binary_search_by(|y| y.total_cmp(&p.y)).unwrap() * nx
            + xs.binary_search_by(|x| x.total_cmp(&p.x)).unwrap()
    };
    let point = |v: usize| Point {
        x: xs[v % nx],
        y: ys[v / nx],
    };
    let mut goals = vec![Vec::new(); nx * ny];
    for (i, dock) in ends.iter().enumerate() {
        goals[at(dock.stub)].push(i);
    }
    let mut distance = vec![u64::MAX; nx * ny * 2];
    let mut parent = vec![usize::MAX; distance.len()];
    let mut root = vec![0; distance.len()];
    let mut heap = BinaryHeap::new();
    for (i, dock) in starts.iter().enumerate() {
        let axis = usize::from((dock.port.y - dock.stub.y).abs() > 1e-6);
        let state = at(dock.stub) * 2 + axis;
        let cost = ((dock.port.x - dock.stub.x).hypot(dock.port.y - dock.stub.y) * 100.0) as u64;
        if cost < distance[state] {
            distance[state] = cost;
            root[state] = i;
            heap.push(Reverse((cost, state)));
        }
    }
    let mut result = Vec::new();
    while let Some(Reverse((cost, state))) = heap.pop() {
        if cost != distance[state] {
            continue;
        }
        let v = state / 2;
        let p = point(v);
        for &end_index in &goals[v] {
            let mut path = vec![ends[end_index].port];
            let mut cursor = state;
            loop {
                path.push(point(cursor / 2));
                if parent[cursor] == usize::MAX {
                    break;
                }
                cursor = parent[cursor];
            }
            path.push(starts[root[state]].port);
            path.reverse();
            let path = simplify(path);
            if simple(&path)
                && check.clear(&path, previous)
                && orthogonal::Contours::new(&boxes[from], &boxes[to]).normal(&path, false)
            {
                result.push(path);
                // Different goal ports/corridors remain available for label placement.
            }
        }
        let (x, y) = (v % nx, v / nx);
        for next in [
            x.checked_sub(1).map(|x| y * nx + x),
            (x + 1 < nx).then_some(v + 1),
            y.checked_sub(1).map(|y| y * nx + x),
            (y + 1 < ny).then_some(v + nx),
        ]
        .into_iter()
        .flatten()
        {
            let q = point(next);
            let axis = usize::from(next.abs_diff(v) == nx);
            let target = next * 2 + axis;
            let mut added = ((p.x - q.x).hypot(p.y - q.y) * 100.0) as u64;
            if axis != state % 2 {
                added += 2400;
            }
            added += previous
                .iter()
                .flat_map(|r| r.points.windows(2))
                .filter(|r| geometry::crosses(p, q, r[0], r[1]))
                .count() as u64
                * 8000;
            if cost.saturating_add(added) >= distance[target]
                || !check.clear_segments(&[p, q], previous)
            {
                continue;
            }
            distance[target] = cost + added;
            parent[target] = state;
            root[target] = root[state];
            heap.push(Reverse((cost + added, target)));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grid_search_can_escape_a_cage_with_more_than_four_bends() {
        let d=crate::parser::parse("semantics {\nA\nB\nTop\nBottom\nWall\n@r A -> B\n}\nlayout {\nrelation[r].source : side=right\nrelation[r].target : side=left\n}\n").unwrap();
        let sheet = crate::styles::StyleSheet::default();
        let mut boxes = crate::geometry::place(&d, &sheet).unwrap();
        for (id, x, y, w, h) in [
            ("A", 0., 0., 100., 60.),
            ("B", 600., 0., 100., 60.),
            ("Top", 120., -80., 280., 40.),
            ("Bottom", 120., 100., 280., 40.),
            ("Wall", 360., -40., 40., 100.),
        ] {
            let b = boxes.iter_mut().find(|b| b.id == id).unwrap();
            b.x = x;
            b.y = y;
            b.width = w;
            b.height = h;
        }
        let paths = candidates(&d, 0, &boxes, &[], 32.0);
        assert!(
            !paths.is_empty(),
            "a cage with a lower opening is not an impossible connection"
        );
        assert!(
            paths.iter().any(|p| p.len() > 5),
            "must allow longer polylines, not just L and Z candidates"
        );
        for path in paths {
            assert!(Obstacles::new(&boxes, 0, 1).clear(&path, &[]));
            assert_eq!(path[0].x, 100.);
            assert_eq!(path.last().unwrap().x, 600.);
        }
    }
}
