//! UML deployment target: measured perspective cube drawn within its collision box.
use crate::geometry::BoxNode;
use std::fmt::Write;

pub(super) fn center(node: &BoxNode) -> f64 {
    node.x
        + (node.width
            - if node.kind.is_deployment_target() {
                14.0
            } else {
                0.0
            })
            / 2.0
}

pub(super) fn backgrounds(svg: &mut String, boxes: &[BoxNode]) {
    for node in boxes
        .iter()
        .filter(|node| node.kind.is_deployment_target() && node.is_container)
    {
        cube(svg, node);
    }
}

pub(super) fn cube(svg: &mut String, node: &BoxNode) {
    let (x, y, w, h, d) = (node.x, node.y, node.width, node.height, 14.0);
    writeln!(
        svg,
        "<g data-deployment-cube=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\">",
        super::escape(&node.id),
        node.fill,
        node.border_color,
        node.border_width
    )
    .unwrap();
    writeln!(
        svg,
        "<polygon points=\"{x:.2},{:.2} {:.2},{y:.2} {:.2},{y:.2} {:.2},{:.2}\"/>",
        y + d,
        x + d,
        x + w,
        x + w - d,
        y + d
    )
    .unwrap();
    writeln!(
        svg,
        "<polygon points=\"{:.2},{:.2} {:.2},{y:.2} {:.2},{:.2} {:.2},{:.2}\"/>",
        x + w - d,
        y + d,
        x + w,
        x + w,
        y + h - d,
        x + w - d,
        y + h
    )
    .unwrap();
    writeln!(
        svg,
        "<rect x=\"{x:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\"/></g>",
        y + d,
        w - d,
        h - d
    )
    .unwrap();
}
