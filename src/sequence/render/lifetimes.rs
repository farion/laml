//! Lifeline segments can repeat in mutually exclusive operands under one ID.

use super::{Event, HEADER_Y, SequenceDiagram, geometry::head_label, styled_text, styles};
use crate::styles::StyleSheet;
use std::fmt::Write;

struct Segment {
    head: Option<f64>,
    start: f64,
    end: f64,
}

pub(super) struct Timeline<'a> {
    pub diagram: &'a SequenceDiagram,
    pub positions: &'a [(usize, f64)],
    pub deaths: &'a [(String, f64)],
    pub branches: &'a [f64],
    pub header_height: f64,
    pub bottom: f64,
}

fn segments(
    diagram: &SequenceDiagram,
    id: &str,
    positions: &[(usize, f64)],
    deaths: &[(String, f64)],
    branches: &[f64],
    header_height: f64,
    bottom: f64,
) -> Result<Vec<Segment>, String> {
    let births: Vec<_> = diagram
        .births
        .iter()
        .filter(|(participant, _)| participant == id)
        .map(|(_, line)| *line)
        .collect();
    let ends: Vec<_> = deaths
        .iter()
        .filter(|(name, _)| name == id)
        .map(|(_, y)| *y)
        .collect();
    if births.is_empty() {
        let mut result = vec![Segment {
            head: Some(HEADER_Y),
            start: HEADER_Y + header_height,
            end: ends.first().copied().unwrap_or(bottom - 16.0),
        }];
        for pair in ends.windows(2) {
            let start = branches
                .iter()
                .copied()
                .find(|branch| *branch > pair[0] && *branch < pair[1])
                .ok_or_else(|| format!("missing alternate operand for '{id}'"))?;
            result.push(Segment {
                head: None,
                start,
                end: pair[1],
            });
        }
        return Ok(result);
    }
    births
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let y = positions
                .iter()
                .find(|(at, _)| at == line)
                .map(|(_, y)| *y)
                .ok_or_else(|| format!("created participant '{id}' has no event position"))?;
            let incoming = diagram.events.iter().any(|event| {
                matches!(event,
            Event::Message(m) if m.line == *line && m.to == id)
            });
            let head = if incoming { y - header_height / 2.0 } else { y };
            Ok(Segment {
                head: Some(head),
                start: head + header_height,
                end: ends.get(index).copied().unwrap_or(bottom - 16.0),
            })
        })
        .collect()
}

pub(super) fn draw_lines(
    svg: &mut String,
    sheet: &StyleSheet,
    centers: &[f64],
    timeline: &Timeline<'_>,
) -> Result<(), String> {
    let Timeline {
        diagram,
        positions,
        deaths,
        branches,
        header_height,
        bottom,
    } = timeline;
    for (i, participant) in diagram.participants.iter().enumerate() {
        let x = centers[i];
        let paint = styles::lifeline(sheet, &participant.id, &participant.classes);
        for segment in segments(
            diagram,
            &participant.id,
            positions,
            deaths,
            branches,
            *header_height,
            *bottom,
        )? {
            writeln!(svg, "<path d=\"M{x:.2},{:.2} L{x:.2},{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-dasharray=\"5 5\" />",
                segment.start, segment.end, paint.color, paint.width).unwrap();
        }
    }
    Ok(())
}

pub(super) fn draw_heads(
    svg: &mut String,
    sheet: &StyleSheet,
    centers: &[f64],
    widths: &[f64],
    timeline: &Timeline<'_>,
) -> Result<(), String> {
    let Timeline {
        diagram,
        positions,
        deaths,
        branches,
        header_height,
        bottom,
    } = timeline;
    for (i, participant) in diagram.participants.iter().enumerate() {
        let x = centers[i] - widths[i] / 2.0;
        let paint = styles::node(sheet, &participant.id, &participant.classes);
        for segment in segments(
            diagram,
            &participant.id,
            positions,
            deaths,
            branches,
            *header_height,
            *bottom,
        )? {
            let Some(head_y) = segment.head else { continue };
            writeln!(svg, "<rect x=\"{x:.2}\" y=\"{head_y:.2}\" width=\"{:.2}\" height=\"{header_height:.2}\" fill=\"{}\" stroke=\"{}\" data-participant=\"{}\" />",
                widths[i], paint.fill, paint.border, styles::escape(&participant.id)).unwrap();
            styled_text(
                svg,
                centers[i],
                head_y + 30.0,
                &head_label(diagram, participant),
                paint.font_size,
                &paint.text,
            );
        }
    }
    Ok(())
}
