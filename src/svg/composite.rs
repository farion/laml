//! The mandatory structured-classifier internal-structure compartment and dashed role box.
use crate::geometry::{BoxNode, composite::header_height};
use crate::route::{Point, Route};
use crate::text::measure;
use std::fmt::Write;

pub(super) fn background(svg: &mut String, node: &BoxNode) {
    let separator = node.y + header_height(node) + 7.0;
    writeln!(svg,"<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
        node.x,node.y,node.width,node.height,node.radius,node.fill,node.border_color,node.border_width).unwrap();
    writeln!(svg,"<path d=\"M{:.2},{separator:.2} H{:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\" /><text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"12\" fill=\"{}\">internal structure</text>",
        node.x,node.x+node.width,node.divider_color,node.border_width,node.x+node.padding+24.0,separator+17.0,node.color).unwrap();
}

fn crosses(a: Point, b: Point, rect: (f64, f64, f64, f64)) -> bool {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let mut low = 0.0_f64;
    let mut high = 1.0_f64;
    for (p, q) in [
        (-dx, a.x - rect.0),
        (dx, rect.2 - a.x),
        (-dy, a.y - rect.1),
        (dy, rect.3 - a.y),
    ] {
        if p.abs() < 1e-9 {
            if q < 0.0 {
                return false;
            }
            continue;
        }
        let ratio = q / p;
        if p < 0.0 {
            low = low.max(ratio);
        } else {
            high = high.min(ratio);
        }
        if low > high {
            return false;
        }
    }
    true
}

pub(super) fn validate(boxes: &[BoxNode], routes: &[Route]) -> Result<(), String> {
    for node in boxes.iter().filter(|node| {
        node.kind == crate::syntax::NodeKind::CompositeStructure && node.is_container
    }) {
        let width = measure("internal structure", 12.0)?;
        let x = node.x + node.padding + 24.0;
        let y = node.y + header_height(node) + 7.0;
        if x + width + node.padding > node.x + node.width {
            return Err("internal structure label does not fit its classifier frame".into());
        }
        let rect = (x - 3.0, y + 2.0, x + width + 3.0, y + 21.0);
        if routes.iter().any(|r| {
            r.points
                .windows(2)
                .any(|points| crosses(points[0], points[1], rect))
        }) {
            return Err(
                "composite connector crosses internal structure label; change port side or layout"
                    .into(),
            );
        }
    }
    Ok(())
}

pub(super) fn role(svg: &mut String, node: &BoxNode) {
    writeln!(svg,"<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-dasharray=\"6 4\" />",
        node.x,node.y,node.width,node.height,node.radius,node.fill,node.border_color,node.border_width).unwrap();
}
