//! Draw different visible time/duration forms and state boxes.

use super::{
    anchors::MessageAnchor, escape, executions::DrawnExecution, geometry::label_width_at, order,
    styles,
};
use crate::sequence::{
    Annotation, AnnotationEnd, AnnotationKind, Event, OrderingEnd, SequenceDiagram,
};
use crate::styles::StyleSheet;
use std::fmt::Write;

pub(super) fn reserve_row(
    diagram: &SequenceDiagram,
    annotation: &Annotation,
    previous: &[&Annotation],
    positions: &[(usize, f64)],
    sheet: &StyleSheet,
    y: f64,
) -> Result<f64, String> {
    if annotation.second.is_some() || matches!(annotation.kind, AnnotationKind::TimeObservation) {
        return Ok(y);
    }
    let OrderingEnd::Message(first) = &annotation.first else {
        return Ok(y);
    };
    let message = diagram
        .events
        .iter()
        .find_map(|event| match event {
            Event::Message(m) if m.name.as_deref() == Some(&first.message) => Some(m),
            _ => None,
        })
        .ok_or("unknown annotation message")?;
    let anchor = positions
        .iter()
        .find(|(line, _)| *line == message.line)
        .map(|(_, at)| *at)
        .ok_or("missing annotation event position")?
        + if first.end == AnnotationEnd::Receive && message.from == message.to {
            24.0
        } else {
            0.0
        };
    let offset: f64 = previous
        .iter()
        .filter(|earlier| earlier.first == annotation.first && earlier.second.is_none())
        .map(|earlier| {
            (styles::annotation(sheet, styles::annotation_kind(earlier.kind)).font_size * 1.25
                + 12.0)
                .max(27.0)
        })
        .sum();
    let font = styles::annotation(sheet, styles::annotation_kind(annotation.kind)).font_size;
    let height = (font * 1.25 + 8.0).max(24.0);
    Ok(y.max(anchor + 15.0 + offset + height + 12.0 - 58.0))
}

fn execution_half_width(
    end: &OrderingEnd,
    bars: &[DrawnExecution],
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
) -> Result<f64, String> {
    let OrderingEnd::Execution {
        participant, id, ..
    } = end
    else {
        return Ok(0.0);
    };
    let bar = bars
        .iter()
        .find(|bar| bar.opened.participant == *participant && bar.opened.id.as_deref() == Some(id))
        .ok_or("missing execution")?;
    let classes = &diagram
        .participants
        .iter()
        .find(|p| p.id == *participant)
        .ok_or("missing participant")?
        .classes;
    let paint = styles::activation(sheet, participant, Some(id), classes);
    Ok(bar
        .opened
        .label
        .as_deref()
        .map(|label| label_width_at(label, paint.font_size).map(|w| paint.width.max(w + 20.0)))
        .transpose()?
        .unwrap_or(paint.width)
        / 2.0)
}

