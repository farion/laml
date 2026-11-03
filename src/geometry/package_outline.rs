//! Shared dimensions of the visible UML package tab and body.
use super::BoxNode;
use crate::text::measure;

pub(crate) fn tab(node: &BoxNode) -> Result<(f64, f64), String> {
    let step = node.font_size * 1.25;
    let height = if node.is_container {
        node.padding + step * node.lines.len() as f64 + 4.0
    } else {
        14.0
    };
    let title_width = node.lines.iter().try_fold(0.0_f64, |max, line| {
        Ok::<f64, String>(max.max(measure(line, node.font_size)?))
    })?;
    let width = if node.is_container {
        (title_width + node.padding * 2.0 + 12.0).min(node.width - 8.0)
    } else {
        32.0_f64.min(node.width - 8.0)
    };
    Ok((width, height))
}
