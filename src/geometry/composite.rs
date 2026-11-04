//! Header and internal-structure compartment geometry shared by GNE and SVG.
use super::BoxNode;
use crate::syntax::PortSide;

pub(crate) fn internal_label_rect(node: &BoxNode) -> Result<(f64, f64, f64, f64), String> {
    let width = crate::text::measure("internal structure", 12.0)?;
    let x = node.x + node.padding + 24.0;
    let y = node.y + header_height(node) + 7.0;
    if x + width + node.padding > node.x + node.width {
        return Err("internal structure label does not fit its classifier frame".into());
    }
    Ok((x - 3.0, y + 2.0, x + width + 3.0, y + 21.0))
}

pub(crate) fn header_height(node: &BoxNode) -> f64 {
    let side_ports = node
        .ports
        .iter()
        .filter(|p| matches!(p.side, PortSide::Left | PortSide::Right))
        .count();
    (node.port_start + side_ports as f64 * node.font_size * 1.25 + node.padding).max(50.0)
}
