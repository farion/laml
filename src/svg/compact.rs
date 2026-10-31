//! Render-only interface glyphs; the classifier box is omitted only after validation.
use crate::geometry::BoxNode;
use crate::syntax::NodeKind;
use std::fmt::Write;

pub(super) fn draw(svg: &mut String, node: &BoxNode) {
    let cy = node.y + node.height / 2.0;
    if node.kind == NodeKind::Socket {
        writeln!(svg,"<path d=\"M{:.2},{:.2} A10,10 0 0 0 {:.2},{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
            node.x+10.0,cy-10.0,node.x+10.0,cy+10.0,node.border_color,node.border_width).unwrap();
    } else {
        writeln!(svg,"<circle cx=\"{:.2}\" cy=\"{cy:.2}\" r=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
            node.x+10.0,node.fill,node.border_color,node.border_width).unwrap();
    }
    writeln!(svg,"<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\">{}</text>",
        node.x+28.0,cy+node.font_size*0.28,node.font_size,node.color,super::escape(&node.label)).unwrap();
}
