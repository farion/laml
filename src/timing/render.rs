//! Numeric horizontal time axis and checked discrete state-invariant timelines.
use super::{Document, Lifeline};
use crate::styles::{Element, StyleSheet, Value};
use crate::svg::{self, TextMode};
use crate::text::measure;
use std::fmt::Write;

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn number(sheet: &StyleSheet, element: &Element<'_>, name: &str, default: f64) -> f64 {
    match sheet.get(element, name).map(|d| &d.value) {
        Some(Value::Number(n)) => *n,
        _ => default,
    }
}
fn color(sheet: &StyleSheet, element: &Element<'_>, name: &str, default: &str) -> String {
    match sheet.get(element, name).map(|d| &d.value) {
        Some(Value::Color(s)) => s.clone(),
        _ => default.into(),
    }
}
#[derive(Clone)]
struct LaneStyle {
    font: f64,
    color: String,
    stroke: String,
    width: f64,
}
fn lane_style(sheet: &StyleSheet, id: &str) -> Result<LaneStyle, String> {
    let node = Element::Node { id, classes: &[] };
    for property in [
        "display",
        "shape",
        "icon",
        "symbol-rendering",
        "width",
        "height",
        "min-width",
        "min-height",
        "padding",
        "fill",
        "corner-radius",
    ] {
        if sheet.get(&node, property).is_some() {
            return Err(format!(
                "timing lifeline '{id}' does not support '{property}' styling"
            ));
        }
    }
    let font = number(sheet, &node, "font-size", 14.0);
    let width = number(sheet, &node, "border-width", 1.6);
    if !(10.0..=20.0).contains(&font) || !(1.0..=4.0).contains(&width) {
        return Err(format!(
            "timing lifeline '{id}' needs font-size 10..20 and border-width 1..4"
        ));
    }
    let stroke = color(sheet, &node, "border-color", "#395373");
    let ink = color(sheet, &node, "color", "#17212e");
    if stroke == "none" || ink == "none" {
        return Err(format!(
            "timing lifeline '{id}' and its state timeline must remain visible"
        ));
    }
    Ok(LaneStyle {
        font,
        color: ink,
        stroke,
        width,
    })
}
fn state_names(lane: &Lifeline) -> Vec<&str> {
    let mut names = vec![lane.initial.as_str()];
    for change in &lane.changes {
        if !names.contains(&change.state.as_str()) {
            names.push(&change.state);
        }
    }
    names
}
fn state_y(base: f64, names: &[&str], state: &str) -> f64 {
    base + (names.len()
        - 1
        - names
            .iter()
            .position(|name| *name == state)
            .expect("validated state")) as f64
        * 38.0
}
fn validate_ticks(doc: &Document, x0: f64) -> Result<(), String> {
    let mut previous = None;
    for tick in &doc.ticks {
        let center = x0 + tick.time * doc.scale;
        let width = measure(&tick.label, 12.0)? + 6.0;
        let start = center - width / 2.0;
        if let Some(last) = previous
            && start < last + 4.0
        {
            return Err(format!(
                "timing ruler tick at line {} overlaps another label; increase scale",
                tick.line
            ));
        }
        previous = Some(center + width / 2.0);
    }
    Ok(())
}