pub(super) fn draw(
    svg: &mut String,
    annotations: &[&Annotation],
    anchors: &[MessageAnchor],
    sheet: &StyleSheet,
    diagram: &SequenceDiagram,
    centers: &[f64],
    bars: &[DrawnExecution],
) -> Result<(), String> {
    for (index, annotation) in annotations.iter().enumerate() {
        let kind = styles::annotation_kind(annotation.kind);
        let paint = styles::annotation(sheet, kind);
        let color = &paint.color;
        let font = paint.font_size;
        let stroke = paint.width;
        let fill = &paint.fill;
        let (x, y) = order::endpoint(&annotation.first, anchors, diagram, centers, bars)?;
        if let Some(second) = &annotation.second {
            let (other_x, other_y) = order::endpoint(second, anchors, diagram, centers, bars)?;
            let right = (x + execution_half_width(&annotation.first, bars, diagram, sheet)?)
                .max(other_x + execution_half_width(second, bars, diagram, sheet)?)
                + 22.0;
            let (top, top_x, bottom, bottom_x) = if y < other_y {
                (y, x, other_y, other_x)
            } else if y > other_y {
                (other_y, other_x, y, x)
            } else {
                (y, x, y + 22.0, other_x)
            };
            let kind = if annotation.kind == AnnotationKind::DurationConstraint {
                "duration-constraint"
            } else {
                "duration-observation"
            };
            if annotation.kind == AnnotationKind::DurationObservation
                && (y - other_y).abs() < 0.01
                && (x - other_x).abs() > 1.0
            {
                let midpoint = (x + other_x) / 2.0;
                writeln!(svg, "<path d=\"M{midpoint:.2},{y:.2} l16,-22\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{stroke:.2}\" data-annotation=\"duration-observation\" />").unwrap();
                writeln!(svg, "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"{font:.2}\" fill=\"{color}\">{}</text>",
                    midpoint + 19.0, y - 23.0, escape(&annotation.text)).unwrap();
                continue;
            }
            writeln!(svg, "<path d=\"M{top_x:.2},{top:.2} H{right:.2} V{bottom:.2} H{bottom_x:.2}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{stroke:.2}\" data-annotation=\"{kind}\" />").unwrap();
            let text = if annotation.kind == AnnotationKind::DurationConstraint {
                format!("[{}]", annotation.text)
            } else {
                annotation.text.clone()
            };
            writeln!(svg, "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"start\" font-family=\"Inter\" font-size=\"{font:.2}\" fill=\"{color}\">{}</text>",
                right + 7.0, (top + bottom) / 2.0 + 4.0, escape(&text)).unwrap();
            continue;
        }
        let offset: f64 = annotations[..index]
            .iter()
            .filter(|earlier| earlier.first == annotation.first && earlier.second.is_none())
            .map(|earlier| {
                (styles::annotation(sheet, styles::annotation_kind(earlier.kind)).font_size * 1.25
                    + 12.0)
                    .max(27.0)
            })
            .sum();
        let top = y + 15.0 + offset;
        let box_height = (font * 1.25 + 8.0).max(24.0);
        match annotation.kind {
            AnnotationKind::TimeObservation | AnnotationKind::TimeConstraint => {
                let (kind, label, at_x, at_y) = if annotation.kind == AnnotationKind::TimeConstraint
                {
                    (
                        "time-constraint",
                        format!("[{}]", annotation.text),
                        x,
                        top + 15.0,
                    )
                } else {
                    writeln!(svg, "<path d=\"M{x:.2},{y:.2} l18,-{:.2}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{stroke:.2}\" data-annotation=\"time-observation\" />", 17.0 + offset).unwrap();
                    (
                        "time-observation",
                        annotation.text.clone(),
                        x + 22.0,
                        y - 18.0 - offset,
                    )
                };
                writeln!(svg, "<text x=\"{at_x:.2}\" y=\"{at_y:.2}\" text-anchor=\"start\" font-family=\"Inter\" font-size=\"{font:.2}\" fill=\"{color}\" data-annotation=\"{kind}\">{}</text>",
                    escape(&label)).unwrap();
            }
            AnnotationKind::Invariant => {
                writeln!(svg, "<text x=\"{x:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"{font:.2}\" fill=\"{color}\" data-annotation=\"invariant\">{{{}}}</text>",
                    top + font + 3.0, escape(&annotation.text)).unwrap();
            }
            AnnotationKind::Action => {
                let width = label_width_at(&annotation.text, font)? + 20.0;
                let left = x + 24.0;
                writeln!(svg, "<path d=\"M{x:.2},{y:.2} H{left:.2} V{top:.2}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{stroke:.2}\" data-action-link=\"true\" />").unwrap();
                writeln!(svg, "<rect x=\"{left:.2}\" y=\"{top:.2}\" width=\"{width:.2}\" height=\"{box_height:.2}\" fill=\"{fill}\" stroke=\"{color}\" stroke-width=\"{stroke:.2}\" data-annotation=\"action\" />").unwrap();
                writeln!(svg, "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"{font:.2}\" fill=\"{color}\">{}</text>",
                    left + width / 2.0, top + font + 3.0, escape(&annotation.text)).unwrap();
            }
            AnnotationKind::State => {
                let width = label_width_at(&annotation.text, font)? + 20.0;
                writeln!(svg, "<rect x=\"{:.2}\" y=\"{top:.2}\" width=\"{width:.2}\" height=\"{box_height:.2}\" rx=\"9\" fill=\"{fill}\" stroke=\"{color}\" stroke-width=\"{stroke:.2}\" data-annotation=\"state\" />",
                    x - width / 2.0).unwrap();
                writeln!(svg, "<text x=\"{x:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"{font:.2}\" fill=\"{color}\">{}</text>",
                    top + font + 3.0, escape(&annotation.text)).unwrap();
            }
            AnnotationKind::DurationObservation | AnnotationKind::DurationConstraint => {
                return Err("duration requires two anchors".into());
            }
        }
    }
    Ok(())
}
