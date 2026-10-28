//! Deterministic exterior loops for a node related to itself.

use super::Point;
use crate::geometry::BoxNode;

pub(super) fn candidates(node: &BoxNode, rank: usize) -> Vec<Vec<Point>> {
    let cx = node.x + node.width / 2.0;
    let cy = node.y + node.height / 2.0;
    let (left, right, top, bottom) = (node.x, node.x + node.width, node.y, node.y + node.height);
    let spread = 10.0_f64.min(node.width / 4.0).min(node.height / 4.0);
    let mut options = Vec::new();
    // Try the source-order preferred quadrant first, then other quadrants
    // if another node or relation blocks it. No diagram history is needed.
    let order = [3, 2, 0, 1];
    for priority in 0..4 {
        let side = order[(rank + priority) % 4];
        for distance in [32.0, 48.0, 64.0, 96.0] {
            let points = match side {
                0 => vec![
                    Point {
                        x: right,
                        y: cy - spread,
                    },
                    Point {
                        x: right + distance,
                        y: cy - spread,
                    },
                    Point {
                        x: right + distance,
                        y: top - distance,
                    },
                    Point {
                        x: cx + spread,
                        y: top - distance,
                    },
                    Point {
                        x: cx + spread,
                        y: top,
                    },
                ],
                1 => vec![
                    Point {
                        x: right,
                        y: cy + spread,
                    },
                    Point {
                        x: right + distance,
                        y: cy + spread,
                    },
                    Point {
                        x: right + distance,
                        y: bottom + distance,
                    },
                    Point {
                        x: cx + spread,
                        y: bottom + distance,
                    },
                    Point {
                        x: cx + spread,
                        y: bottom,
                    },
                ],
                2 => vec![
                    Point {
                        x: cx - spread,
                        y: bottom,
                    },
                    Point {
                        x: cx - spread,
                        y: bottom + distance,
                    },
                    Point {
                        x: left - distance,
                        y: bottom + distance,
                    },
                    Point {
                        x: left - distance,
                        y: cy + spread,
                    },
                    Point {
                        x: left,
                        y: cy + spread,
                    },
                ],
                _ => vec![
                    Point {
                        x: left,
                        y: cy - spread,
                    },
                    Point {
                        x: left - distance,
                        y: cy - spread,
                    },
                    Point {
                        x: left - distance,
                        y: top - distance,
                    },
                    Point {
                        x: cx - spread,
                        y: top - distance,
                    },
                    Point {
                        x: cx - spread,
                        y: top,
                    },
                ],
            };
            options.push(points);
        }
    }
    options
}
