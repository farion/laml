//! SVG output with selectable or outlined text measured against bundled Inter.

use crate::geometry::{BoxNode, NodeShape};
use crate::route::{Point, Route};
use crate::syntax::{EndSymbol, NodeKind, NodePart};
use crate::text::{FONT, measure};
use base64::Engine;
use std::fmt::Write;

fn escape(value: &str) -> String {
    let mut output = String::new();
    for ch in value.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(ch),
        }
    }
    output
}
fn marker(symbol: EndSymbol, start: bool) -> &'static str {
    match (symbol, start) {
        (EndSymbol::None, _) => "",
        (EndSymbol::Arrow, _) => "url(#arrow)",
        (EndSymbol::Triangle, _) => "url(#triangle)",
        (EndSymbol::OpenDiamond, true) => "url(#open-diamond)",
        (EndSymbol::OpenDiamond, false) => "url(#open-diamond-end)",
        (EndSymbol::FilledDiamond, true) => "url(#filled-diamond)",
        (EndSymbol::FilledDiamond, false) => "url(#filled-diamond-end)",
    }
}
fn include_point(point: Point, bounds: &mut (f64, f64, f64, f64)) {
    bounds.0 = bounds.0.min(point.x);
    bounds.1 = bounds.1.min(point.y);
    bounds.2 = bounds.2.max(point.x);
    bounds.3 = bounds.3.max(point.y);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextMode {
    Embedded,
    System,
    Paths,
}

pub fn render(boxes: &[BoxNode], routes: &[Route]) -> Result<String, String> {
    render_with_mode(boxes, routes, TextMode::Embedded)
}

pub fn render_with_mode(
    boxes: &[BoxNode],
    routes: &[Route],
    mode: TextMode,
) -> Result<String, String> {
    let svg = render_svg(boxes, routes, mode)?;
    if mode == TextMode::Paths {
        crate::export::outlines(&svg)
    } else {
        Ok(svg)
    }
}

fn render_svg(boxes: &[BoxNode], routes: &[Route], mode: TextMode) -> Result<String, String> {
    let mut bounds = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
    for node in boxes {
        include_point(
            Point {
                x: node.x,
                y: node.y,
            },
            &mut bounds,
        );
        include_point(
            Point {
                x: node.x + node.width,
                y: node.y + node.height,
            },
            &mut bounds,
        );
    }
    for route in routes {
        for &point in &route.points {
            include_point(point, &mut bounds);
        }
        for bridge in &route.bridges {
            let pair = &route.points[bridge.segment..=bridge.segment + 1];
            let dx = pair[1].x - pair[0].x;
            let dy = pair[1].y - pair[0].y;
            let length = dx.hypot(dy);
            let center = Point {
                x: pair[0].x + dx * bridge.distance / length,
                y: pair[0].y + dy * bridge.distance / length,
            };
            include_point(
                Point {
                    x: center.x + dy / length * bridge.height * 2.0,
                    y: center.y - dx / length * bridge.height * 2.0,
                },
                &mut bounds,
            );
        }
        for label in route.label.iter().chain(route.end_labels.iter()) {
            include_point(
                Point {
                    x: label.x,
                    y: label.y,
                },
                &mut bounds,
            );
            include_point(
                Point {
                    x: label.x + label.width,
                    y: label.y + label.height,
                },
                &mut bounds,
            );
        }
    }
    let (left, top, right, bottom) = (
        bounds.0 - 32.0,
        bounds.1 - 32.0,
        bounds.2 + 32.0,
        bounds.3 + 32.0,
    );
    let mut svg = String::new();
    writeln!(svg, "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.2}\" height=\"{:.2}\" viewBox=\"{left:.2} {top:.2} {:.2} {:.2}\">", right-left,bottom-top,right-left,bottom-top).unwrap();
    svg.push_str("<defs>");
    if mode == TextMode::Embedded {
        let font = base64::engine::general_purpose::STANDARD.encode(FONT);
        writeln!(svg, "<style>@font-face {{ font-family: 'Inter'; src: url('data:font/otf;base64,{font}') format('opentype'); }}</style>").unwrap();
    }
    svg.push_str("<marker id=\"arrow\" markerWidth=\"10\" markerHeight=\"8\" refX=\"9\" refY=\"4\" orient=\"auto-start-reverse\" markerUnits=\"userSpaceOnUse\"><path d=\"M0,0 L10,4 L0,8 Z\" fill=\"context-stroke\"/></marker>");
    svg.push_str("<marker id=\"triangle\" markerWidth=\"13\" markerHeight=\"11\" refX=\"12\" refY=\"5.5\" orient=\"auto-start-reverse\" markerUnits=\"userSpaceOnUse\"><path d=\"M1,1 L12,5.5 L1,10 Z\" fill=\"white\" stroke=\"context-stroke\" stroke-width=\"1.5\"/></marker>");
    svg.push_str("<marker id=\"open-diamond\" markerWidth=\"17\" markerHeight=\"11\" refX=\"1\" refY=\"5.5\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"><path d=\"M1,5.5 L8,1 L16,5.5 L8,10 Z\" fill=\"white\" stroke=\"context-stroke\" stroke-width=\"1.5\"/></marker>");
    svg.push_str("<marker id=\"filled-diamond\" markerWidth=\"17\" markerHeight=\"11\" refX=\"1\" refY=\"5.5\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"><path d=\"M1,5.5 L8,1 L16,5.5 L8,10 Z\" fill=\"context-stroke\"/></marker>");
    svg.push_str("<marker id=\"open-diamond-end\" markerWidth=\"17\" markerHeight=\"11\" refX=\"16\" refY=\"5.5\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"><path d=\"M1,5.5 L8,1 L16,5.5 L8,10 Z\" fill=\"white\" stroke=\"context-stroke\" stroke-width=\"1.5\"/></marker>");
    svg.push_str("<marker id=\"filled-diamond-end\" markerWidth=\"17\" markerHeight=\"11\" refX=\"16\" refY=\"5.5\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"><path d=\"M1,5.5 L8,1 L16,5.5 L8,10 Z\" fill=\"context-stroke\"/></marker></defs>\n");
    for route in routes {
        let start = marker(route.appearance.start, true);
        let end = marker(route.appearance.end, false);
        let start_marker = if start.is_empty() {
            String::new()
        } else {
            format!(" marker-start=\"{start}\"")
        };
        let end_marker = if end.is_empty() {
            String::new()
        } else {
            format!(" marker-end=\"{end}\"")
        };
        let dash = if route.appearance.dashed {
            " stroke-dasharray=\"6 4\""
        } else {
            ""
        };
        if route.bridges.is_empty() && route.gaps.is_empty() {
            let points: Vec<String> = route
                .points
                .iter()
                .map(|p| format!("{:.2},{:.2}", p.x, p.y))
                .collect();
            writeln!(svg,"<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"{dash}{end_marker}{start_marker} />",
                points.join(" "), route.color, route.width).unwrap();
        } else {
            let fragments = crate::svg_paths::fragments(route)?;
            let last = fragments.len() - 1;
            for (i, path) in fragments.iter().enumerate() {
                writeln!(svg,"<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"{dash}{}{} />",
                    path,route.color,route.width,
                    if i==0 { &start_marker } else { "" },
                    if i==last { &end_marker } else { "" }).unwrap();
            }
        }
        for label in route.label.iter().chain(route.end_labels.iter()) {
            let cx = label.x + label.width / 2.0;
            let cy = label.y + label.height / 2.0;
            let rotation = if label.angle.abs() > 1e-9 {
                format!(" transform=\"rotate({:.2} {cx:.2} {cy:.2})\"", label.angle)
            } else {
                String::new()
            };
            writeln!(svg,"<text x=\"{cx:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\"{}>{}</text>",
                cy+route.font_size*1.25*0.28,route.font_size,route.color,rotation,escape(&label.text)).unwrap();
        }
    }
    for node in boxes {
        writeln!(
            svg,
            "<g data-node-id=\"{}\"><title>{}</title>",
            escape(&node.id),
            escape(&node.label)
        )
        .unwrap();
        if node.shape == NodeShape::Note {
            let fold = 12.0_f64.min(node.width / 4.0).min(node.height / 4.0);
            writeln!(svg,"<path d=\"M{:.2},{:.2} H{:.2} L{:.2},{:.2} V{:.2} H{:.2} Z M{:.2},{:.2} V{:.2} H{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                node.x,node.y,node.x+node.width-fold,node.x+node.width,node.y+fold,
                node.y+node.height,node.x,node.x+node.width-fold,node.y,node.y+fold,node.x+node.width,
                node.fill,node.border_color,node.border_width).unwrap();
        } else if node.shape == NodeShape::Ellipse {
            writeln!(svg,"<ellipse cx=\"{:.2}\" cy=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                node.x+node.width/2.0,node.y+node.height/2.0,node.width/2.0,node.height/2.0,node.fill,node.border_color,node.border_width).unwrap();
        } else if node.shape == NodeShape::Diamond {
            writeln!(svg,"<polygon points=\"{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                node.x+node.width/2.0,node.y,node.x+node.width,node.y+node.height/2.0,
                node.x+node.width/2.0,node.y+node.height,node.x,node.y+node.height/2.0,
                node.fill,node.border_color,node.border_width).unwrap();
        } else {
            writeln!(svg,"<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                node.x,node.y,node.width,node.height,node.radius,node.fill,node.border_color,node.border_width).unwrap();
        }
        let line_height = node.font_size * 1.25;
        let first_baseline = if node.body.is_empty() {
            node.y
                + (node.height - line_height * node.lines.len() as f64) / 2.0
                + line_height * 0.78
        } else {
            node.y + node.padding + line_height * 0.78
        };
        for (i, line) in node.lines.iter().enumerate() {
            writeln!(svg,"<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\"{}>{}</text>",
                node.x+node.width/2.0,first_baseline+line_height*i as f64,node.font_size,node.color,
                if node.abstract_node { " font-style=\"italic\"" } else { "" },escape(line)).unwrap();
            if node.kind == NodeKind::Object {
                let width = measure(line, node.font_size)?;
                let center = node.x + node.width / 2.0;
                let baseline = first_baseline + line_height * i as f64;
                writeln!(svg,"<line x1=\"{:.2}\" x2=\"{:.2}\" y1=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1\" />",
                    center-width/2.0,center+width/2.0,baseline+2.0,baseline+2.0,node.color).unwrap();
            }
        }
        if !node.body.is_empty() {
            let mut y = node.y + node.padding + line_height * node.lines.len() as f64;
            writeln!(svg,"<line x1=\"{:.2}\" y1=\"{y:.2}\" x2=\"{:.2}\" y2=\"{y:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                node.x, node.x+node.width, node.border_color, node.border_width).unwrap();
            y += 4.0;
            for (part, row) in node.body.iter().zip(&node.body_styles) {
                match part {
                    NodePart::Separator => {
                        y += 4.0;
                        writeln!(svg,"<line x1=\"{:.2}\" y1=\"{y:.2}\" x2=\"{:.2}\" y2=\"{y:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                            node.x,node.x+node.width,node.border_color,node.border_width).unwrap();
                        y += 4.0;
                    }
                    NodePart::Member {
                        id,
                        text,
                        abstract_member,
                        static_member,
                        ..
                    } => {
                        let baseline = y + row.font_size * 1.25 * 0.78;
                        writeln!(svg,"<text x=\"{:.2}\" y=\"{baseline:.2}\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\"{}{}>{}</text>",
                            node.x+node.padding,row.font_size,row.color,
                            if *abstract_member { " font-style=\"italic\"" } else { "" },
                            id.as_ref().map(|id| format!(" data-member-id=\"{}\"",escape(id))).unwrap_or_default(),
                            escape(text)).unwrap();
                        if *static_member {
                            let width = measure(text, row.font_size)?;
                            writeln!(svg,"<line x1=\"{:.2}\" x2=\"{:.2}\" y1=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1\" />",
                                node.x+node.padding,node.x+node.padding+width,baseline+2.0,baseline+2.0,row.color).unwrap();
                        }
                        y += row.font_size * 1.25;
                    }
                }
            }
        }
        svg.push_str("</g>\n");
    }
    svg.push_str("</svg>\n");
    Ok(svg)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escapes_untrusted_labels() {
        assert_eq!(escape("<&\"'"), "&lt;&amp;&quot;&apos;");
    }
}
