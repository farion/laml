//! Contour-aware labels and boundary anchors for shaped generic nodes.
use super::{BoxNode, GeometryError, NodeShape};
use crate::syntax::PortSide;
use crate::text::{measure, measure_italic};

fn contains(node: &BoxNode, x: f64, y: f64) -> bool {
    let dx = (x - node.x - node.width / 2.0) / (node.width / 2.0);
    let dy = (y - node.y - node.height / 2.0) / (node.height / 2.0);
    match node.shape {
        NodeShape::Ellipse => dx * dx + dy * dy <= 0.96,
        NodeShape::Diamond => dx.abs() + dy.abs() <= 0.98,
        _ => true,
    }
}
fn fits(node: &BoxNode, x: f64, y: f64, w: f64, h: f64) -> bool {
    [(x, y), (x + w, y), (x, y + h), (x + w, y + h)]
        .into_iter()
        .all(|(x, y)| contains(node, x, y))
}

pub(super) fn validate(boxes: &[BoxNode]) -> Result<(), GeometryError> {
    for node in boxes.iter().filter(|node| {
        !node.ports.is_empty() && matches!(node.shape, NodeShape::Ellipse | NodeShape::Diamond)
    }) {
        let fail = || GeometryError {
            line: node.line,
            message: "shaped node cannot fit port or header labels inside its contour".into(),
        };
        if !node.icons.is_empty() {
            return Err(fail());
        }
        let size = node.font_size;
        let mut text_rects: Vec<(f64, f64, f64, f64)> = Vec::new();
        let line_height = size * 1.25;
        let header = node.y + node.padding + node.top_band + line_height * 0.78;
        for (index, text) in node.lines.iter().enumerate() {
            let width = if node.abstract_node && index + 1 == node.lines.len() {
                measure_italic(text, size)
            } else {
                measure(text, size)
            }
            .map_err(|_| fail())?;
            let baseline = header + index as f64 * line_height;
            let rect = (
                node.x + (node.width - width) / 2.0,
                baseline - size * 0.82,
                width,
                size * 1.05,
            );
            if !fits(node, rect.0, rect.1, rect.2, rect.3) {
                return Err(fail());
            }
            text_rects.push(rect);
        }
        for port in &node.ports {
            let width = measure(&port.label, size).map_err(|_| fail())?;
            let (x, baseline) = match port.side {
                PortSide::Left => (port.x + node.padding + 4.0, port.y + size * 0.28),
                PortSide::Right => (port.x - node.padding - 4.0 - width, port.y + size * 0.28),
                PortSide::Top => (port.x - width / 2.0, node.y + node.padding + size * 0.78),
                PortSide::Bottom => (
                    port.x - width / 2.0,
                    node.y + node.height - node.padding - node.bottom_band + 4.0 + size * 0.78,
                ),
            };
            let rect = (x, baseline - size * 0.82, width, size * 1.05);
            if !fits(node, rect.0, rect.1, rect.2, rect.3)
                || text_rects.iter().any(|other| {
                    rect.0 < other.0 + other.2 + 2.0
                        && rect.0 + rect.2 + 2.0 > other.0
                        && rect.1 < other.1 + other.3 + 2.0
                        && rect.1 + rect.3 + 2.0 > other.1
                })
            {
                return Err(fail());
            }
            text_rects.push(rect);
        }
    }
    Ok(())
}
