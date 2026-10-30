//! Draw referenced note boxes in their own event rows.

use super::{
    SequenceDiagram,
    anchors::{MessageAnchor, position},
    geometry::label_width_at,
    styles,
    styles::escape,
};
use crate::sequence::{AnnotationEnd, Note};
use crate::styles::StyleSheet;
use std::fmt::Write;

pub(super) fn height(note: &Note, sheet: &StyleSheet) -> f64 {
    let font = styles::note(sheet, &note.id).font_size;
    (note.text.split('\n').count() as f64 * font * 1.25 + 18.0).max(38.0)
}

pub(super) fn draw(
    svg: &mut String,
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    centers: &[f64],
    notes: &[(&Note, f64)],
    anchors: &[MessageAnchor],
) -> Result<(), String> {
    for (note, top) in notes {
        let placement = diagram
            .note_layout
            .iter()
            .find(|p| p.note == note.id)
            .ok_or_else(|| format!("note '{}' has no placement", note.id))?;
        let paint = styles::note(sheet, &note.id);
        let width = label_width_at(&note.text, paint.font_size)? + 32.0;
        let height = height(note, sheet);
        let x = if let Some((name, end)) = placement.target.split_once('.') {
            let (anchor_x, at) = position(
                anchors,
                name,
                if end == "send" {
                    AnnotationEnd::Send
                } else {
                    AnnotationEnd::Receive
                },
            )?;
            let left = anchor_x + 18.0;
            writeln!(svg, "<path d=\"M{anchor_x:.2},{at:.2} L{anchor_x:.2},{:.2} L{left:.2},{:.2}\" fill=\"none\" stroke=\"{}\" stroke-dasharray=\"4 3\" data-note-anchor=\"{}\" />",
                top + height / 2.0, top + height / 2.0,
                escape(&paint.border), escape(&placement.target)).unwrap();
            left
        } else {
            let index = diagram
                .participants
                .iter()
                .position(|p| p.id == placement.target)
                .ok_or_else(|| format!("unknown participant '{}'", placement.target))?;
            centers[index] - width / 2.0
        };
        let fold = 12.0;
        writeln!(svg, "<path d=\"M{x:.2},{top:.2} h{:.2} l{fold:.2},{fold:.2} v{:.2} h-{width:.2} Z M{:.2},{top:.2} v{fold:.2} h{fold:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" data-note=\"{}\" />",
            width - fold, height - fold, x + width - fold,
            escape(&paint.fill), escape(&paint.border), paint.border_width, escape(&note.id)).unwrap();
        for (line, value) in note.text.split('\n').enumerate() {
            writeln!(svg, "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\">{}</text>",
                x + 16.0, top + 11.0 + paint.font_size + line as f64 * paint.font_size * 1.25,
                paint.font_size, escape(&paint.color), escape(value)).unwrap();
        }
    }
    Ok(())
}
