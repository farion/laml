//! Composite-component background before internal connectors and foreground parts.
use crate::geometry::BoxNode;
use std::fmt::Write;

pub(super) fn icon(svg: &mut String, node: &BoxNode) {
    let (x, y) = (
        node.x + node.width - node.padding - 24.0,
        node.y + node.padding + 2.0,
    );
    writeln!(svg, "<g data-component-icon=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\"><rect x=\"{:.2}\" y=\"{y:.2}\" width=\"18\" height=\"16\"/><rect x=\"{x:.2}\" y=\"{:.2}\" width=\"9\" height=\"4\"/><rect x=\"{x:.2}\" y=\"{:.2}\" width=\"9\" height=\"4\"/></g>",
        super::escape(&node.id), node.fill, node.border_color, node.border_width,
        x+6.0, y+3.0, y+10.0).unwrap();
}

pub(super) fn draw(svg: &mut String, node: &BoxNode) {
    writeln!(svg,
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
        node.x,node.y,node.width,node.height,node.radius,node.fill,node.border_color,node.border_width
    ).unwrap();
    // Port bands already draw their own separator in the foreground.
    if node.ports.is_empty() {
        writeln!(svg,"<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
            node.x,node.y+node.port_start,node.x+node.width,node.y+node.port_start,node.border_color,node.border_width).unwrap();
    }
}
