//! BPMN SVG renderer: participant bands, independent flow kinds and shape markers.
use super::geometry::{BoxAt, Geometry, place};
mod edges;
mod labels;
mod orthogonal;
mod plan;
mod ports;
#[cfg(test)]
mod routing_tests;
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
fn shape(svg: &mut String, n: &Node, b: BoxAt, icon: Option<&str>, caption_above: bool) {
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
                    x + w / 2.0 - 28.0
                } else {
                    x - w / 2.0 + 10.0
                };
                phosphor(svg, icon, icon_x, y - h / 2.0 + 9.0, 18.0);
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
            text(svg, x, label_y, &n.label, super::type_sizes::TASK);
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
            if n.kind != Kind::Boundary {
                let caption_y = if caption_above {
                    y - radius - 12.0
                } else {
                    y + radius + 22.0
                };
                text(svg, x, caption_y, &n.label, super::type_sizes::CAPTION);
            }
        }
        Kind::Exclusive | Kind::Parallel | Kind::Inclusive | Kind::EventBased => {
            writeln!(svg,"<path class=\"bpmn-body\" d=\"M {x:.1} {:.1} L {:.1} {y:.1} L {x:.1} {:.1} L {:.1} {y:.1} Z\" fill=\"white\" stroke=\"#263548\" stroke-width=\"2\"/>",y-h/2.0,x+w/2.0,y+h/2.0,x-w/2.0).unwrap();
            match n.kind {
                Kind::Exclusive => {
                    writeln!(svg, "<path class=\"bpmn-gateway-marker\" d=\"M {:.1} {:.1} L {:.1} {:.1} M {:.1} {:.1} L {:.1} {:.1}\" fill=\"none\" stroke=\"#182330\" stroke-width=\"4\" stroke-linecap=\"square\"/>",
                        x - 12.0, y - 12.0, x + 12.0, y + 12.0,
                        x + 12.0, y - 12.0, x - 12.0, y + 12.0).unwrap();
                }
                Kind::Parallel => {
                    writeln!(svg, "<path class=\"bpmn-gateway-marker\" d=\"M {x:.1} {:.1} V {:.1} M {:.1} {y:.1} H {:.1}\" fill=\"none\" stroke=\"#182330\" stroke-width=\"4\" stroke-linecap=\"square\"/>",
                        y - 15.0, y + 15.0, x - 15.0, x + 15.0).unwrap();
                }
                Kind::Inclusive => {
                    writeln!(svg,"<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"13\" fill=\"none\" stroke=\"#263548\" stroke-width=\"2.5\"/>").unwrap();
                }
                Kind::EventBased => {
                    writeln!(svg,"<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"16\" fill=\"none\" stroke=\"#263548\" stroke-width=\"1.6\"/><circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"12\" fill=\"none\" stroke=\"#263548\"/><path d=\"M {x:.1} {:.1} l 9 7 -3 10 h -12 l -3 -10 Z\" fill=\"none\" stroke=\"#263548\"/>",y-10.0).unwrap();
                }
                _ => {}
            }
            if !n.id.starts_with("gateway_") {
                text(
                    svg,
                    x,
                    y + h / 2.0 + 22.0,
                    &n.label,
                    super::type_sizes::CAPTION,
                );
            }
        }
        Kind::Annotation => {
            writeln!(svg,"<path class=\"bpmn-body\" d=\"M {:.1} {:.1} H {:.1} V {:.1} H {:.1}\" fill=\"none\" stroke=\"#263548\" stroke-width=\"1.6\"/>",x-w/2.0+15.0,y-h/2.0+6.0,x-w/2.0,y+h/2.0-6.0,x-w/2.0+15.0).unwrap();
            let lines = n.label.split('\n').count();
            text(
                svg,
                x + 8.0,
                y + 5.0 - (lines - 1) as f64 * 9.0,
                &n.label,
                super::type_sizes::NOTE,
            );
        }
        Kind::Data => {
            writeln!(svg,"<path class=\"bpmn-body\" d=\"M {:.1} {:.1} h 49 l 13 13 v 60 h -62 Z M {:.1} {:.1} v 13 h 13\" fill=\"white\" stroke=\"#263548\" stroke-width=\"1.8\"/>",x-31.0,y-39.0,x+18.0,y-39.0).unwrap();
            text(svg, x, y + 57.0, &n.label, super::type_sizes::CAPTION);
        }
        Kind::Store => {
            writeln!(svg,"<path class=\"bpmn-body\" d=\"M {:.1} {:.1} v 63 a 31 8 0 0 0 62 0 v -63 M {:.1} {:.1} a 31 8 0 0 0 62 0 a 31 8 0 0 0 -62 0\" fill=\"white\" stroke=\"#263548\" stroke-width=\"1.8\"/>",x-31.0,y-31.0,x-31.0,y-31.0).unwrap();
            text(svg, x, y + 57.0, &n.label, super::type_sizes::CAPTION);
        }
    }
}
pub(super) fn validate_attachments(doc: &Document, geom: &Geometry) -> Result<(), String> {
    let ports = ports::assign(doc, geom)?;
    let (mut sink, routes) = plan::all(doc, geom, &ports)?;
    labels::render(&mut sink, doc, geom, &routes)?;
    Ok(())
}
pub fn render(doc: &Document, mode: TextMode) -> Result<String, String> {
    render_with_styles(doc, &StyleSheet::default(), mode)
}
pub fn render_with_styles(
    doc: &Document,
    sheet: &StyleSheet,
    mode: TextMode,
) -> Result<String, String> {
    let mut effective = sheet.clone();
    effective.theme = doc.theme;
    let sheet = &effective;
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
        let base = match super::style::value(sheet, n, "base-color") {
            Some(Value::Color(c)) => Some(c.as_str()),
            _ => None,
        };
        let fallback = if sheet.theme.modern() || base.is_some() {
            let seed = base.unwrap_or(match n.kind {
                Kind::Annotation
                | Kind::Exclusive
                | Kind::Inclusive
                | Kind::Parallel
                | Kind::EventBased => "#d99520",
                Kind::Data | Kind::Store => "#8394a7",
                _ => sheet.theme.default_base(crate::syntax::NodeKind::Generic),
            });
            Some(sheet.theme.colors(seed))
        } else {
            None
        };
        let fill = match super::style::value(sheet, n, "fill") {
            Some(Value::Color(c)) => Some(c.as_str()),
            _ => fallback.as_ref().map(|p| p.fill.as_str()),
        };
        let border = match super::style::value(sheet, n, "border-color") {
            Some(Value::Color(c)) => Some(c.as_str()),
            _ => fallback.as_ref().map(|p| p.border.as_str()),
        };
        let text_color = match super::style::value(sheet, n, "color") {
            Some(Value::Color(c)) => Some(c.as_str()),
            _ => fallback.as_ref().map(|p| p.text.as_str()),
        };
        if fill.is_some() || border.is_some() || text_color.is_some() {
            write!(
                svg,
                "<style>[data-bpmn-id=\"{}\"] > .bpmn-body {{",
                escape(&n.id)
            )
            .unwrap();
            // Annotation is an open bracket; filling it changes its notation.
            if n.kind != Kind::Annotation
                && let Some(c) = fill
            {
                write!(svg, "fill:{c};").unwrap();
            }
            if let Some(c) = border {
                write!(svg, "stroke:{c};").unwrap();
            }
            svg.push_str("}</style>\n");
            if let Some(c) = text_color {
                writeln!(
                    svg,
                    "<style>[data-bpmn-id=\"{}\"] text {{fill:{c};}} [data-bpmn-id=\"{}\"] .bpmn-gateway-marker {{stroke:{c};}}</style>",
                    escape(&n.id),
                    escape(&n.id)
                )
                .unwrap();
            }
        }
    }
    for p in &geom.pools {
        writeln!(svg,"<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"#f8fbff\" stroke=\"#263548\" stroke-width=\"2\"/>",p.x,p.y,p.w,p.h).unwrap();
        writeln!(
            svg,
            "<path d=\"M {:.1} {:.1} H {:.1}\" stroke=\"#263548\"/>",
            p.x,
            p.y + 30.0,
            p.x + p.w
        )
        .unwrap();
        for (band_index, band) in p.bands.iter().enumerate() {
            if !p.bands.iter().any(|b| b.id.is_some()) {
                continue;
            }
            let last = band_index + 1 == p.bands.len();
            let span = if last {
                if doc.direction == crate::syntax::FlowDirection::TopToBottom {
                    p.x + p.w - band.y
                } else {
                    p.y + p.h - band.y
                }
            } else {
                band.h
            };
            if doc.direction == crate::syntax::FlowDirection::TopToBottom {
                writeln!(svg,"<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"30\" fill=\"#f8fbff\"/><path d=\"M {:.1} {:.1} H {:.1}\" stroke=\"#263548\"/>",band.y,p.y+30.0,span,band.y,p.y+60.0,band.y+span).unwrap();
                writeln!(
                    svg,
                    "<path d=\"M {:.1} {:.1} V {:.1}\" stroke=\"#263548\" fill=\"none\"/>",
                    band.y,
                    p.y + 30.0,
                    p.y + p.h
                )
                .unwrap();
                text(
                    &mut svg,
                    band.y + band.h / 2.0,
                    p.y + 50.0,
                    &band.label,
                    super::type_sizes::BAND,
                );
            } else {
                writeln!(svg,"<rect x=\"{:.1}\" y=\"{:.1}\" width=\"30\" height=\"{:.1}\" fill=\"#f8fbff\"/><path d=\"M {:.1} {:.1} V {:.1}\" stroke=\"#263548\" fill=\"none\"/><text x=\"{:.1}\" y=\"{:.1}\" transform=\"rotate(-90 {:.1} {:.1})\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"15\">{}</text>",p.x,band.y,span,p.x+30.0,band.y,band.y+span,p.x+15.0,band.y+band.h/2.0,p.x+15.0,band.y+band.h/2.0,escape(&band.label)).unwrap();
                if band.y > p.y + 30.5 {
                    writeln!(
                        svg,
                        "<path d=\"M {:.1} {:.1} H {:.1}\" stroke=\"#263548\"/>",
                        p.x,
                        band.y,
                        p.x + p.w
                    )
                    .unwrap();
                }
            }
        }
        // Bands reach the pool's outer edge; repaint the outer stroke last so
        // their fills cannot erase the final border pixels.
        writeln!(svg,"<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"#263548\" stroke-width=\"2\"/>",p.x,p.y,p.w,p.h).unwrap();
        writeln!(
            svg,
            "<text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Inter\" font-size=\"15\">{}</text>",
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
        shape(&mut svg, n, b, super::style::icon(sheet, n), false);
        svg.push_str("</g>\n");
    }
    let ports = ports::assign(doc, &geom)?;
    let (paths, routes) = plan::all(doc, &geom, &ports)?;
    svg.push_str(&paths);
    for (n, &b) in doc
        .nodes
        .iter()
        .zip(&geom.nodes)
        .filter(|(n, _)| n.kind != Kind::Subprocess)
    {
        writeln!(svg, "<g data-bpmn-id=\"{}\">", escape(&n.id)).unwrap();
        let caption_above = doc.direction == crate::syntax::FlowDirection::TopToBottom
            && n.kind == Kind::Start
            && doc
                .edges
                .iter()
                .any(|edge| edge.kind == super::EdgeKind::Sequence && edge.from == n.id);
        shape(&mut svg, n, b, super::style::icon(sheet, n), caption_above);
        svg.push_str("</g>\n");
    }
    labels::render(&mut svg, doc, &geom, &routes)?;
    svg.push_str("</svg>\n");
    if mode == TextMode::Paths {
        crate::export_metadata::preserve_svg_notice(&svg, crate::export::outlines(&svg)?)
    } else {
        Ok(svg)
    }
}
