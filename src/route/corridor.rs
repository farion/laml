//! Route through shared obstacle corridors before expanding coordinate variants.
//! This search is source-derived, call-local and uses the normal router's final
//! marker, label, crossing and contour checks. Exhaustive routing remains a fallback.
use super::{Point, Route, pass_cache::PassCache};
use crate::{
    geometry::BoxNode,
    styles::StyleSheet,
    syntax::{Diagram, DiagramType},
};

mod docks;
mod search;
#[cfg(test)]
mod tests;

pub(super) fn supported(d: &Diagram) -> bool {
    !d.settings.allow_diagonal
        && matches!(
            d.settings.diagram_type,
            DiagramType::Generic | DiagramType::Package
        )
        && !d
            .nodes
            .iter()
            .any(|n| n.classes.iter().any(|c| c.starts_with("c4")))
}

pub(super) fn route(d: &Diagram, boxes: &[BoxNode], sheet: &StyleSheet) -> Option<Vec<Route>> {
    if !supported(d) {
        return None;
    }
    let visible = crate::geometry::visible_diagram(d, sheet).ok()?;
    let d = &visible;
    let original: Vec<_> = (0..d.relations.len()).collect();
    let mut constrained = original.clone();
    constrained.sort_by_key(|&i| {
        let r = &d.relations[i];
        let ends = [
            crate::syntax::EndPosition::Source,
            crate::syntax::EndPosition::Target,
        ]
        .into_iter()
        .filter(|&end| super::side_rules::requested(d, i, end).is_some())
        .count();
        (r.from != r.to, std::cmp::Reverse(ends), i)
    });
    let mut orders = vec![
        constrained,
        original.iter().copied().rev().collect(),
        original,
    ];
    orders.dedup();
    for crossing_free in [true, false] {
        let mut cache = PassCache::default();
        cache.corridor = true;
        cache.crossing_free = crossing_free;
        for order in &orders {
            if let Ok(routes) = cache.run(d, boxes, sheet, Some(usize::MAX), order, false) {
                return Some(routes);
            }
        }
    }
    None
}

pub(super) fn direct(
    d: &Diagram,
    index: usize,
    boxes: &[BoxNode],
    previous: &[Route],
    terminal_gap: f64,
) -> Vec<Vec<Point>> {
    let r = &d.relations[index];
    let from = boxes.iter().position(|b| b.has_endpoint(&r.from)).unwrap();
    let to = boxes.iter().position(|b| b.has_endpoint(&r.to)).unwrap();
    let check = super::obstacles::Obstacles::new(boxes, from, to);
    let starts = docks::docks(
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
    let ends = docks::docks(
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
    let contours = super::orthogonal::Contours::new(&boxes[from], &boxes[to]);
    // Direct L/Z shapes and the four sides of the endpoint envelope. These
    // few logical shapes also leave room for labels; arbitrary obstacle-track
    // combinations belong to the shared graph, not this endpoint-pair loop.
    let (a, b) = (&boxes[from], &boxes[to]);
    let xs = [a.x.min(b.x) - 12., (a.x + a.width).max(b.x + b.width) + 12.];
    let ys = [
        a.y.min(b.y) - 12.,
        (a.y + a.height).max(b.y + b.height) + 12.,
    ];
    let mut options = Vec::new();
    for start in starts {
        for end in &ends {
            let (u, v) = (start.stub, end.stub);
            for x in xs {
                options.push(vec![
                    start.port,
                    u,
                    Point { x, y: u.y },
                    Point { x, y: v.y },
                    v,
                    end.port,
                ]);
            }
            for y in ys {
                options.push(vec![
                    start.port,
                    u,
                    Point { x: u.x, y },
                    Point { x: v.x, y },
                    v,
                    end.port,
                ]);
            }
            let (a, b) = (start.port, end.port);
            options.push(vec![a, Point { x: b.x, y: a.y }, b]);
            options.push(vec![a, Point { x: a.x, y: b.y }, b]);
            let (x, y) = ((a.x + b.x) / 2., (a.y + b.y) / 2.);
            options.push(vec![a, Point { x, y: a.y }, Point { x, y: b.y }, b]);
            options.push(vec![a, Point { x: a.x, y }, Point { x: b.x, y }, b]);
        }
    }
    let mut unique = std::collections::BTreeSet::new();
    options
        .into_iter()
        .map(search::simplify)
        .filter(|p| search::simple(p) && contours.normal(p, false))
        .filter(|p| {
            unique.insert(
                p.iter()
                    .map(|p| (p.x.to_bits(), p.y.to_bits()))
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

pub(super) fn paths<'a>(
    d: &'a Diagram,
    index: usize,
    boxes: &'a [BoxNode],
    previous: &'a [Route],
    terminal_gap: f64,
    crossing_free: bool,
) -> impl Iterator<Item = Vec<Point>> + 'a {
    std::iter::once_with(move || {
        search::Search::new(d, index, boxes, previous, terminal_gap, crossing_free)
    })
    .flatten()
}

pub(super) fn crosses(points: &[Point], previous: &[Route]) -> bool {
    points.windows(2).any(|p| {
        previous.iter().any(|r| {
            r.points
                .windows(2)
                .any(|q| super::geometry::crosses(p[0], p[1], q[0], q[1]))
        })
    })
}
