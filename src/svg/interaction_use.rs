//! UML InteractionUse is a ref fragment, not an activity action box.
use crate::geometry::BoxNode;
use std::fmt::Write;

pub(super) fn tab(svg: &mut String, node: &BoxNode) {
    writeln!(svg,"<path d=\"M{:.2},{:.2} h36 l8,16 h-44 Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
        node.x,node.y,node.fill,node.border_color,node.border_width).unwrap();
    writeln!(svg,"<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"12\" fill=\"{}\">ref</text>",
        node.x+6.0,node.y+12.0,node.color).unwrap();
}
