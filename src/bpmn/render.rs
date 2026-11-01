//! BPMN SVG renderer: participant bands, independent flow kinds and shape markers.
use super::geometry::{BoxAt, Geometry, place};
use super::{Document, EdgeKind, Kind, Node};
use crate::styles::{StyleSheet, Value};
use crate::svg::TextMode;
use crate::text::FONT;
use base64::Engine;
use std::fmt::Write;

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn text(svg: &mut String, x: f64, y: f64, s: &str, size: f64) {
    for (i, line) in s.split('\n').enumerate() {
        writeln!(svg,"<text x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"{size:.1}\" fill=\"#182330\">{}</text>",y+i as f64*size*1.2,escape(line)).unwrap();
    }
}
fn phosphor(svg: &mut String, icon: &str, x: f64, y: f64, size: f64) {
    crate::svg_icons::draw(svg, icon, x, y, size, "#263548").unwrap();
}
fn event_marker(svg: &mut String, marker: &str, x: f64, y: f64, throwing: bool) {
    match marker {
        "timer" => phosphor(svg, "clock", x - 13.0, y - 13.0, 26.0),
        "message" => {
            let fill = if throwing { "#263548" } else { "white" };
            let flap = if throwing { "white" } else { "#263548" };
            writeln!(svg,"<rect x=\"{:.1}\" y=\"{:.1}\" width=\"23\" height=\"16\" fill=\"{fill}\" stroke=\"#263548\" stroke-width=\"1.7\"/><path d=\"M {:.1} {:.1} l 11.5 8 l 11.5 -8\" fill=\"none\" stroke=\"{flap}\"/>",x-11.5,y-8.0,x-11.5,y-8.0).unwrap();
        }
        "error" => {
            writeln!(
                svg,
                "<path d=\"M {:.1} {:.1} l 7 -12 -2 10 9 -3 -12 16 3 -11 Z\" fill=\"#263548\"/>",
                x - 8.0,
                y + 9.0
            )
            .unwrap();
        }
        "terminate" => {
            writeln!(
                svg,
                "<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"12\" fill=\"#263548\"/>"
            )
            .unwrap();
        }
        _ => {}
    }
}
fn shape(svg: &mut String, n: &Node, b: BoxAt, icon: Option<&str>) {
    let (x, y, w, h) = (b.x, b.y, b.w, b.h);
    match n.kind {
        Kind::Task
        | Kind::UserTask
        | Kind::ServiceTask
        | Kind::SendTask
        | Kind::ReceiveTask
        | Kind::ScriptTask
        | Kind::ManualTask
        | Kind::Collapsed
        | Kind::Subprocess => {
            writeln!(svg,"<rect class=\"bpmn-body\" x=\"{:.1}\" y=\"{:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" rx=\"9\" fill=\"#fff\" stroke=\"#263548\" stroke-width=\"2\"/>",x-w/2.0,y-h/2.0).unwrap();
            if let Some(icon) = icon {
                let typed = !matches!(n.kind, Kind::Task | Kind::Collapsed | Kind::Subprocess);
                let icon_x = if typed {
                    x + w / 2.0 - 33.0
                } else {
                    x - w / 2.0 + 12.0
                };
                phosphor(svg, icon, icon_x, y - h / 2.0 + 10.0, 22.0);
            }
            match n.kind {
                Kind::UserTask => {
                    phosphor(svg, "user", x - w / 2.0 + 11.0, y - h / 2.0 + 8.0, 19.0)
                }
                Kind::ServiceTask => {
                    phosphor(svg, "gear", x - w / 2.0 + 8.0, y - h / 2.0 + 7.0, 17.0);
                    phosphor(svg, "gear", x - w / 2.0 + 21.0, y - h / 2.0 + 14.0, 12.0);
                }
                Kind::SendTask | Kind::ReceiveTask => event_marker(
                    svg,
                    "message",
                    x - w / 2.0 + 23.0,
                    y - h / 2.0 + 20.0,
                    n.kind == Kind::SendTask,
                ),
                Kind::ScriptTask => {
                    phosphor(svg, "scroll", x - w / 2.0 + 9.0, y - h / 2.0 + 8.0, 21.0)
                }
                Kind::ManualTask => {
                    phosphor(svg, "hand", x - w / 2.0 + 9.0, y - h / 2.0 + 8.0, 21.0)
                }
                _ => {}
            }
            let label_y = if n.kind == Kind::Subprocess {
                y - h / 2.0 + 21.0
            } else {
                y + 5.0
            };
            text(svg, x, label_y, &n.label, 14.0);
            if n.kind == Kind::Collapsed {
                writeln!(svg,"<rect x=\"{:.1}\" y=\"{:.1}\" width=\"13\" height=\"13\" fill=\"white\" stroke=\"#263548\"/><path d=\"M {:.1} {:.1} v 9 m -4.5 -4.5 h 9\" stroke=\"#263548\"/>",x-6.5,y+h/2.0-15.0,x,y+h/2.0-13.0).unwrap();
            }
        }
        Kind::Start | Kind::End | Kind::Intermediate | Kind::Boundary => {
            let radius = w / 2.0;
            let thick = if n.kind == Kind::End { 3.5 } else { 1.8 };
            let dash = if n.kind == Kind::Boundary && !n.interrupting {
                " stroke-dasharray=\"4 3\""
            } else {
                ""
            };
            writeln!(svg,"<circle class=\"bpmn-body\" cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"{radius:.1}\" fill=\"white\" stroke=\"#263548\" stroke-width=\"{thick:.1}\"{dash}/>").unwrap();
            if matches!(n.kind, Kind::Intermediate | Kind::Boundary) {
                writeln!(svg,"<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"{:.1}\" fill=\"none\" stroke=\"#263548\" stroke-width=\"1.3\"{dash}/>",radius-5.0).unwrap();
            }
            if let Some(marker) = &n.marker {
                event_marker(svg, marker, x, y, n.kind == Kind::End);
            }
            text(svg, x, y + radius + 22.0, &n.label, 12.0);
        }
        Kind::Exclusive | Kind::Parallel | Kind::Inclusive | Kind::EventBased => {
            writeln!(svg,"<path class=\"bpmn-body\" d=\"M {x:.1} {:.1} L {:.1} {y:.1} L {x:.1} {:.1} L {:.1} {y:.1} Z\" fill=\"white\" stroke=\"#263548\" stroke-width=\"2\"/>",y-h/2.0,x+w/2.0,y+h/2.0,x-w/2.0).unwrap();
            match n.kind {
                Kind::Exclusive => text(svg, x, y + 9.0, "X", 28.0),
                Kind::Parallel => text(svg, x, y + 10.0, "+", 32.0),
                Kind::Inclusive => {
                    writeln!(svg,"<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"13\" fill=\"none\" stroke=\"#263548\" stroke-width=\"2.5\"/>").unwrap();
                }
                Kind::EventBased => {
                    writeln!(svg,"<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"16\" fill=\"none\" stroke=\"#263548\" stroke-width=\"1.6\"/><circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"12\" fill=\"none\" stroke=\"#263548\"/><path d=\"M {x:.1} {:.1} l 9 7 -3 10 h -12 l -3 -10 Z\" fill=\"none\" stroke=\"#263548\"/>",y-10.0).unwrap();
                }
                _ => {}
            }
            if !n.id.starts_with("gateway_") {
                text(svg, x, y + h / 2.0 + 22.0, &n.label, 12.0);
            }
        }
        Kind::Data => {
            writeln!(svg,"<path class=\"bpmn-body\" d=\"M {:.1} {:.1} h 49 l 13 13 v 60 h -62 Z M {:.1} {:.1} v 13 h 13\" fill=\"white\" stroke=\"#263548\" stroke-width=\"1.8\"/>",x-31.0,y-39.0,x+18.0,y-39.0).unwrap();
            text(svg, x, y + 57.0, &n.label, 12.0);
        }
        Kind::Store => {
            writeln!(svg,"<path class=\"bpmn-body\" d=\"M {:.1} {:.1} v 63 a 31 8 0 0 0 62 0 v -63 M {:.1} {:.1} a 31 8 0 0 0 62 0 a 31 8 0 0 0 -62 0\" fill=\"white\" stroke=\"#263548\" stroke-width=\"1.8\"/>",x-31.0,y-31.0,x-31.0,y-31.0).unwrap();
            text(svg, x, y + 57.0, &n.label, 12.0);
        }
    }
}
fn edge(svg: &mut String, doc: &Document, geom: &Geometry, e: &super::Edge) {
    let ai = doc.nodes.iter().position(|n| n.id == e.from).unwrap();
    let bi = doc.nodes.iter().position(|n| n.id == e.to).unwrap();
    let (a, b) = (geom.nodes[ai], geom.nodes[bi]);
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let horizontal = dx.abs() > dy.abs() * 0.75 && e.kind != EdgeKind::Message;
    let (x1, y1, x2, y2) = if horizontal {
        let sign = dx.signum();
        (
            a.x + sign * a.w / 2.0,
            a.y,
            b.x - sign * b.w / 2.0 - sign * 4.0,
            b.y,
        )
    } else {
        let sign = dy.signum();
        (
            a.x,
            a.y + sign * a.h / 2.0,
            b.x,
            b.y - sign * b.h / 2.0 - sign * 4.0,
        )
    };
    let (dash, marker) = match e.kind {
        EdgeKind::Sequence => ("", "solid-arrow"),
        EdgeKind::Message => ("stroke-dasharray=\"8 5\"", "open-arrow"),
        EdgeKind::Data => ("stroke-dasharray=\"2 5\"", "open-arrow"),
    };
    let (mid_x, mid_y) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
    let blocked = horizontal
        && (y1 - y2).abs() < 2.0
        && doc.nodes.iter().enumerate().any(|(i, n)| {
            if i == ai
                || i == bi
                || n.parent != doc.nodes[ai].parent
                || n.pool != doc.nodes[ai].pool
            {
                return false;
            }
            let q = geom.nodes[i];
            q.x + q.w / 2.0 > x1.min(x2)
                && q.x - q.w / 2.0 < x1.max(x2)
                && q.y - q.h / 2.0 < y1
                && q.y + q.h / 2.0 > y1
        });
    let channel_clear = |y: f64| {
        !doc.nodes.iter().enumerate().any(|(i, n)| {
            if i == ai
                || i == bi
                || n.parent != doc.nodes[ai].parent
                || n.pool != doc.nodes[ai].pool
            {
                return false;
            }
            let q = geom.nodes[i];
            q.x + q.w / 2.0 > x1.min(x2)
                && q.x - q.w / 2.0 < x1.max(x2)
                && q.y - q.h / 2.0 - 8.0 < y
                && q.y + q.h / 2.0 + 8.0 > y
        })
    };
    let mut corridor = y1.max(y2) + a.h.max(b.h) / 2.0 + 55.0;
    if !channel_clear(corridor) {
        corridor = y1.min(y2) - a.h.max(b.h) / 2.0 - 55.0;
        while !channel_clear(corridor) && corridor > 35.0 {
            corridor -= 45.0;
        }
    }
    let path = if blocked {
        format!(
            "M {x1:.1} {y1:.1} L {:.1} {y1:.1} L {:.1} {corridor:.1} L {:.1} {corridor:.1} L {:.1} {y2:.1} L {x2:.1} {y2:.1}",
            x1 + 15.0,
            x1 + 15.0,
            x2 - 15.0,
            x2 - 15.0
        )
    } else if horizontal {
        format!("M {x1:.1} {y1:.1} L {mid_x:.1} {y1:.1} L {mid_x:.1} {y2:.1} L {x2:.1} {y2:.1}")
    } else {
        format!("M {x1:.1} {y1:.1} L {x1:.1} {mid_y:.1} L {x2:.1} {mid_y:.1} L {x2:.1} {y2:.1}")
    };
    writeln!(svg,"<path d=\"{path}\" fill=\"none\" stroke=\"#263548\" stroke-width=\"1.7\" {dash} marker-end=\"url(#{marker})\"/>").unwrap();
    if e.kind == EdgeKind::Message {
        writeln!(
            svg,
            "<circle cx=\"{x1:.1}\" cy=\"{y1:.1}\" r=\"4\" fill=\"white\" stroke=\"#263548\"/>"
        )
        .unwrap();
    }
    if let Some(label) = &e.label {
        text(
            svg,
            mid_x,
            if blocked { corridor - 9.0 } else { mid_y - 9.0 },
            label,
            12.0,
        );
    }
}
pub fn render(doc: &Document, mode: TextMode) -> Result<String, String> {
    render_with_styles(doc, &StyleSheet::default(), mode)
}
pub fn render_with_styles(
    doc: &Document,
    sheet: &StyleSheet,
    mode: TextMode,
) -> Result<String, String> {
    super::style::validate_styles(doc, sheet)?;
    let geom = place(doc)?;
    let (width, height) = (geom.width, geom.height);
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width:.1}\" height=\"{height:.1}\" viewBox=\"0 0 {width:.1} {height:.1}\">\n"
    );
    if mode == TextMode::Embedded {
        let font = base64::engine::general_purpose::STANDARD.encode(FONT);
        writeln!(svg,"<style>@font-face {{ font-family: 'Inter'; src: url('data:font/otf;base64,{font}') format('opentype'); }}</style>").unwrap();
    }
    svg.push_str("<defs><marker id=\"solid-arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\"><path d=\"M 1 1 L 9 5 L 1 9 Z\" fill=\"#263548\"/></marker><marker id=\"open-arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\"><path d=\"M 1 1 L 9 5 L 1 9\" fill=\"none\" stroke=\"#263548\"/></marker></defs>\n");
    if doc.nodes.iter().any(|n| {
        super::style::icon(sheet, n).is_some()
            || n.marker.as_deref() == Some("timer")
            || matches!(
                n.kind,
                Kind::UserTask | Kind::ServiceTask | Kind::ScriptTask | Kind::ManualTask
            )
    }) {
        svg.push_str(crate::export_metadata::MARKER);
        svg.push_str(&escape(crate::export_metadata::PHOSPHOR_LICENSE));
        svg.push_str("</metadata>\n");
    }
    for n in &doc.nodes {
        let fill = super::style::value(sheet, n, "fill");
        let border = super::style::value(sheet, n, "border-color");
        if fill.is_some() || border.is_some() {
            write!(
                svg,
                "<style>[data-bpmn-id=\"{}\"] > .bpmn-body {{",
                escape(&n.id)
            )
            .unwrap();
            if let Some(Value::Color(c)) = fill {
                write!(svg, "fill:{c};").unwrap();
            }
            if let Some(Value::Color(c)) = border {
                write!(svg, "stroke:{c};").unwrap();
            }
            svg.push_str("}</style>\n");
        }
    }
    for p in &geom.pools {
        writeln!(svg,"<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"#f8fbff\" stroke=\"#263548\" stroke-width=\"2\"/>",p.x,p.y,p.w,p.h).unwrap();
        if doc.direction == crate::syntax::FlowDirection::TopToBottom {
            writeln!(
                svg,
                "<path d=\"M {:.1} {:.1} H {:.1}\" stroke=\"#263548\"/>",
                p.x,
                p.y + 30.0,
                p.x + p.w
            )
            .unwrap();
        }
        for band in &p.bands {
            if doc.direction == crate::syntax::FlowDirection::TopToBottom {
                writeln!(
                    svg,
                    "<path d=\"M {:.1} {:.1} V {:.1}\" stroke=\"#263548\" fill=\"none\"/>",
                    band.y,
                    p.y,
                    p.y + p.h
                )
                .unwrap();
                text(
                    &mut svg,
                    band.y + band.h / 2.0,
                    p.y + 54.0,
                    &band.label,
                    13.0,
                );
            } else {
                writeln!(svg,"<path d=\"M {:.1} {:.1} H {:.1}\" stroke=\"#263548\" fill=\"none\"/><text x=\"{:.1}\" y=\"{:.1}\" transform=\"rotate(-90 {:.1} {:.1})\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"13\">{}</text>",p.x,band.y,p.x+p.w,p.x+62.0,band.y+band.h/2.0,p.x+62.0,band.y+band.h/2.0,escape(&band.label)).unwrap();
            }
        }
        writeln!(
            svg,
            "<text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Inter\" font-size=\"13\">{}</text>",
            p.x + 10.0,
            p.y + 20.0,
            escape(&p.label)
        )
        .unwrap();
    }
    // Expanded subprocesses are containers: paint their background below internal flows.
    for (n, &b) in doc
        .nodes
        .iter()
        .zip(&geom.nodes)
        .filter(|(n, _)| n.kind == Kind::Subprocess)
    {
        writeln!(svg, "<g data-bpmn-id=\"{}\">", escape(&n.id)).unwrap();
        shape(&mut svg, n, b, super::style::icon(sheet, n));
        svg.push_str("</g>\n");
    }
    for e in &doc.edges {
        edge(&mut svg, doc, &geom, e);
    }
    for (n, &b) in doc
        .nodes
        .iter()
        .zip(&geom.nodes)
        .filter(|(n, _)| n.kind != Kind::Subprocess)
    {
        writeln!(svg, "<g data-bpmn-id=\"{}\">", escape(&n.id)).unwrap();
        shape(&mut svg, n, b, super::style::icon(sheet, n));
        svg.push_str("</g>\n");
    }
    svg.push_str("</svg>\n");
    if mode == TextMode::Paths {
        crate::export_metadata::preserve_svg_notice(&svg, crate::export::outlines(&svg)?)
    } else {
        Ok(svg)
    }
}
