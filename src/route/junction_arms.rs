//! Orthogonal first choices for unpositioned n-ary association arms.
//! A diamond is a junction, not a classifier-to-classifier diagonal.
use super::Point;
use super::geometry::{center, port};
use crate::geometry::BoxNode;
use crate::syntax::NodeKind;

pub(super) fn candidates(a: &BoxNode, b: &BoxNode) -> Vec<Vec<Point>> {
    if (a.kind != NodeKind::Association && b.kind != NodeKind::Association)
        || matches!(a.kind, NodeKind::AssociationClass)
        || matches!(b.kind, NodeKind::AssociationClass)
    {
        return Vec::new();
    }
    let (ca, cb) = (center(a), center(b));
    if (ca.y - cb.y).abs() < 1.0 || (ca.x - cb.x).abs() < 1.0 {
        return Vec::new(); // the ordinary straight option is already axial
    }
    [Point { x: cb.x, y: ca.y }, Point { x: ca.x, y: cb.y }]
        .into_iter()
        .map(|bend| vec![port(a, bend), bend, port(b, bend)])
        .collect()
}
