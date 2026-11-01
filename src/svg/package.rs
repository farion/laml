//! Package tab, frame, and measured heading behind contained graph relations.
use crate::geometry::BoxNode;
use crate::text::measure;
use std::fmt::Write;

pub(super) fn background(svg: &mut String, node: &BoxNode) -> Result<(), String> {
    let step = node.font_size * 1.25;
    let tab_height = if node.is_container {
        node.padding + step * node.lines.len() as f64 + 4.0
    } else {
        14.0
    };
    let title_width = node
        .lines
        .iter()
        .try_fold(0.0_f64, |max, line| -> Result<f64, String> {
            Ok(max.max(measure(line, node.font_size)?))
        })?;
    let tab_width = if node.is_container {
        (title_width + node.padding * 2.0 + 12.0).min(node.width - 8.0)
    } else {
        32.0_f64.min(node.width - 8.0)
    };
    writeln!(svg,"<g data-node-id=\"{}\"><title>{}</title><path d=\"M{:.2},{:.2} V{:.2} H{:.2} V{:.2} H{:.2} V{:.2} H{:.2} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
        super::escape(&node.id), super::escape(&node.label), node.x, node.y+tab_height, node.y,
        node.x+tab_width, node.y+tab_height, node.x+node.width, node.y+node.height, node.x,
        node.fill, node.border_color, node.border_width).unwrap();
    for (index, line) in node.lines.iter().enumerate() {
        writeln!(svg,"<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\">{}</text>",
            if node.is_container { node.x+tab_width/2.0 } else { node.x+node.width/2.0 },
            node.y+if node.is_container { (tab_height-step*node.lines.len() as f64)/2.0 } else { node.padding+tab_height }+step*(index as f64+0.78),
            node.font_size,node.color,super::escape(line)).unwrap();
    }
    writeln!(svg,"<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\" /></g>",
        node.x,node.y+tab_height,node.x+tab_width,node.y+tab_height,node.border_color,node.border_width).unwrap();
    Ok(())
}
