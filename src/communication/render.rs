//! Message arrows lie on the declared links; labels occupy checked free lanes.
use super::{Document, Message};
use crate::geometry::BoxNode;
use crate::route::{Point, Route};
use crate::svg::{self, TextMode};
use crate::text::measure;
use std::fmt::Write;

#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}
impl Rect {
    fn meets(self, other: Self) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }
}
fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn link<'a>(
    doc: &Document,
    boxes: &[BoxNode],
    routes: &'a [Route],
    msg: &Message,
) -> Result<&'a Route, String> {
    let index = doc
        .graph
        .relations
        .iter()
        .position(|rel| {
            (rel.from == msg.from && rel.to == msg.to) || (rel.from == msg.to && rel.to == msg.from)
        })
        .ok_or("missing checked message link")?;
    let route = routes
        .get(index)
        .ok_or("missing routed communication link")?;
    if boxes.get(route.from).is_none() || boxes.get(route.to).is_none() {
        return Err("communication route refers to unknown lifeline".into());
    }
    Ok(route)
}
fn covered(rect: Rect, boxes: &[BoxNode], used: &[Rect], routes: &[Route], own: usize) -> bool {
    if boxes.iter().any(|node| {
        rect.meets(Rect {
            x: node.x - 3.0,
            y: node.y - 3.0,
            w: node.width + 6.0,
            h: node.height + 6.0,
        })
    }) || used.iter().any(|prior| rect.meets(*prior))
    {
        return true;
    }
    for (index, route) in routes.iter().enumerate() {
        if index == own {
            continue;
        }
        for pair in route.points.windows(2) {
            let x = pair[0].x.min(pair[1].x) - 2.0;
            let y = pair[0].y.min(pair[1].y) - 2.0;
            let w = (pair[0].x - pair[1].x).abs() + 4.0;
            let h = (pair[0].y - pair[1].y).abs() + 4.0;
            if rect.meets(Rect { x, y, w, h }) {
                return true;
            }
        }
    }
    false
}
struct Placement<'a> {
    used: &'a mut Vec<Rect>,
    bounds: &'a mut (f64, f64, f64, f64),
    out: &'a mut String,
}
fn annotation(
    doc: &Document,
    boxes: &[BoxNode],
    routes: &[Route],
    msg: &Message,
    preferred: f64,
    placement: &mut Placement<'_>,
) -> Result<(), String> {
    let route = link(doc, boxes, routes, msg)?;
    let own = routes
        .iter()
        .position(|candidate| std::ptr::eq(candidate, route))
        .ok_or("unknown link route")?;
    let label = format!(
        "{}: {}",
        msg.number
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join("."),
        msg.label
    );
    let width = measure(&label, 12.0)? + 8.0;
    let mut segments: Vec<_> = route
        .points
        .windows(2)
        .map(|pair| {
            let dx = pair[1].x - pair[0].x;
            let dy = pair[1].y - pair[0].y;
            (dx.hypot(dy), pair[0], pair[1])
        })
        .collect();
    segments.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (length, start, end) in segments {
        if length < 24.0 {
            continue;
        }
        let ux = (end.x - start.x) / length;
        let uy = (end.y - start.y) / length;
        for fraction in [preferred, 0.50, 0.30, 0.70, 0.18, 0.82] {
            let center = Point {
                x: start.x + (end.x - start.x) * fraction,
                y: start.y + (end.y - start.y) * fraction,
            };
            for distance in [16.0, 34.0, 52.0, 70.0] {
                for sign in [-1.0, 1.0] {
                    if ux.abs() >= uy.abs() && sign > 0.0 && distance < 34.0 {
                        continue;
                    }
                    let offset = distance * sign;
                    let rect = if ux.abs() >= uy.abs() {
                        Rect {
                            x: center.x - width / 2.0,
                            y: center.y + offset - 14.0,
                            w: width,
                            h: 17.0,
                        }
                    } else {
                        Rect {
                            x: center.x + offset - (if sign < 0.0 { width } else { 0.0 }),
                            y: center.y - 8.0,
                            w: width,
                            h: 17.0,
                        }
                    };
                    if covered(rect, boxes, placement.used, routes, own) {
                        continue;
                    }
                    let forward = boxes[route.from].id == msg.from;
                    let direction = if forward { 1.0 } else { -1.0 };
                    // Keep the small message arrow beside, not on top of, the link:
                    // an open chevron over the link reads falsely as a UML diamond.
                    let (nx, ny) = if ux.abs() >= uy.abs() {
                        (0.0, sign)
                    } else {
                        (sign, 0.0)
                    };
                    let tip = Point {
                        x: center.x + nx * 7.0 + ux * 7.0 * direction,
                        y: center.y + ny * 7.0 + uy * 7.0 * direction,
                    };
                    let base = Point {
                        x: tip.x - ux * 9.0 * direction,
                        y: tip.y - uy * 9.0 * direction,
                    };
                    let start = Point {
                        x: tip.x - ux * 14.0 * direction,
                        y: tip.y - uy * 14.0 * direction,
                    };
                    writeln!(placement.out,"<path d=\"M{:.2},{:.2} L{:.2},{:.2} M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}\" fill=\"none\" stroke=\"#395373\" stroke-width=\"1.5\" />",
                        start.x,start.y,tip.x,tip.y,base.x+nx*4.0,base.y+ny*4.0,tip.x,tip.y,base.x-nx*4.0,base.y-ny*4.0).unwrap();
                    writeln!(placement.out,"<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"12\" fill=\"#17212e\">{}</text>",
                        rect.x+width/2.0,rect.y+13.0,xml(&label)).unwrap();
                    placement.bounds.0 = placement.bounds.0.min(rect.x);
                    placement.bounds.1 = placement.bounds.1.min(rect.y);
                    placement.bounds.2 = placement.bounds.2.max(rect.x + rect.w);
                    placement.bounds.3 = placement.bounds.3.max(rect.y + rect.h);
                    placement.used.push(rect);
                    return Ok(());
                }
            }
        }
    }
    Err(format!(
        "line {}: cannot place numbered message '{}' without covering a lifeline, link or label; increase the link gap",
        msg.line, label
    ))
}

