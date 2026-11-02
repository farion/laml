//! Tight, measured SVG canvas bounds without a phantom origin.
use crate::geometry::BoxNode;
use crate::route::{Point, Route};
fn include_point(point: Point, bounds: &mut (f64, f64, f64, f64)) {
    bounds.0 = bounds.0.min(point.x);
    bounds.1 = bounds.1.min(point.y);
    bounds.2 = bounds.2.max(point.x);
    bounds.3 = bounds.3.max(point.y);
}

pub(super) fn measure(boxes: &[BoxNode], routes: &[Route]) -> (f64, f64, f64, f64) {
    let mut bounds = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for node in boxes {
        include_point(
            Point {
                x: node.x,
                y: node.y,
            },
            &mut bounds,
        );
        include_point(
            Point {
                x: node.x + node.width,
                y: node.y + node.height,
            },
            &mut bounds,
        );
    }
    for route in routes {
        for &point in &route.points {
            include_point(point, &mut bounds);
        }
        for qualifier in &route.qualifiers {
            include_point(
                Point {
                    x: qualifier.x,
                    y: qualifier.y,
                },
                &mut bounds,
            );
            include_point(
                Point {
                    x: qualifier.x + qualifier.width,
                    y: qualifier.y + qualifier.height,
                },
                &mut bounds,
            );
        }
        for bridge in &route.bridges {
            let pair = &route.points[bridge.segment..=bridge.segment + 1];
            let dx = pair[1].x - pair[0].x;
            let dy = pair[1].y - pair[0].y;
            let length = dx.hypot(dy);
            let along = bridge.distance / length;
            let normal = crate::route::bridge_normal(Point {
                x: dx / length,
                y: dy / length,
            });
            let apex = Point {
                x: pair[0].x + dx * along + normal.x * bridge.height * 2.0,
                y: pair[0].y + dy * along + normal.y * bridge.height * 2.0,
            };
            include_point(apex, &mut bounds);
        }
        for label in route.label.iter().chain(route.end_labels.iter()) {
            include_point(
                Point {
                    x: label.x,
                    y: label.y,
                },
                &mut bounds,
            );
            include_point(
                Point {
                    x: label.x + label.width,
                    y: label.y + label.height,
                },
                &mut bounds,
            );
        }
    }
    if !bounds.0.is_finite() {
        bounds = (0.0, 0.0, 0.0, 0.0);
    }
    let (left, top, right, bottom) = (
        bounds.0 - 32.0,
        bounds.1 - 32.0,
        bounds.2 + 32.0,
        bounds.3 + 32.0,
    );
    (left, top, right, bottom)
}
