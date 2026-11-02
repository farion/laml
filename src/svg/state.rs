//! UML state-machine vertices and composite-state background.
use crate::geometry::BoxNode;
use crate::syntax::NodeKind;
use std::fmt::Write;

pub(super) fn backgrounds(svg: &mut String, boxes: &[BoxNode]) {
    for node in boxes
        .iter()
        .filter(|node| node.kind == NodeKind::State && node.is_container)
    {
        writeln!(svg,"<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
            node.x,node.y,node.width,node.height,node.radius,node.fill,node.border_color,node.border_width).unwrap();
        writeln!(svg,"<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
            node.x,node.y+node.port_start,node.x+node.width,node.y+node.port_start,node.divider_color,node.border_width).unwrap();
    }
}

pub(super) fn symbol(svg: &mut String, node: &BoxNode) {
    let (x, y) = (node.x + node.width / 2.0, node.y + node.height / 2.0);
    match node.kind {
        NodeKind::StateInitial => writeln!(svg,"<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"9\" fill=\"{}\" />",node.border_color).unwrap(),
        NodeKind::StateFinal => {
            writeln!(svg,"<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"12\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                node.fill,node.border_color,node.border_width).unwrap();
            writeln!(svg,"<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"6\" fill=\"{}\" />",node.border_color).unwrap();
        }
        NodeKind::StateChoice => writeln!(svg,"<polygon points=\"{x:.2},{:.2} {:.2},{y:.2} {x:.2},{:.2} {:.2},{y:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
            y-12.0,x+12.0,y+12.0,x-12.0,node.fill,node.border_color,node.border_width).unwrap(),
        _ => unreachable!("checked state vertex"),
    }
}
