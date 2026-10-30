//! Sequence message strokes, labels, arrows, unknown ends and gates.
use super::styles::escape;
use super::{
    Message, MessageAnchor, MessageKind, arrows, geometry::label_width_at, styled_text, styles,
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
) -> Result<(), String> {
    let (start, y) = anchor.send;
    let (end, receive_y) = anchor.receive;
    let paint = styles::message(sheet, m.name.as_deref(), &m.classes);
    let name = escape(m.name.as_deref().unwrap_or(""));
    let dash = if matches!(m.kind, MessageKind::Reply | MessageKind::Create) {
        " stroke-dasharray=\"7 5\""
    } else {
        ""
    };
    if m.from == m.to {
        let height = receive_y - y;
        writeln!(svg, "<path d=\"M{start:.2},{y:.2} h44 v{height:.2} h-44\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\"{dash} data-message=\"{name}\" />", paint.color, paint.width).unwrap();
        if m.kind != MessageKind::Delete {
            arrows::draw(
                svg,
                (end, receive_y),
                (-44.0, 0.0),
                m.kind == MessageKind::Sync || m.filled_reply,
                &paint.color,
            );
        }
    } else {
        writeln!(svg, "<path d=\"M{start:.2},{y:.2} L{end:.2},{receive_y:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\"{dash} data-message=\"{name}\" />", paint.color, paint.width).unwrap();
        if m.to != "?" && m.kind != MessageKind::Delete {
            arrows::draw(
                svg,
                (end, receive_y),
                (end - start, receive_y - y),
                m.kind == MessageKind::Sync || m.filled_reply,
                &paint.color,
            );
        }
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
        let x = if m.from == m.to {
            start + 44.0 + label_width_at(label, paint.font_size)? / 2.0 + 8.0
        } else {
            (start + end) / 2.0
        };
        let lines = label.split('\n').count();
        styled_text(
            svg,
            x,
            (y + receive_y) / 2.0 - 9.0 - paint.font_size * 1.25 * (lines - 1) as f64,
            label,
            paint.font_size,
            &paint.color,
        );
    }
    Ok(())
}
