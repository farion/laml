//! Lazy multi-source A* on shared obstacle-edge tracks. Each graph state is
//! explored once per improving cost; endpoint pairs never multiply path lists.
use super::{
    super::{Point, Route, geometry, obstacles::Obstacles},
    docks::{Dock, docks},
};
use crate::{geometry::BoxNode, syntax::Diagram};
use std::{
    cmp::Reverse,
    collections::{BTreeSet, BinaryHeap},
};

type Cost = (usize, u64);
const INFINITY: Cost = (usize::MAX, u64::MAX);

pub(super) fn simplify(points: Vec<Point>) -> Vec<Point> {
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

pub(super) struct Search<'a> {
    xs: Vec<f64>,
    ys: Vec<f64>,
    previous: &'a [Route],
    check: Obstacles,
    starts: Vec<Dock>,
    ends: Vec<Dock>,
    goals: Vec<Vec<usize>>,
    distance: Vec<Cost>,
    parent: Vec<usize>,
    root: Vec<usize>,
    heuristic: Vec<u64>,
    heap: BinaryHeap<Reverse<(usize, u64, usize, u64)>>,
    // Each undirected grid edge is collision-tested only once, across both axes.
    edges: Vec<Option<Option<usize>>>,
    crossing_free: bool,
    pending: Vec<Vec<Point>>,
    emitted: BTreeSet<Vec<(u64, u64)>>,
}
impl<'a> Search<'a> {
    pub(super) fn new(
        d: &Diagram,
        index: usize,
        boxes: &[BoxNode],
        previous: &'a [Route],
        gap: f64,
        crossing_free: bool,
    ) -> Self {
        let r = &d.relations[index];
        let from = boxes.iter().position(|b| b.has_endpoint(&r.from)).unwrap();
        let to = boxes.iter().position(|b| b.has_endpoint(&r.to)).unwrap();
        let check = Obstacles::new(boxes, from, to);
        let starts = docks(d, index, boxes, from, to, true, previous, &check, gap);
        let ends = docks(d, index, boxes, to, from, false, previous, &check, gap);
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        // A single canonical track on each side of an obstacle. Smaller gaps
        // get their own midpoint rather than multiple arbitrary pixel offsets.
        let clearance = super::super::clearance::GAP + 4.0;
        for b in boxes {
            xs.extend([b.x - clearance, b.x + b.width + clearance]);
            ys.extend([b.y - clearance, b.y + b.height + clearance]);
        }
        for r in previous {
            for p in &r.points {
                xs.extend([p.x - clearance, p.x + clearance]);
                ys.extend([p.y - clearance, p.y + clearance]);
            }
        }
        for dock in starts.iter().chain(&ends) {
            xs.push(dock.stub.x);
            ys.push(dock.stub.y);
        }
        let sorted = |v: &mut Vec<f64>| {
            v.sort_by(f64::total_cmp);
            v.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        };
        sorted(&mut xs);
        sorted(&mut ys);
        let n = xs.len().saturating_mul(ys.len());
        let mut search = Self {
            xs,
            ys,
            previous,
            check,
            starts,
            ends,
            goals: Vec::new(),
            distance: Vec::new(),
            parent: Vec::new(),
            root: Vec::new(),
            heuristic: Vec::new(),
            heap: BinaryHeap::new(),
            edges: Vec::new(),
            crossing_free,
            pending: Vec::new(),
            emitted: BTreeSet::new(),
        };
        // Same resource protection as the exhaustive grid fallback, not a path cap.
        if n > 1_000_000 || search.starts.is_empty() || search.ends.is_empty() {
            return search;
        }
        search.goals = vec![Vec::new(); n];
        search.distance = vec![INFINITY; n * 4];
        search.parent = vec![usize::MAX; n * 4];
        search.root = vec![0; n * 4];
        search.edges = vec![None; n * 2];
        search.heuristic = (0..n)
            .map(|v| {
                let p = search.point(v);
                search
                    .ends
                    .iter()
                    .map(|e| {
                        units(
                            (p.x - e.stub.x).abs()
                                + (p.y - e.stub.y).abs()
                                + manhattan(e.port, e.stub),
                        )
                    })
                    .min()
                    .unwrap()
            })
            .collect();
        for (i, e) in search.ends.iter().enumerate() {
            let v = search.at(e.stub);
            search.goals[v].push(i);
        }
        for (i, s) in search.starts.iter().enumerate() {
            let heading = direction(s.port, s.stub);
            let v = search.at(s.stub);
            let state = v * 4 + heading;
            let cost = (0, units(manhattan(s.port, s.stub)));
            if cost < search.distance[state] {
                search.distance[state] = cost;
                search.root[state] = i;
                search
                    .heap
                    .push(Reverse((0, cost.1 + search.heuristic[v], state, cost.1)));
            }
        }
        search
    }
    fn at(&self, p: Point) -> usize {
        self.ys.binary_search_by(|y| y.total_cmp(&p.y)).unwrap() * self.xs.len()
            + self.xs.binary_search_by(|x| x.total_cmp(&p.x)).unwrap()
    }
    fn point(&self, v: usize) -> Point {
        Point {
            x: self.xs[v % self.xs.len()],
            y: self.ys[v / self.xs.len()],
        }
    }
    fn path(&self, state: usize, end: usize) -> Vec<Point> {
        let mut path = vec![self.ends[end].port];
        let mut cursor = state;
        loop {
            path.push(self.point(cursor / 4));
            if self.parent[cursor] == usize::MAX {
                break;
            }
            cursor = self.parent[cursor];
        }
        path.push(self.starts[self.root[state]].port);
        path.reverse();
        simplify(path)
    }
    fn edge(&mut self, v: usize, next: usize, axis: usize) -> Option<usize> {
        let key = v.min(next) * 2 + axis;
        if let Some(cached) = self.edges[key] {
            return cached;
        }
        let (p, q) = (self.point(v), self.point(next));
        let result = if self.check.clear_segments(&[p, q], self.previous) {
            let count = self
                .previous
                .iter()
                .flat_map(|r| r.points.windows(2))
                .filter(|r| geometry::crosses(p, q, r[0], r[1]))
                .count();
            (!self.crossing_free || count == 0).then_some(count)
        } else {
            None
        };
        self.edges[key] = Some(result);
        result
    }
}
impl Iterator for Search<'_> {
    type Item = Vec<Point>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(path) = self.pending.pop() {
                let key = path
                    .iter()
                    .map(|p| (p.x.to_bits(), p.y.to_bits()))
                    .collect();
                if simple(&path)
                    && self.emitted.insert(key)
                    && self.check.clear(&path, self.previous)
                {
                    return Some(path);
                }
            }
            let Reverse((crossings, _, state, cost)) = self.heap.pop()?;
            if (crossings, cost) != self.distance[state] {
                continue;
            }
            let v = state / 4;
            let (nx, ny) = (self.xs.len(), self.ys.len());
            let (x, y) = (v % nx, v / nx);
            for &end in &self.goals[v] {
                if direction(self.ends[end].stub, self.ends[end].port) != (state % 4) ^ 1 {
                    self.pending.push(self.path(state, end));
                }
            }
            for (next, heading) in [
                (x.checked_sub(1).map(|x| y * nx + x), 1),
                ((x + 1 < nx).then_some(v + 1), 0),
                (y.checked_sub(1).map(|y| y * nx + x), 3),
                ((y + 1 < ny).then_some(v + nx), 2),
            ] {
                let Some(next) = next else {
                    continue;
                };
                if heading == (state % 4) ^ 1 {
                    continue;
                }
                let axis = heading / 2;
                let target = next * 4 + heading;
                let length = units(manhattan(self.point(v), self.point(next)))
                    + if axis != (state % 4) / 2 { 2400 } else { 0 };
                if (crossings, cost.saturating_add(length)) >= self.distance[target] {
                    continue;
                }
                let Some(added) = self.edge(v, next, axis) else {
                    continue;
                };
                let candidate = (crossings + added, cost.saturating_add(length));
                if candidate < self.distance[target] {
                    self.distance[target] = candidate;
                    self.parent[target] = state;
                    self.root[target] = self.root[state];
                    self.heap.push(Reverse((
                        candidate.0,
                        candidate.1 + self.heuristic[next],
                        target,
                        candidate.1,
                    )));
                }
            }
        }
    }
}
fn manhattan(a: Point, b: Point) -> f64 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
fn units(length: f64) -> u64 {
    (length * 100.0).round() as u64
}

pub(super) fn simple(points: &[Point]) -> bool {
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

fn direction(a: Point, b: Point) -> usize {
    if (a.y - b.y).abs() < 1e-6 {
        usize::from(b.x < a.x)
    } else {
        2 + usize::from(b.y < a.y)
    }
}
