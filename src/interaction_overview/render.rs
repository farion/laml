//! Wrap checked GNE ref frames and activity control symbols in one sd interaction frame.
use super::Document;
use crate::geometry::BoxNode;
use crate::route::Route;
use crate::svg::{self, TextMode};
use crate::text::measure;

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn render(
    doc: &Document,
    boxes: &[BoxNode],
    routes: &[Route],
    mode: TextMode,
) -> Result<String, String> {
    if boxes.len() != doc.graph.nodes.len() || routes.len() != doc.graph.relations.len() {
        return Err("interaction overview requires all control vertices and flows visible".into());
    }
    let base_mode = if mode == TextMode::Paths {
        TextMode::Embedded
    } else {
        mode
    };
    let mut svg = svg::render_with_mode(boxes, routes, base_mode)?;
    let mut bounds = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    let mut include = |x: f64, y: f64| {
        bounds.0 = bounds.0.min(x);
        bounds.1 = bounds.1.min(y);
        bounds.2 = bounds.2.max(x);
        bounds.3 = bounds.3.max(y);
    };
    for node in boxes {
        include(node.x, node.y);
        include(node.x + node.width, node.y + node.height);
    }
    for route in routes {
        for point in &route.points {
            include(point.x, point.y);
        }
        for label in route.label.iter().chain(route.end_labels.iter()) {
            include(label.x, label.y);
            include(label.x + label.width, label.y + label.height);
        }
    }
    let left = bounds.0 - 24.0;
    let top = bounds.1 - 32.0;
    let right =
        (bounds.2 + 24.0).max(left + measure(&format!("sd {}", doc.interaction), 13.0)? + 20.0);
    let bottom = bounds.3 + 24.0;
    let frame = format!(
        "<g data-interaction=\"{}\"><rect x=\"{left:.2}\" y=\"{top:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"#395373\" /><text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"13\" fill=\"#17212e\">sd {}</text></g>\n",
        escape(&doc.interaction),
        right - left,
        bottom - top,
        left + 9.0,
        top + 16.0,
        escape(&doc.interaction)
    );
    let after = svg.find("</defs>\n").ok_or("missing SVG definitions")? + "</defs>\n".len();
    svg.insert_str(after, &frame);
    let canvas = (left - 32.0, top - 32.0, right + 32.0, bottom + 32.0);
    let opening = svg.find('>').ok_or("missing SVG opening tag")? + 1;
    svg.replace_range(..opening,&format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.2}\" height=\"{:.2}\" viewBox=\"{:.2} {:.2} {:.2} {:.2}\">",
        canvas.2-canvas.0,canvas.3-canvas.1,canvas.0,canvas.1,canvas.2-canvas.0,canvas.3-canvas.1));
    if mode == TextMode::Paths {
        crate::export::outlines(&svg)
    } else {
        Ok(svg)
    }
}
