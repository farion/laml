//! Mandatory visible whitespace for generic/package routing.
//! Endpoints may touch their own contour normally, never run along it.
use super::Point;
use crate::geometry::BoxNode;
use crate::syntax::NodeKind;

pub(super) const GAP: f64 = 8.0;

pub(super) fn outlines(node: &BoxNode) -> Vec<(f64, f64, f64, f64)> {
    if node.kind == NodeKind::Package
        && !node.c4_boundary
        && let Ok((w, h)) = crate::geometry::package_outline::tab(node)
    {
        return vec![
            (node.x, node.y, node.x + w, node.y),
            (node.x + w, node.y, node.x + w, node.y + h),
            (node.x + w, node.y + h, node.x + node.width, node.y + h),
            (
                node.x + node.width,
                node.y + h,
                node.x + node.width,
                node.y + node.height,
            ),
            (
                node.x,
                node.y + node.height,
                node.x + node.width,
                node.y + node.height,
            ),
            (node.x, node.y, node.x, node.y + node.height),
        ];
    }
    vec![
        (node.x, node.y, node.x + node.width, node.y),
        (
            node.x,
            node.y + node.height,
            node.x + node.width,
            node.y + node.height,
        ),
        (node.x, node.y, node.x, node.y + node.height),
        (
            node.x + node.width,
            node.y,
            node.x + node.width,
            node.y + node.height,
        ),
    ]
}

pub(super) fn away(a: Point, b: Point, (x1, y1, x2, y2): (f64, f64, f64, f64)) -> bool {
    let overlap =
        |a: f64, b: f64, c: f64, d: f64| a.min(b).max(c.min(d)) < a.max(b).min(c.max(d)) - 1e-5;
    if (a.y - b.y).abs() < 1e-5 && (y1 - y2).abs() < 1e-5 {
        !((a.y - y1).abs() < GAP - 1e-5 && overlap(a.x, b.x, x1, x2))
    } else if (a.x - b.x).abs() < 1e-5 && (x1 - x2).abs() < 1e-5 {
        !((a.x - x1).abs() < GAP - 1e-5 && overlap(a.y, b.y, y1, y2))
    } else {
        true
    }
}