pub fn render(
    doc: &Document,
    boxes: &[BoxNode],
    routes: &[Route],
    mode: TextMode,
) -> Result<String, String> {
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
    for node in boxes {
        bounds.0 = bounds.0.min(node.x);
        bounds.1 = bounds.1.min(node.y);
        bounds.2 = bounds.2.max(node.x + node.width);
        bounds.3 = bounds.3.max(node.y + node.height);
    }
    for route in routes {
        for point in &route.points {
            bounds.0 = bounds.0.min(point.x);
            bounds.1 = bounds.1.min(point.y);
            bounds.2 = bounds.2.max(point.x);
            bounds.3 = bounds.3.max(point.y);
        }
    }
    let mut annotations = String::new();
    let mut used = Vec::new();
    for (index, message) in doc.messages.iter().enumerate() {
        let same = |other: &&Message| {
            (other.from == message.from && other.to == message.to)
                || (other.from == message.to && other.to == message.from)
        };
        let total = doc.messages.iter().filter(same).count();
        let rank = doc.messages[..index].iter().filter(same).count();
        let preferred = (rank + 1) as f64 / (total + 1) as f64;
        annotation(
            doc,
            boxes,
            routes,
            message,
            preferred,
            &mut Placement {
                used: &mut used,
                bounds: &mut bounds,
                out: &mut annotations,
            },
        )?;
    }
    let (left, top, right, bottom) = (
        bounds.0 - 20.0,
        bounds.1 - 28.0,
        bounds.2 + 20.0,
        bounds.3 + 20.0,
    );
    let frame = format!(
        "<g data-interaction=\"{}\"><rect x=\"{left:.2}\" y=\"{top:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"#395373\" /><text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"13\" fill=\"#17212e\">sd {}</text></g>\n",
        xml(&doc.interaction),
        right - left,
        bottom - top,
        left + 9.0,
        top + 16.0,
        xml(&doc.interaction)
    );
    let after_def = svg
        .find("</defs>\n")
        .ok_or("SVG missing definition boundary")?
        + "</defs>\n".len();
    svg.insert_str(after_def, &frame);
    let end = svg.rfind("</svg>").ok_or("SVG missing closing tag")?;
    svg.insert_str(end, &annotations);
    let canvas = (left - 32.0, top - 32.0, right + 32.0, bottom + 32.0);
    let opening = svg.find('>').ok_or("SVG missing opening tag")? + 1;
    svg.replace_range(..opening,&format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.2}\" height=\"{:.2}\" viewBox=\"{:.2} {:.2} {:.2} {:.2}\">",
        canvas.2-canvas.0,canvas.3-canvas.1,canvas.0,canvas.1,canvas.2-canvas.0,canvas.3-canvas.1));
    if mode == TextMode::Paths {
        crate::export::outlines(&svg)
    } else {
        Ok(svg)
    }
}
