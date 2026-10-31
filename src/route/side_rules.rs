//! Mandatory relation-end sides, including bounded exterior loops for all side pairs.
use super::Point;
use crate::geometry::BoxNode;
use crate::syntax::{Diagram, EndPosition, PortSide};

pub(super) fn member_towards(node: &BoxNode, side: Option<PortSide>, otherwise: Point) -> Point {
    match side {
        Some(PortSide::Left) => Point {
            x: node.x - 1000.0,
            y: otherwise.y,
        },
        Some(PortSide::Right) => Point {
            x: node.x + node.width + 1000.0,
            y: otherwise.y,
        },
        _ => otherwise,
    }
}

pub(super) fn requested(diagram: &Diagram, index: usize, end: EndPosition) -> Option<PortSide> {
    diagram
        .relations
        .get(index)
        .and_then(|relation| relation.name.as_deref())
        .and_then(|name| {
            diagram
                .relation_end_layout
                .iter()
                .find(|rule| rule.name == name && rule.end == end)
                .and_then(|rule| rule.side)
        })
}
fn face(node: &BoxNode, point: Point) -> Option<PortSide> {
    if (point.x - node.x).abs() < 1e-6 {
        Some(PortSide::Left)
    } else if (point.x - node.x - node.width).abs() < 1e-6 {
        Some(PortSide::Right)
    } else if (point.y - node.y).abs() < 1e-6 {
        Some(PortSide::Top)
    } else if (point.y - node.y - node.height).abs() < 1e-6 {
        Some(PortSide::Bottom)
    } else {
        None
    }
}
fn anchor(node: &BoxNode, side: PortSide, offset: f64) -> Point {
    let (cx, cy) = (node.x + node.width / 2.0, node.y + node.height / 2.0);
    match side {
        PortSide::Left => Point {
            x: node.x,
            y: cy + offset,
        },
        PortSide::Right => Point {
            x: node.x + node.width,
            y: cy + offset,
        },
        PortSide::Top => Point {
            x: cx + offset,
            y: node.y,
        },
        PortSide::Bottom => Point {
            x: cx + offset,
            y: node.y + node.height,
        },
    }
}
fn exterior(node: &BoxNode, p: Point, side: PortSide, d: f64) -> Point {
    match side {
        PortSide::Left => Point {
            x: node.x - d,
            y: p.y,
        },
        PortSide::Right => Point {
            x: node.x + node.width + d,
            y: p.y,
        },
        PortSide::Top => Point {
            x: p.x,
            y: node.y - d,
        },
        PortSide::Bottom => Point {
            x: p.x,
            y: node.y + node.height + d,
        },
    }
}
fn perimeter(node: &BoxNode, p: Point, side: PortSide, d: f64) -> f64 {
    let (w, h) = (node.width + 2.0 * d, node.height + 2.0 * d);
    match side {
        PortSide::Top => p.x - (node.x - d),
        PortSide::Right => w + p.y - (node.y - d),
        PortSide::Bottom => w + h + (node.x + node.width + d - p.x),
        PortSide::Left => 2.0 * w + h + (node.y + node.height + d - p.y),
    }
}
fn corner(node: &BoxNode, u: f64, d: f64) -> Point {
    let (w, h) = (node.width + 2.0 * d, node.height + 2.0 * d);
    if u <= w {
        Point {
            x: node.x - d + u,
            y: node.y - d,
        }
    } else if u <= w + h {
        Point {
            x: node.x + node.width + d,
            y: node.y - d + (u - w),
        }
    } else if u <= 2.0 * w + h {
        Point {
            x: node.x + node.width + d - (u - w - h),
            y: node.y + node.height + d,
        }
    } else {
        Point {
            x: node.x - d,
            y: node.y + node.height + d - (u - 2.0 * w - h),
        }
    }
}
fn arc(
    node: &BoxNode,
    start: Point,
    end: Point,
    source: PortSide,
    target: PortSide,
    d: f64,
) -> Vec<Point> {
    let (w, h) = (node.width + 2.0 * d, node.height + 2.0 * d);
    let length = 2.0 * (w + h);
    let u = perimeter(node, start, source, d);
    let mut v = perimeter(node, end, target, d);
    if v < u {
        v += length;
    }
    let mut result = vec![start];
    for value in [
        w,
        w + h,
        2.0 * w + h,
        length,
        length + w,
        length + w + h,
        length + 2.0 * w + h,
    ] {
        if value > u + 1e-6 && value < v - 1e-6 {
            result.push(corner(node, value % length, d));
        }
    }
    result.push(end);
    result
}
fn routes(node: &BoxNode, source: PortSide, target: PortSide, d: f64) -> [Vec<Point>; 2] {
    let spread = 10.0_f64.min(node.width / 4.0).min(node.height / 4.0);
    let (start_offset, end_offset) = if source == target {
        (-spread, spread)
    } else {
        (0.0, 0.0)
    };
    let a = anchor(node, source, start_offset);
    let b = anchor(node, target, end_offset);
    let x = exterior(node, a, source, d);
    let y = exterior(node, b, target, d);
    let cw = arc(node, x, y, source, target, d);
    let mut ccw = arc(node, y, x, target, source, d);
    ccw.reverse();
    [cw, ccw].map(|outer| {
        let mut points = Vec::with_capacity(outer.len() + 2);
        points.push(a);
        points.extend(outer);
        points.push(b);
        points
    })
}
pub(super) fn loop_options(
    diagram: &Diagram,
    index: usize,
    node: &BoxNode,
    mut base: Vec<Vec<Point>>,
) -> Vec<Vec<Point>> {
    let (first, last) = (
        requested(diagram, index, EndPosition::Source),
        requested(diagram, index, EndPosition::Target),
    );
    if first.is_none() && last.is_none() {
        return base;
    }
    let sides = [
        PortSide::Left,
        PortSide::Right,
        PortSide::Top,
        PortSide::Bottom,
    ];
    base.retain(|points| {
        first.is_none_or(|side| face(node, points[0]) == Some(side))
            && last.is_none_or(|side| face(node, *points.last().unwrap()) == Some(side))
    });
    for source in sides
        .into_iter()
        .filter(|side| first.is_none_or(|f| f == *side))
    {
        for target in sides
            .into_iter()
            .filter(|side| last.is_none_or(|f| f == *side))
        {
            for distance in [32.0, 48.0, 64.0, 96.0] {
                base.extend(routes(node, source, target, distance));
            }
        }
    }
    base
}
