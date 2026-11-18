//! Reserve a straight terminal run beyond the marker and any rounded bend.
use super::{Point, geometry::marker_radius};
use crate::{
    geometry::BoxNode,
    syntax::{EndSymbol, RelationDecl},
};

fn straight_run(points: &[Point], source: bool) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    let point = |step: usize| {
        points[if source {
            step
        } else {
            points.len() - 1 - step
        }]
    };
    let tip = point(0);
    let mut direction = None;
    let mut run = 0.0_f64;
    for step in 1..points.len() {
        let p = point(step);
        let (x, y) = (p.x - tip.x, p.y - tip.y);
        let length = x.hypot(y);
        if length < 1e-6 {
            continue;
        }
        let (ux, uy) = *direction.get_or_insert((x / length, y / length));
        let along = x * ux + y * uy;
        if (x * uy - y * ux).abs() > 1e-5 || along < run - 1e-5 {
            break;
        }
        run = along;
    }
    run
}

pub(super) fn minimum(symbol: EndSymbol, width: f64, modern: bool, port: bool) -> f64 {
    let marker = marker_radius(symbol, width);
    let rounding = if modern { 9.0 } else { 0.0 };
    let port_inset = if port && symbol == EndSymbol::Arrow {
        6.0
    } else {
        0.0
    };
    (marker + rounding + 4.0 + port_inset).max(20.0)
}

pub(super) fn clear(
    points: &[Point],
    relation: &RelationDecl,
    width: f64,
    modern: bool,
    a: &BoxNode,
    b: &BoxNode,
) -> bool {
    if points.len() < 2 {
        return false;
    }
    [
        (true, relation.appearance.start, a, points[0]),
        (false, relation.appearance.end, b, *points.last().unwrap()),
    ]
    .into_iter()
    .all(|(source, symbol, node, tip)| {
        let port = node
            .ports
            .iter()
            .any(|p| (p.x - tip.x).abs() < 1e-6 && (p.y - tip.y).abs() < 1e-6);
        straight_run(points, source) + 1e-5 >= minimum(symbol, width, modern, port)
    })
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
