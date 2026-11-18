//! A containing endpoint is docked on its inner face, not its exterior face.
use super::Point;
use crate::geometry::BoxNode;

pub(super) fn encloses(container: &BoxNode, other: &BoxNode) -> bool {
    container.is_container
        && other.x > container.x
        && other.y > container.y
        && other.x + other.width < container.x + container.width
        && other.y + other.height < container.y + container.height
}

pub(super) fn polarity(node: &BoxNode, other: &BoxNode) -> f64 {
    if encloses(node, other) { -1.0 } else { 1.0 }
}

/// Reuse the normal-to-contour test, reversing only an enclosing endpoint's face.
/// The actual points and authored attachment side remain unchanged.
pub(super) fn departure(node: &BoxNode, other: &BoxNode, port: Point, next: Point) -> Point {
    let sign = polarity(node, other);
    Point {
        x: port.x + sign * (next.x - port.x),
        y: port.y + sign * (next.y - port.y),
    }
}