pub fn render(doc: &Document, sheet: &StyleSheet, mode: TextMode) -> Result<String, String> {
    let styles = doc
        .lifelines
        .iter()
        .map(|p| lane_style(sheet, &p.id))
        .collect::<Result<Vec<_>, _>>()?;
    let names = doc.lifelines.iter().map(state_names).collect::<Vec<_>>();
    let mut max_label = 0.0_f64;
    for (lane, (style, states)) in doc.lifelines.iter().zip(styles.iter().zip(&names)) {
        for state in states {
            max_label = max_label.max(measure(state, style.font)?);
        }
        if lane.events.len() > 32 {
            return Err("too many event labels on one timing lifeline".into());
        }
    }
    let left = 24.0;
    let x0 = left + 62.0 + max_label + 16.0;
    let end_x = x0 + doc.end * doc.scale;
    validate_ticks(doc, x0)?;
    let base_mode = if mode == TextMode::Paths {
        TextMode::Embedded
    } else {
        mode
    };
    let mut svg = svg::render_with_mode(&[], &[], base_mode)?;
    let mut drawing = String::new();
    let mut top = 64.0;
    let mut panel_bottom = top;
    for ((lane, style), states) in doc.lifelines.iter().zip(&styles).zip(&names) {
        let caption = lane
            .classifier
            .as_ref()
            .map_or(lane.id.clone(), |ty| format!("{}:{ty}", lane.id));
        let height = (70.0
            + (states.len() - 1) as f64 * 38.0
            + if lane.events.is_empty() { 0.0 } else { 24.0 })
        .max(measure(&caption, style.font)? + 36.0);
        let base = top + 28.0;
        let center = top + height / 2.0;
        writeln!(drawing,"<text x=\"{:.2}\" y=\"{center:.2}\" text-anchor=\"middle\" transform=\"rotate(-90 {:.2} {center:.2})\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\">{}</text>",
            left+22.0,left+22.0,style.font,style.color,escape(&caption)).unwrap();
        for state in states {
            let y = state_y(base, states, state);
            writeln!(drawing,"<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"end\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\">{}</text>",
                x0-16.0,y+style.font*0.34,style.font,style.color,escape(state)).unwrap();
        }
        let mut state = &lane.initial[..];
        let mut previous = x0;
        let mut points = format!("{x0:.2},{:.2}", state_y(base, states, state));
        for change in &lane.changes {
            let x = x0 + change.time * doc.scale;
            if x - previous < 12.0 {
                return Err(format!(
                    "state changes too close on lifeline '{}' at line {}; increase scale",
                    lane.id, change.line
                ));
            }
            write!(
                points,
                " {x:.2},{:.2} {x:.2},{:.2}",
                state_y(base, states, state),
                state_y(base, states, &change.state)
            )
            .unwrap();
            state = &change.state;
            previous = x;
        }
        write!(points, " {end_x:.2},{:.2}", state_y(base, states, state)).unwrap();
        writeln!(drawing,"<polyline points=\"{points}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-linejoin=\"round\" />",style.stroke,style.width).unwrap();
        let event_y = base + (states.len() - 1) as f64 * 38.0 + 34.0;
        let mut labels = Vec::new();
        for event in &lane.events {
            let x = x0 + event.time * doc.scale;
            let width = measure(&event.label, style.font)? + 8.0;
            let interval = (x - width / 2.0, x + width / 2.0);
            if interval.0 < x0 - 4.0
                || interval.1 > end_x - 4.0
                || labels
                    .iter()
                    .any(|(a, b)| interval.0 < *b + 5.0 && interval.1 > *a - 5.0)
            {
                return Err(format!(
                    "event label at line {} overlaps another label or the timeline boundary; increase scale or end",
                    event.line
                ));
            }
            labels.push(interval);
            writeln!(drawing,"<path d=\"M{x:.2},{:.2} v8\" stroke=\"{}\" stroke-width=\"1\" /><text x=\"{x:.2}\" y=\"{event_y:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\">{}</text>",
                event_y-20.0,style.stroke,style.font,style.color,escape(&event.label)).unwrap();
        }
        panel_bottom = top + height;
        writeln!(drawing,"<path d=\"M{:.2},{panel_bottom:.2} H{:.2}\" fill=\"none\" stroke=\"#c8d2df\" stroke-width=\"1\" />",left+4.0,end_x+14.0).unwrap();
        top = panel_bottom + doc.lane_gap;
    }
    let axis_y = panel_bottom + 36.0;
    writeln!(
        drawing,
        "<path d=\"M{x0:.2},{axis_y:.2} H{end_x:.2}\" fill=\"none\" stroke=\"#395373\" />"
    )
    .unwrap();
    for tick in &doc.ticks {
        let x = x0 + tick.time * doc.scale;
        writeln!(drawing,"<path d=\"M{x:.2},{axis_y:.2} v8\" stroke=\"#395373\" /><text x=\"{x:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"12\" fill=\"#17212e\">{}</text>",axis_y+23.0,escape(&tick.label)).unwrap();
    }
    let axis_title = doc
        .unit
        .as_ref()
        .map_or("time".to_string(), |unit| format!("time ({unit})"));
    writeln!(drawing,"<text x=\"{x0:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"12\" fill=\"#17212e\">{}</text>",axis_y+44.0,escape(&axis_title)).unwrap();
    let title = format!("sd {}", doc.interaction);
    let right = (end_x + 28.0).max(left + measure(&title, 13.0)? + 24.0);
    let bottom = axis_y + 58.0;
    let frame = format!(
        "<g data-interaction=\"{}\"><rect x=\"{left:.2}\" y=\"24\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"#395373\" /><text x=\"{:.2}\" y=\"43\" font-family=\"Inter\" font-size=\"13\" fill=\"#17212e\">{}</text></g>\n",
        escape(&doc.interaction),
        right - left,
        bottom - 24.0,
        left + 9.0,
        escape(&title)
    );
    let after = svg.find("</defs>\n").ok_or("SVG missing definitions")? + "</defs>\n".len();
    svg.insert_str(after, &frame);
    let end = svg.rfind("</svg>").ok_or("SVG missing closing tag")?;
    svg.insert_str(end, &drawing);
    let opening = svg.find('>').ok_or("SVG missing opening tag")? + 1;
    svg.replace_range(..opening,&format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.2}\" height=\"{:.2}\" viewBox=\"0 0 {:.2} {:.2}\">",right+24.0,bottom+24.0,right+24.0,bottom+24.0));
    if mode == TextMode::Paths {
        crate::export::outlines(&svg)
    } else {
        Ok(svg)
    }
}
