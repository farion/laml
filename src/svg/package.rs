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
    let (x, y, right, bottom) = (node.x, node.y, node.x + node.width, node.y + node.height);
    let r = node.radius.min(tab_height / 3.0).min(tab_width / 3.0);
    let outline = if r > 0.0 {
        format!(
            "M{x:.2},{:.2} V{:.2} Q{x:.2},{y:.2} {:.2},{y:.2} H{:.2} Q{:.2},{y:.2} {:.2},{:.2} V{:.2} H{:.2} Q{right:.2},{:.2} {right:.2},{:.2} V{:.2} Q{right:.2},{bottom:.2} {:.2},{bottom:.2} H{:.2} Q{x:.2},{bottom:.2} {x:.2},{:.2} Z",
            y + tab_height,
            y + r,
            x + r,
            x + tab_width - r,
            x + tab_width,
            x + tab_width,
            y + r,
            y + tab_height,
            right - r,
            y + tab_height,
            y + tab_height + r,
            bottom - r,
            right - r,
            x + r,
            bottom - r
        )
    } else {
        format!(
            "M{x:.2},{:.2} V{y:.2} H{:.2} V{:.2} H{right:.2} V{bottom:.2} H{x:.2} Z",
            y + tab_height,
            x + tab_width,
            y + tab_height
        )
    };
    writeln!(svg,"<g data-node-id=\"{}\"><title>{}</title><path d=\"{outline}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
        super::escape(&node.id), super::escape(&node.label),node.fill,node.border_color,node.border_width).unwrap();
    for (index, line) in node.lines.iter().enumerate() {
        writeln!(svg,"<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\">{}</text>",
            if node.is_container { node.x+tab_width/2.0 } else { node.x+node.width/2.0 },
            node.y+if node.is_container { (tab_height-step*node.lines.len() as f64)/2.0 } else { node.padding+tab_height }+step*(index as f64+0.78),
            node.font_size,node.color,super::escape(line)).unwrap();
    }
    writeln!(svg,"<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\" /></g>",
        node.x,node.y+tab_height,node.x+tab_width,node.y+tab_height,node.divider_color,node.border_width).unwrap();
    Ok(())
}
