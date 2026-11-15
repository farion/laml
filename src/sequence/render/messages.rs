//! Sequence message strokes, labels, arrows, unknown ends and gates.
use super::styles::escape;
use super::{
    Message, MessageKind, anchors::MessageAnchor, arrows, geometry::label_width_at, styled_text,
    styles,
};
use crate::sequence::messages::gate_id;
use crate::styles::StyleSheet;
use std::fmt::Write;

pub(super) fn draw(
    svg: &mut String,
    sheet: &StyleSheet,
    m: &Message,
    anchor: &MessageAnchor,
    drawn_gates: &mut Vec<(String, f64, f64)>,
    allow_diagonal: bool,
) -> Result<(), String> {
    let (start, y) = anchor.send;
    let (end, receive_y) = anchor.receive;
    let paint = styles::message(sheet, m.name.as_deref(), &m.classes);
    let name = escape(m.name.as_deref().unwrap_or(""));
    let dash = if super::message_geometry::dashed(m.kind) { " stroke-dasharray=\"7 5\"" } else { "" };
    let points = super::message_geometry::path(anchor.send, anchor.receive, m.from == m.to, allow_diagonal);
    let path = super::message_geometry::svg_path(&points, m.from == m.to);
    writeln!(svg, "<path d=\"{path}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\"{dash} data-message=\"{name}\" />", paint.color, paint.width).unwrap();
    if m.to != "?" && m.kind != MessageKind::Delete {
        let previous = points[points.len() - 2];
        arrows::draw(svg, anchor.receive, (end - previous.0, receive_y - previous.1),
            super::message_geometry::filled(m.kind, m.filled_reply), &paint.color, sheet.theme.modern());
    }
    if m.from == "?" || m.to == "?" {
        let ((at, at_y), side) = if m.from == "?" {
            ((start, y), "source")
        } else {
            ((end, receive_y), "destination")
        };
        writeln!(svg, "<circle cx=\"{at:.2}\" cy=\"{at_y:.2}\" r=\"4\" fill=\"#20252b\" data-unknown-end=\"{side}\" />").unwrap();
    }
    for (endpoint, (at, at_y)) in [(&m.from, anchor.send), (&m.to, anchor.receive)] {
        if let Some(id) = gate_id(endpoint) {
            if drawn_gates
                .iter()
                .any(|(name, x, y)| name == id && (x - at).abs() < 0.01 && (y - at_y).abs() < 0.01)
            {
                continue;
            }
            drawn_gates.push((id.into(), at, at_y));
            let gate = styles::gate(sheet, id, &paint);
            writeln!(svg, "<path d=\"M{at:.2},{:.2} v8\" stroke=\"{}\" stroke-width=\"{:.2}\" data-gate=\"{}\" />", at_y - 4.0, gate.color, gate.width, escape(id)).unwrap();
            styled_text(svg, at, at_y - 30.0, id, gate.font_size, &gate.color);
        }
    }
    if let Some(label) = &m.label {
        let bent_message = !allow_diagonal && m.from != m.to && (receive_y - y).abs() >= 0.01;
        let x = if m.from == m.to {
            start + 44.0 + label_width_at(label, paint.font_size)? / 2.0 + 8.0
        } else if bent_message {
            let width = label_width_at(label, paint.font_size)?;
            start + (end - start).signum() * (width / 2.0 + 20.0).min((end - start).abs() / 2.0)
        } else {
            (start + end) / 2.0
        };
        let lines = label.split('\n').count();
        let baseline_y = if bent_message {
            y
        } else {
            (y + receive_y) / 2.0
        };
        let baseline = baseline_y - 9.0 - paint.font_size * 1.25 * (lines - 1) as f64;
        if sheet.theme.modern() {
            let width = label_width_at(label, paint.font_size)? + 10.0;
            let height = paint.font_size * 1.25 * (lines - 1) as f64 + paint.font_size * 1.2 + 4.0;
            let fill = sheet
                .theme
                .colors(sheet.theme.default_base(crate::syntax::NodeKind::Package))
                .fill;
            writeln!(svg, "<rect data-message-pill=\"true\" x=\"{:.2}\" y=\"{:.2}\" width=\"{width:.2}\" height=\"{height:.2}\" rx=\"6\" fill=\"{fill}\" />",
                x-width/2.0,baseline-paint.font_size-2.0).unwrap();
        }
        styled_text(svg, x, baseline, label, paint.font_size, &paint.color);
    }
    Ok(())
}
