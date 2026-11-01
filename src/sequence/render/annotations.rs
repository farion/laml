//! Draw different visible time/duration forms and state boxes.

use super::{
    anchors::MessageAnchor, escape, executions::DrawnExecution, geometry::label_width_at, order,
    styles, text_collision,
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
    Ok(y.max(anchor + 15.0 + offset + height + 12.0 - 48.0))
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

fn invariant_left(
    end: &OrderingEnd,
    x: f64,
    y: f64,
    bars: &[DrawnExecution],
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    centers: &[f64],
) -> Result<f64, String> {
    let participant = match end {
        OrderingEnd::Execution { participant, .. } => Some(participant.as_str()),
        OrderingEnd::Message(anchor) => diagram.events.iter().find_map(|event| match event {
            Event::Message(m) if m.name.as_deref() == Some(&anchor.message) => {
                Some(if anchor.end == AnnotationEnd::Send {
                    m.from.as_str()
                } else {
                    m.to.as_str()
                })
            }
            _ => None,
        }),
    };
    let Some(participant) = participant else {
        return Ok(x + 24.0);
    };
    let Some(bar) = bars
        .iter()
        .filter(|bar| {
            bar.opened.participant == participant
                && y >= bar.opened.from - 0.01
                && y <= bar.to + 0.01
        })
        .max_by_key(|bar| bar.opened.depth)
    else {
        return Ok(x + 24.0);
    };
    let Some((index, person)) = diagram
        .participants
        .iter()
        .enumerate()
        .find(|(_, p)| p.id == participant)
    else {
        return Ok(x + 24.0);
    };
    let paint = styles::activation(
        sheet,
        participant,
        bar.opened.id.as_deref(),
        &person.classes,
    );
    let width = bar
        .opened
        .label
        .as_deref()
        .map(|label| label_width_at(label, paint.font_size).map(|w| w + 20.0))
        .transpose()?
        .unwrap_or(0.0)
        .max(paint.width);
    Ok((x + 24.0).max(centers[index] + 5.0 * bar.opened.depth as f64 + width / 2.0 + 12.0))
}

// All referenced coordinates and obstacles belong to the same rendered timeline.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    svg: &mut String,
    annotations: &[&Annotation],
    anchors: &[MessageAnchor],
    sheet: &StyleSheet,
    diagram: &SequenceDiagram,
    centers: &[f64],
    bars: &[DrawnExecution],
    positioned: &[(&super::Message, f64)],
    notes: &[(&crate::sequence::Note, f64)],
    continuations: &[(&crate::sequence::Continuation, f64)],
    frame_text: &[text_collision::Rect],
    header_bottom: f64,
    canvas_bottom: f64,
    canvas_width: f64,
) -> Result<Vec<text_collision::Rect>, String> {
    let mut occupied = text_collision::messages(positioned, anchors, sheet)?;
    let mut annotation_regions = Vec::new();
    occupied.extend_from_slice(frame_text);
    occupied.extend(text_collision::note_regions(
        diagram,
        anchors,
        centers,
        notes,
        continuations,
        sheet,
    )?);
    let mut connectors: Vec<Vec<(f64, f64)>> = Vec::new();
    for (index, annotation) in annotations.iter().enumerate() {
        let kind = styles::annotation_kind(annotation.kind);
        let paint = styles::annotation(sheet, kind);
        let color = &paint.color;
        let font = paint.font_size;
        let stroke = paint.width;
        let fill = &paint.fill;
        let (x, y) = order::endpoint(&annotation.first, anchors, diagram, centers, bars)?;
        let anchor_name = match &annotation.first {
            OrderingEnd::Message(end) => end.message.as_str(),
            OrderingEnd::Execution { id, .. } => id.as_str(),
        };
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
                let text_width = label_width_at(&annotation.text, font)?;
                let mut chosen = None;
                for baseline in [y - 23.0, y - 45.0, y + 20.0] {
                    for side in [1.0, -1.0] {
                        let label_x = if side > 0.0 {
                            midpoint + 19.0
                        } else {
                            midpoint - 19.0 - text_width
                        };
                        let tip_x = midpoint + 16.0 * side;
                        let region =
                            text_collision::text(label_x, baseline, &annotation.text, font)?;
                        if region.x >= 8.0
                            && region.x + region.width < canvas_width - 8.0
                            && region.y >= header_bottom + 4.0
                            && region.y + region.height < canvas_bottom - 8.0
                            && text_collision::region_clear(region, &occupied, &connectors)
                            && text_collision::path_clear(
                                &occupied,
                                &[(midpoint, y), (tip_x, baseline + 1.0)],
                            )
                        {
                            chosen = Some((label_x, baseline, tip_x, region));
                            break;
                        }
                    }
                    if chosen.is_some() {
                        break;
                    }
                }
                let (label_x, baseline, tip_x, region) = chosen.ok_or_else(|| {
                    format!(
                        "duration annotation '{}' from '{anchor_name}' has no clear text and leader position; increase the row gap",
                        annotation.text
                    )
                })?;
                occupied.push(region);
                annotation_regions.push(region);
                connectors.push(vec![(midpoint, y), (tip_x, baseline + 1.0)]);
                writeln!(svg, "<path d=\"M{midpoint:.2},{y:.2} L{tip_x:.2},{:.2}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{stroke:.2}\" data-annotation=\"duration-observation\" />", baseline + 1.0).unwrap();
                writeln!(svg, "<text x=\"{label_x:.2}\" y=\"{baseline:.2}\" font-family=\"Inter\" font-size=\"{font:.2}\" fill=\"{color}\">{}</text>", escape(&annotation.text)).unwrap();
                continue;
            }
            let text = if annotation.kind == AnnotationKind::DurationConstraint {
                format!("[{}]", annotation.text)
            } else {
                annotation.text.clone()
            };
            let middle = (top + bottom) / 2.0 + 4.0;
            let text_width = label_width_at(&text, font)?;
            let mut chosen = None;
            for side in [1.0, -1.0] {
                for extra in [0.0, 24.0, 48.0, 80.0] {
                    let lane = if side > 0.0 {
                        right + extra
                    } else {
                        top_x.min(bottom_x) - 22.0 - extra
                    };
                    if !text_collision::path_clear(
                        &occupied,
                        &[
                            (top_x, top),
                            (lane, top),
                            (lane, bottom),
                            (bottom_x, bottom),
                        ],
                    ) {
                        continue;
                    }
                    let label_x = if side > 0.0 {
                        lane + 7.0
                    } else {
                        lane - 7.0 - text_width
                    };
                    for baseline in [middle, middle - 22.0, middle + 22.0] {
                        let region = text_collision::text(label_x, baseline, &text, font)?;
                        if region.x >= 8.0
                            && region.x + region.width < canvas_width - 8.0
                            && region.y >= header_bottom + 4.0
                            && region.y + region.height < canvas_bottom - 8.0
                            && text_collision::region_clear(region, &occupied, &connectors)
                        {
                            chosen = Some((lane, label_x, baseline, region));
                            break;
                        }
                    }
                    if chosen.is_some() {
                        break;
                    }
                }
                if chosen.is_some() {
                    break;
                }
            }
            let (lane, label_x, baseline, region) = chosen.ok_or_else(|| {
                format!(
                    "duration annotation '{}' from '{anchor_name}' has no clear bracket and text position; increase the row gap",
                    annotation.text
                )
            })?;
            occupied.push(region);
            annotation_regions.push(region);
            connectors.push(vec![
                (top_x, top),
                (lane, top),
                (lane, bottom),
                (bottom_x, bottom),
            ]);
            writeln!(svg, "<path d=\"M{top_x:.2},{top:.2} H{lane:.2} V{bottom:.2} H{bottom_x:.2}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{stroke:.2}\" data-annotation=\"{kind}\" />").unwrap();
            writeln!(svg, "<text x=\"{label_x:.2}\" y=\"{baseline:.2}\" text-anchor=\"start\" font-family=\"Inter\" font-size=\"{font:.2}\" fill=\"{color}\">{}</text>", escape(&text)).unwrap();
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
        let mut top = y + 15.0 + offset;
        let box_height = (font * 1.25 + 8.0).max(24.0);
        if !matches!(annotation.kind, AnnotationKind::TimeObservation) {
            let label = match annotation.kind {
                AnnotationKind::TimeConstraint => format!("[{}]", annotation.text),
                AnnotationKind::Invariant => format!("{{{}}}", annotation.text),
                _ => annotation.text.clone(),
            };
            let text_width = label_width_at(&label, font)?;
            let (left, width) = match annotation.kind {
                AnnotationKind::Action => (x + 24.0, text_width + 20.0),
                AnnotationKind::State => (x - text_width / 2.0 - 10.0, text_width + 20.0),
                AnnotationKind::Invariant => (
                    invariant_left(&annotation.first, x, y, bars, diagram, sheet, centers)?,
                    text_width,
                ),
                _ => (x, text_width),
            };
            let chosen = (0..=3).find_map(|step| {
                let candidate_top = top + step as f64 * 24.0;
                let region = text_collision::Rect {
                    x: left,
                    y: candidate_top,
                    width,
                    height: box_height,
                };
                let link_clear = annotation.kind != AnnotationKind::Action
                    || text_collision::path_clear(
                        &occupied,
                        &[(x, y), (left, y), (left, candidate_top)],
                    );
                (region.y >= header_bottom + 4.0
                    && region.y + region.height < canvas_bottom - 8.0
                    && text_collision::region_clear(region, &occupied, &connectors)
                    && link_clear)
                    .then_some((candidate_top, region))
            });
            let (at, region) = chosen.ok_or_else(|| {
                format!(
                    "annotation '{}' at '{anchor_name}' overlaps message text; increase spacing around this message",
                    annotation.text
                )
            })?;
            top = at;
            occupied.push(region);
            annotation_regions.push(region);
        }
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
                    let (at_x, at_y) = text_collision::time_label(
                        &occupied,
                        &connectors,
                        x,
                        y,
                        &annotation.text,
                        font,
                        header_bottom + 4.0,
                        canvas_bottom,
                        offset,
                    )
                    .map_err(|reason| format!("annotation at '{anchor_name}': {reason}"))?;
                    let dx = if at_x < x { -18.0 } else { 18.0 };
                    connectors.push(vec![(x, y), (x + dx, at_y + 1.0)]);
                    writeln!(svg, "<path d=\"M{x:.2},{y:.2} l{dx:.2},{:.2}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{stroke:.2}\" data-annotation=\"time-observation\" />", at_y - y + 1.0).unwrap();
                    ("time-observation", annotation.text.clone(), at_x, at_y)
                };
                if annotation.kind == AnnotationKind::TimeObservation {
                    let region = text_collision::text(at_x, at_y, &label, font)?;
                    occupied.push(region);
                    annotation_regions.push(region);
                }
                writeln!(svg, "<text x=\"{at_x:.2}\" y=\"{at_y:.2}\" text-anchor=\"start\" font-family=\"Inter\" font-size=\"{font:.2}\" fill=\"{color}\" data-annotation=\"{kind}\">{}</text>",
                    escape(&label)).unwrap();
            }
            AnnotationKind::Invariant => {
                let width = label_width_at(&format!("{{{}}}", annotation.text), font)?;
                let center =
                    invariant_left(&annotation.first, x, y, bars, diagram, sheet, centers)?
                        + width / 2.0;
                writeln!(svg, "<text x=\"{center:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"{font:.2}\" fill=\"{color}\" data-annotation=\"invariant\">{{{}}}</text>",
                    top + font + 3.0, escape(&annotation.text)).unwrap();
            }
            AnnotationKind::Action => {
                let width = label_width_at(&annotation.text, font)? + 20.0;
                let left = x + 24.0;
                connectors.push(vec![(x, y), (left, y), (left, top)]);
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
    Ok(annotation_regions)
}
