//! Notation-aware terminal departure checks against visible node contours.
use super::{Point, preferred::forced_port};
use crate::{
    geometry::{BoxNode, NodeShape},
    syntax::PortSide,
};

// For box-like nodes, both terminal strokes must be normal to a visible
// side; following an edge into an arrowhead is never a valid connector.
pub(super) fn valid(
    node: &BoxNode,
    port: Point,
    outside: Point,
    allow_slant: bool,
    visible_top: Option<(f64, f64)>,
) -> bool {
    if node.shape != NodeShape::Diamond
        && !matches!(
            node.kind,
            crate::syntax::NodeKind::Generic
                | crate::syntax::NodeKind::Package
                | crate::syntax::NodeKind::Class
                | crate::syntax::NodeKind::Object
                | crate::syntax::NodeKind::UseCaseSystem
        )
    {
        return true;
    }
    let (dx, dy) = (outside.x - port.x, outside.y - port.y);
    let epsilon = 0.01;
    let normal = |axial: f64, lateral: f64| {
        if allow_slant {
            let distance = axial.hypot(lateral);
            distance >= 20.0 - epsilon && axial >= distance * 0.25
        } else {
            axial >= 20.0 - epsilon && lateral.abs() < epsilon
        }
    };
    if let Some(boundary) = node
        .ports
        .iter()
        .find(|p| (p.x - port.x).abs() < epsilon && (p.y - port.y).abs() < epsilon)
    {
        // Named ports may sit between a shaped node's extremes. Their authored
        // side remains the direction contract, rather than forcing a vertex.
        return match boundary.side {
            PortSide::Left => normal(-dx, dy),
            PortSide::Right => normal(dx, dy),
            PortSide::Top => normal(-dy, dx),
            PortSide::Bottom => normal(dy, dx),
        };
    }
    if matches!(node.shape, NodeShape::Diamond | NodeShape::Ellipse) {
        // A vertex/extreme is a directional dock, not a point that may be
        // approached tangentially just because the segment stays outside.
        for side in [
            PortSide::Left,
            PortSide::Right,
            PortSide::Top,
            PortSide::Bottom,
        ] {
            let dock = forced_port(node, side);
            if (dock.x - port.x).abs() < epsilon && (dock.y - port.y).abs() < epsilon {
                return match side {
                    PortSide::Left => normal(-dx, dy),
                    PortSide::Right => normal(dx, dy),
                    PortSide::Top => normal(-dy, dx),
                    PortSide::Bottom => normal(dy, dx),
                };
            }
        }
        // Compact UML junctions can have more arms than cardinal vertices.
        // Their sloping edges accept outward axial exits too; authored generic
        // diamond side docks remain cardinal and keep their stricter contract.
        let compact = super::geometry::compact_radius(node);
        if node.shape == NodeShape::Diamond && !allow_slant && compact.is_none() {
            return false;
        }
        // Preserve deliberately diagonal/non-cardinal contour attachments, but
        // reject tangent or inward departures there as well.
        let c = super::geometry::center(node);
        let (rx, ry) = (
            compact.unwrap_or(node.width / 2.0),
            compact.unwrap_or(node.height / 2.0),
        );
        let outward = ((port.x - c.x) / (rx * rx), (port.y - c.y) / (ry * ry));
        return dx.hypot(dy) >= 20.0 - epsilon && dx * outward.0 + dy * outward.1 > epsilon;
    }
    let left = (port.x - node.x).abs() < epsilon && normal(-dx, dy);
    let right = (port.x - node.x - node.width).abs() < epsilon
        && visible_top.is_none_or(|(_, tab_height)| port.y >= node.y + tab_height - epsilon)
        && normal(dx, dy);
    let bottom = (port.y - node.y - node.height).abs() < epsilon && normal(dy, dx);
    let top = visible_top.map_or((port.y - node.y).abs() < epsilon, |(tab_w, tab_h)| {
        ((port.y - node.y).abs() < epsilon && port.x <= node.x + tab_w + epsilon)
            || ((port.y - node.y - tab_h).abs() < epsilon && port.x >= node.x + tab_w - epsilon)
    }) && normal(-dy, dx);
    left || right || top || bottom
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compact_junction_slopes_keep_outward_clearance_and_reject_tangency() {
        let diagram = crate::parser::parse("semantics {\nA\n}\n").unwrap();
        let mut node = crate::geometry::place(&diagram, &crate::styles::StyleSheet::default())
            .unwrap()
            .remove(0);
        node.kind = crate::syntax::NodeKind::Association;
        node.shape = NodeShape::Diamond;
        node.width = 101.0; // The label's width is not the glyph's radius.
        node.height = 25.5;
        let c = super::super::geometry::center(&node);
        let dock = Point {
            x: c.x + 6.0,
            y: c.y - 6.0,
        };
        assert!(valid(
            &node,
            dock,
            Point {
                x: dock.x,
                y: dock.y - 24.0
            },
            false,
            None
        ));
        assert!(!valid(
            &node,
            dock,
            Point {
                x: dock.x,
                y: dock.y - 19.0
            },
            false,
            None
        ));
        assert!(!valid(
            &node,
            dock,
            Point {
                x: dock.x,
                y: dock.y + 24.0
            },
            false,
            None
        ));
        assert!(!valid(
            &node,
            dock,
            Point {
                x: dock.x + 24.0,
                y: dock.y + 24.0
            },
            true,
            None
        ));
    }
}
