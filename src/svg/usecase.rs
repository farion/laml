//! UML system boundary behind routes and actor stick-figure silhouette.
use crate::geometry::BoxNode;
use std::fmt::Write;

pub(super) fn background(svg: &mut String, boxes: &[BoxNode]) {
    for node in boxes
        .iter()
        .filter(|node| node.kind == crate::syntax::NodeKind::UseCaseSystem)
    {
        writeln!(svg,"<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
            node.x,node.y,node.width,node.height,node.radius,node.fill,node.border_color,node.border_width).unwrap();
    }
}

pub(super) fn actor(svg: &mut String, node: &BoxNode) {
    use crate::geometry::usecase::{
        ARM_HALF, ARM_Y, FOOT_OFFSET, FOOT_Y, HEAD_CENTER_Y, HEAD_RADIUS, HIP_Y,
    };
    let x = node.x + node.width / 2.0;
    let y = node.y;
    let stroke = node.border_width.max(1.8);
    writeln!(svg,"<circle cx=\"{x:.2}\" cy=\"{:.2}\" r=\"{HEAD_RADIUS:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{stroke:.2}\" />",
        y+HEAD_CENTER_Y,node.border_color).unwrap();
    writeln!(svg,"<path d=\"M{x:.2},{:.2} V{:.2} M{:.2},{:.2} H{:.2} M{x:.2},{:.2} L{:.2},{:.2} M{x:.2},{:.2} L{:.2},{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{stroke:.2}\" stroke-linecap=\"round\" stroke-linejoin=\"round\" />",
        y+HEAD_CENTER_Y+HEAD_RADIUS,y+HIP_Y,x-ARM_HALF,y+ARM_Y,x+ARM_HALF,
        y+HIP_Y,x-FOOT_OFFSET,y+FOOT_Y,
        y+HIP_Y,x+FOOT_OFFSET,y+FOOT_Y,node.border_color).unwrap();
}
