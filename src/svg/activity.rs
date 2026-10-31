//! UML activity control symbols. Actions and object nodes use measured GNE boxes.
use crate::geometry::BoxNode;
use crate::syntax::NodeKind;
use std::fmt::Write;

pub(super) fn symbol(svg: &mut String, node: &BoxNode) {
    let (x, y) = (node.x + node.width / 2.0, node.y + node.height / 2.0);
    match node.kind {
        NodeKind::ActivityInitial => {
            writeln!(
                svg,
                "<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"9\" fill=\"{}\" />",
                node.border_color
            )
            .unwrap();
        }
        NodeKind::ActivityFinal | NodeKind::ActivityFlowFinal => {
            writeln!(svg, "<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"12\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />", node.fill, node.border_color, node.border_width).unwrap();
            if node.kind == NodeKind::ActivityFinal {
                writeln!(
                    svg,
                    "<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"6\" fill=\"{}\" />",
                    node.border_color
                )
                .unwrap();
            } else {
                writeln!(svg, "<path d=\"M{:.2},{:.2} L{:.2},{:.2} M{:.2},{:.2} L{:.2},{:.2}\" stroke=\"{}\" stroke-width=\"2\" />", x-5.0,y-5.0,x+5.0,y+5.0,x-5.0,y+5.0,x+5.0,y-5.0,node.border_color).unwrap();
            }
        }
        NodeKind::ActivityDecision | NodeKind::ActivityMerge => {
            writeln!(svg, "<polygon points=\"{x:.2},{:.2} {:.2},{y:.2} {x:.2},{:.2} {:.2},{y:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                y-12.0,x+12.0,y+12.0,x-12.0,node.fill,node.border_color,node.border_width).unwrap();
        }
        NodeKind::ActivityFork | NodeKind::ActivityJoin => {
            writeln!(
                svg,
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" />",
                node.x, node.y, node.width, node.height, node.border_color
            )
            .unwrap();
        }
        _ => unreachable!("checked activity control node"),
    }
}
