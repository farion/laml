//! Orthogonal first choices for compact n-ary association diamonds.
use super::Point;
use super::geometry::{center, port};
use crate::geometry::BoxNode;
use crate::syntax::NodeKind;

pub(super) fn candidates(a: &BoxNode, b: &BoxNode) -> Vec<Vec<Point>> {
    if a.kind != NodeKind::Association && b.kind != NodeKind::Association {
        return Vec::new();
    }
    let (ca, cb) = (center(a), center(b));
    let mut paths = Vec::new();
    if (ca.y - cb.y).abs() >= 1.0 && (ca.x - cb.x).abs() >= 1.0 {
        paths.extend(
            [Point { x: cb.x, y: ca.y }, Point { x: ca.x, y: cb.y }]
                .into_iter()
                .map(|bend| vec![port(a, bend), bend, port(b, bend)]),
        );
    }
    // Labels can occupy the fourth cardinal side of a compact diamond while
    // three existing arms use its other vertices. Offer true contour docks on
    // its sloping edges instead of overlapping an arm or its visible label.
    let (junction, other, reverse) = if b.kind == NodeKind::Association {
        (b, a, false)
    } else {
        (a, b, true)
    };
    let (c, outside) = (center(junction), center(other));
    let half = super::geometry::compact_radius(junction).unwrap() / 2.0;
    for dx in [-half, half] {
        for dy in [-half, half] {
            let dock = Point {
                x: c.x + dx,
                y: c.y + dy,
            };
            for bend in [
                Point {
                    x: dock.x,
                    y: outside.y,
                },
                Point {
                    x: outside.x,
                    y: dock.y,
                },
            ] {
                let mut path = vec![port(other, bend), bend, dock];
                if reverse {
                    path.reverse();
                }
                paths.push(path);
            }
        }
    }
    paths
}
