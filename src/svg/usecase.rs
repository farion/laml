//! UML system boundary behind routes and actor stick-figure silhouette.
use crate::geometry::BoxNode;
use std::fmt::Write;

pub(super) fn background(svg: &mut String, boxes: &[BoxNode]) {
    for node in boxes
        .iter()
        .filter(|node| node.kind == crate::syntax::NodeKind::UseCaseSystem)
    {
        writeln!(svg,"<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
            node.x,node.y,node.width,node.height,node.fill,node.border_color,node.border_width).unwrap();
    }
}

pub(super) fn actor(svg: &mut String, node: &BoxNode) {
    let x = node.x + node.width / 2.0;
    writeln!(svg,"<circle cx=\"{x:.2}\" cy=\"{:.2}\" r=\"8\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
        node.y+12.0,node.border_color,node.border_width.max(1.5)).unwrap();
    writeln!(svg,"<path d=\"M{x:.2},{:.2} V{:.2} M{:.2},{:.2} H{:.2} M{x:.2},{:.2} L{:.2},{:.2} M{x:.2},{:.2} L{:.2},{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-linecap=\"round\" />",
        node.y+20.0,node.y+60.0,node.x,node.y+47.0,node.x+node.width,
        node.y+60.0,x-16.0,node.y+70.0,
        node.y+60.0,x+16.0,node.y+70.0,node.border_color,node.border_width.max(1.5)).unwrap();
}
