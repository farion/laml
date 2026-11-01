//! Size each interval for content that actually occupies that interval.
//! A wide note on one lifeline must not spread every other lifeline apart.
use super::{Event, SequenceDiagram, geometry::label_width_at, styles};
use crate::styles::StyleSheet;

pub(super) fn measure(
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    widths: &[f64],
) -> Result<Vec<f64>, String> {
    let mut gaps = vec![72.0_f64; widths.len().saturating_sub(1)];
    for event in &diagram.events {
        match event {
            Event::Message(message) => {
                let from = diagram
                    .participants
                    .iter()
                    .position(|p| p.id == message.from);
                let to = diagram.participants.iter().position(|p| p.id == message.to);
                if let (Some(a), Some(b), Some(label)) = (from, to, &message.label) {
                    let span = a.abs_diff(b);
                    if span == 1 {
                        gaps[a.min(b)] = gaps[a.min(b)].max(
                            label_width_at(
                                label,
                                styles::message(sheet, message.name.as_deref(), &message.classes)
                                    .font_size,
                            )? + 36.0,
                        );
                    }
                }
            }
            Event::Note(note) => {
                let Some(placement) = diagram.note_layout.iter().find(|p| p.note == note.id) else {
                    continue;
                };
                let target = if let Some((name, end)) = placement.target.split_once('.') {
                    diagram.events.iter().find_map(|e| match e {
                        Event::Message(m) if m.name.as_deref() == Some(name) => {
                            Some(if end == "send" {
                                m.from.as_str()
                            } else {
                                m.to.as_str()
                            })
                        }
                        _ => None,
                    })
                } else {
                    Some(placement.target.as_str())
                };
                let Some(i) =
                    target.and_then(|t| diagram.participants.iter().position(|p| p.id == t))
                else {
                    continue;
                };
                if i >= gaps.len() {
                    continue;
                }
                let width =
                    label_width_at(&note.text, styles::note(sheet, &note.id).font_size)? + 32.0;
                let extension = if placement.side == crate::sequence::notes::NoteSide::Right {
                    width + 18.0 + placement.gap
                } else if placement.target.contains('.') {
                    width + 18.0
                } else {
                    width / 2.0
                };
                gaps[i] = gaps[i].max(extension + 14.0 - widths[i] / 2.0);
            }
            _ => {}
        }
    }
    for coverage in &diagram.frame_layout {
        let first = coverage
            .participants
            .iter()
            .filter_map(|name| diagram.participants.iter().position(|p| p.id == *name))
            .min();
        if let Some(index) = first.filter(|&index| index > 0) {
            let padding = super::geometry::guard_padding(diagram, sheet, widths[index])?;
            gaps[index - 1] = gaps[index - 1].max(padding + 24.0);
        }
    }
    Ok(gaps)
}

pub(super) fn trailing(
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    centers: &[f64],
    widths: &[f64],
) -> Result<f64, String> {
    let Some(last) = diagram.participants.last() else {
        return Ok(64.0);
    };
    let x = centers[centers.len() - 1];
    let right = x + widths[widths.len() - 1] / 2.0;
    let mut extent = right + 64.0;
    for event in &diagram.events {
        match event {
            Event::Note(note) => {
                let Some(place) = diagram.note_layout.iter().find(|p| p.note == note.id) else {
                    continue;
                };
                let target = if let Some((name, end)) = place.target.split_once('.') {
                    diagram.events.iter().find_map(|e| match e {
                        Event::Message(m) if m.name.as_deref() == Some(name) => {
                            Some(if end == "send" {
                                m.from.as_str()
                            } else {
                                m.to.as_str()
                            })
                        }
                        _ => None,
                    })
                } else {
                    Some(place.target.as_str())
                };
                if target != Some(last.id.as_str()) {
                    continue;
                }
                let width =
                    label_width_at(&note.text, styles::note(sheet, &note.id).font_size)? + 32.0;
                let left = match place.side {
                    crate::sequence::notes::NoteSide::Right => x + 18.0 + place.gap,
                    crate::sequence::notes::NoteSide::Over if place.target.contains('.') => {
                        x + 18.0
                    }
                    crate::sequence::notes::NoteSide::Over => x - width / 2.0,
                };
                extent = extent.max(left + width + 24.0);
            }
            Event::Annotation(annotation) => {
                let on_last = match &annotation.first {
                    crate::sequence::OrderingEnd::Message(end) => {
                        diagram.events.iter().any(|e| match e {
                            Event::Message(m) if m.name.as_deref() == Some(&end.message) => {
                                (end.end == crate::sequence::AnnotationEnd::Send
                                    && m.from == last.id)
                                    || (end.end == crate::sequence::AnnotationEnd::Receive
                                        && m.to == last.id)
                            }
                            _ => false,
                        })
                    }
                    crate::sequence::OrderingEnd::Execution { participant, .. } => {
                        *participant == last.id
                    }
                };
                if on_last {
                    let width = label_width_at(
                        &annotation.text,
                        styles::annotation(sheet, styles::annotation_kind(annotation.kind))
                            .font_size,
                    )?;
                    extent = extent.max(x + width + 88.0);
                }
            }
            Event::Message(message) if message.from == last.id && message.to == "?" => {
                let width = message
                    .label
                    .as_deref()
                    .map(|label| {
                        label_width_at(
                            label,
                            styles::message(sheet, message.name.as_deref(), &message.classes)
                                .font_size,
                        )
                    })
                    .transpose()?
                    .unwrap_or(0.0);
                extent = extent.max(x + 48.0 + width / 2.0 + 28.0);
            }
            Event::Message(message) if message.from == last.id && message.to == last.id => {
                let width = message
                    .label
                    .as_deref()
                    .map(|label| {
                        label_width_at(
                            label,
                            styles::message(sheet, message.name.as_deref(), &message.classes)
                                .font_size,
                        )
                    })
                    .transpose()?
                    .unwrap_or(0.0);
                extent = extent.max(x + 44.0 + width + 32.0);
            }
            Event::Message(message) if super::super::messages::gate_id(&message.to).is_some() => {
                extent = extent.max(right + 96.0);
            }
            _ => {}
        }
    }
    Ok(extent - right)
}
