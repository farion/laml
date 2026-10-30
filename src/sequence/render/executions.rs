//! Per-instance execution bars, nested on participant lifelines.

use super::{
    SequenceDiagram,
    geometry::{label_width_at, x_of},
    styled_text, styles,
};
use crate::styles::StyleSheet;
use std::fmt::Write;

#[derive(Clone)]
pub(super) struct OpenExecution {
    pub participant: String,
    pub id: Option<String>,
    pub label: Option<String>,
    pub from: f64,
    pub depth: usize,
}

pub(super) struct DrawnExecution {
    pub opened: OpenExecution,
    pub to: f64,
}

/// Re-enter the next operand without redrawing executions already closed in
/// the previous operand over its entire vertical range.
pub(super) fn next_operand(open: &mut Vec<OpenExecution>, incoming: &[OpenExecution], y: f64) {
    let mut next = incoming.to_vec();
    for bar in &mut next {
        if !open.iter().any(|current| {
            current.participant == bar.participant
                && current.id == bar.id
                && current.depth == bar.depth
                && current.from == bar.from
        }) {
            bar.from = y;
            bar.label = None;
        }
    }
    *open = next;
}

impl OpenExecution {
    pub(super) fn finish(self, to: f64) -> DrawnExecution {
        DrawnExecution {
            to: to.max(self.from + 2.0),
            opened: self,
        }
    }
}

pub(super) fn draw(
    svg: &mut String,
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    centers: &[f64],
    bars: &[DrawnExecution],
) -> Result<(), String> {
    for bar in bars {
        let OpenExecution {
            participant: id,
            id: local_id,
            label,
            from,
            depth,
        } = &bar.opened;
        let classes = &diagram
            .participants
            .iter()
            .find(|p| p.id == *id)
            .ok_or("unknown activation participant")?
            .classes;
        let paint = styles::activation(sheet, id, local_id.as_deref(), classes);
        let width = if let Some(label) = label {
            paint
                .width
                .max(label_width_at(label, paint.font_size)? + 20.0)
        } else {
            paint.width
        };
        let x = x_of(diagram, centers, id)? - width / 2.0 + 5.0 * *depth as f64;
        let height = bar.to - from;
        writeln!(svg, "<rect x=\"{x:.2}\" y=\"{from:.2}\" width=\"{width:.2}\" height=\"{height:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" data-activation-owner=\"{}\" data-activation-id=\"{}\" />",
            paint.fill, paint.border, paint.border_width, styles::escape(id),
            styles::escape(local_id.as_deref().unwrap_or(""))).unwrap();
        if let Some(label) = label {
            styled_text(
                svg,
                x + width / 2.0,
                from + 9.0 + paint.font_size,
                label,
                paint.font_size,
                &paint.color,
            );
        }
    }
    Ok(())
}
