//! Draw referenced note boxes in their own event rows.

use super::{
    SequenceDiagram,
    anchors::{MessageAnchor, position},
    geometry::label_width_at,
    styles,
    styles::escape,
    text_collision::{self, Rect},
};
use crate::sequence::{
    AnnotationEnd, Note,
    notes::{NoteSide, Placement},
};
use crate::styles::StyleSheet;
use std::fmt::Write;

pub(super) fn left(
    diagram: &SequenceDiagram,
    centers: &[f64],
    anchors: &[MessageAnchor],
    placement: &Placement,
    width: f64,
) -> Result<f64, String> {
    let center = if let Some((name, end)) = placement.target.split_once('.') {
        position(
            anchors,
            name,
            if end == "send" {
                AnnotationEnd::Send
            } else {
                AnnotationEnd::Receive
            },
        )?
        .0
    } else {
        let index = diagram
            .participants
            .iter()
            .position(|p| p.id == placement.target)
            .ok_or_else(|| format!("unknown participant '{}'", placement.target))?;
        centers[index]
    };
    Ok(match placement.side {
        NoteSide::Over if placement.target.contains('.') => center + 18.0,
        NoteSide::Over => center - width / 2.0,
        NoteSide::Right => center + 18.0 + placement.gap,
    })
}

pub(super) fn height(note: &Note, sheet: &StyleSheet) -> f64 {
    let font = styles::note(sheet, &note.id).font_size;
    (note.text.split('\n').count() as f64 * font * 1.25 + 18.0).max(38.0)
}

// Keep annotation text and note obstacles on the same timeline snapshot.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    svg: &mut String,
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    centers: &[f64],
    notes: &[(&Note, f64)],
    anchors: &[MessageAnchor],
    obstacles: &[Rect],
    note_shapes: &[Rect],
) -> Result<(), String> {
    for (index, (note, top)) in notes.iter().enumerate() {
        let placement = diagram
            .note_layout
            .iter()
            .find(|p| p.note == note.id)
            .ok_or_else(|| format!("note '{}' has no placement", note.id))?;
        let paint = styles::note(sheet, &note.id);
        let width = label_width_at(&note.text, paint.font_size)? + 32.0;
        let height = height(note, sheet);
        let x = if placement.side == NoteSide::Over && placement.target.contains('.') {
            let (name, end) = placement.target.split_once('.').unwrap();
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
            let middle = top + height / 2.0;
            let start = (anchor_x, at);
            let target = (left, middle);
            let mut blockers = obstacles.to_vec();
            blockers.extend(
                note_shapes
                    .iter()
                    .enumerate()
                    .filter(|(other, _)| *other != index)
                    .map(|(_, rect)| *rect),
            );
            let mut options = vec![vec![start, (anchor_x, middle), target]];
            for lane in [anchor_x - 24.0, anchor_x - 48.0, anchor_x - 72.0]
                .into_iter()
                .chain(blockers.iter().map(|rect| rect.x - 12.0))
            {
                if lane >= 8.0 && lane < anchor_x {
                    options.push(vec![start, (lane, at), (lane, middle), target]);
                }
            }
            let center = left + width / 2.0;
            options.push(vec![
                start,
                (anchor_x, top - 16.0),
                (center, top - 16.0),
                (center, *top),
            ]);
            options.push(vec![
                start,
                (anchor_x, top + height + 16.0),
                (center, top + height + 16.0),
                (center, top + height),
            ]);
            let chosen = options
                .into_iter()
                .find(|path| text_collision::path_clear(&blockers, path))
                .ok_or_else(|| {
                    format!(
                        "note '{}' cannot reach '{}' without crossing visible text; move the note or increase its gap",
                        note.id, placement.target
                    )
                })?;
            let mut d = String::new();
            for (segment, (x, y)) in chosen.iter().enumerate() {
                write!(d, "{}{x:.2},{y:.2}", if segment == 0 { "M" } else { " L" }).unwrap();
            }
            writeln!(svg, "<path d=\"{d}\" fill=\"none\" stroke=\"{}\" stroke-dasharray=\"4 3\" data-note-anchor=\"{}\" />",
                escape(&paint.border), escape(&placement.target)).unwrap();
            left
        } else {
            left(diagram, centers, anchors, placement, width)?
        };
        if sheet.theme.modern() {
            let radius = 8.0;
            writeln!(svg, "<rect x=\"{x:.2}\" y=\"{top:.2}\" width=\"{width:.2}\" height=\"{height:.2}\" rx=\"{radius:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" data-note=\"{}\" />",
                escape(&paint.fill), escape(&paint.border), paint.border_width, escape(&note.id)).unwrap();
            writeln!(svg, "<clipPath id=\"laml-seq-note-{index}\"><rect x=\"{x:.2}\" y=\"{top:.2}\" width=\"{width:.2}\" height=\"{height:.2}\" rx=\"{radius:.2}\" /></clipPath>").unwrap();
            writeln!(svg, "<rect x=\"{x:.2}\" y=\"{top:.2}\" width=\"6\" height=\"{height:.2}\" fill=\"{}\" clip-path=\"url(#laml-seq-note-{index})\" data-sequence-note-accent=\"{}\" />",
                styles::note_accent(sheet, &note.id), escape(&note.id)).unwrap();
        } else {
            let fold = 12.0;
            writeln!(svg, "<path d=\"M{x:.2},{top:.2} h{:.2} l{fold:.2},{fold:.2} v{:.2} h-{width:.2} Z M{:.2},{top:.2} v{fold:.2} h{fold:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" data-note=\"{}\" />",
                width - fold, height - fold, x + width - fold,
                escape(&paint.fill), escape(&paint.border), paint.border_width, escape(&note.id)).unwrap();
        }
        for (line, value) in note.text.split('\n').enumerate() {
            writeln!(svg, "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\">{}</text>",
                x + 16.0, top + 11.0 + paint.font_size + line as f64 * paint.font_size * 1.25,
                paint.font_size, escape(&paint.color), escape(value)).unwrap();
        }
    }
    Ok(())
}
