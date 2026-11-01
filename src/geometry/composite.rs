//! Header and internal-structure compartment geometry shared by GNE and SVG.
use super::BoxNode;
use crate::syntax::PortSide;

pub(crate) fn header_height(node: &BoxNode) -> f64 {
    let side_ports = node
        .ports
        .iter()
        .filter(|p| matches!(p.side, PortSide::Left | PortSide::Right))
        .count();
    (node.port_start + side_ports as f64 * node.font_size * 1.25 + node.padding).max(50.0)
}
