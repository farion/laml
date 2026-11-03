//! Modern GNE note cards and legacy/UML folded notes share measured geometry.
use crate::geometry::BoxNode;
use std::fmt::Write;

pub(super) fn draw(svg: &mut String, node: &BoxNode, index: usize) {
    if node.note_card {
        writeln!(svg, "<rect data-note-card=\"true\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
            node.x,node.y,node.width,node.height,node.radius,node.fill,node.border_color,node.border_width).unwrap();
        // Clip the accent to the card: it starts flush with the left edge,
        // occupies the entire height, and follows both rounded corners.
        writeln!(svg, "<clipPath id=\"laml-note-accent-{index}\"><rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\"/></clipPath>",
            node.x,node.y,node.width,node.height,node.radius).unwrap();
        writeln!(svg, "<rect data-note-accent=\"true\" x=\"{:.2}\" y=\"{:.2}\" width=\"6\" height=\"{:.2}\" fill=\"{}\" clip-path=\"url(#laml-note-accent-{index})\"/>",
            node.x,node.y,node.height,node.accent_color).unwrap();
    } else {
        let fold = 12.0_f64.min(node.width / 4.0).min(node.height / 4.0);
        writeln!(svg,"<path d=\"M{:.2},{:.2} H{:.2} L{:.2},{:.2} V{:.2} H{:.2} Z M{:.2},{:.2} V{:.2} H{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
            node.x,node.y,node.x+node.width-fold,node.x+node.width,node.y+fold,
            node.y+node.height,node.x,node.x+node.width-fold,node.y,node.y+fold,node.x+node.width,
            node.fill,node.border_color,node.border_width).unwrap();
    }
}
