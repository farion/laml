//! Font-backed lane sizing and frame boundaries.

use super::{Event, FONT_SIZE, SequenceDiagram, styles};
use crate::sequence::Participant;
use crate::sequence::messages::gate_id;
use crate::styles::StyleSheet;
use crate::text::measure;

pub(super) fn label_width_at(text: &str, size: f64) -> Result<f64, String> {
    text.split('\n')
        .map(|line| measure(line, size))
        .try_fold(0.0_f64, |a, b| Ok::<f64, String>(a.max(b?)))
}
pub(super) fn head_label(diagram: &SequenceDiagram, participant: &Participant) -> String {
    match diagram
        .decompositions
        .iter()
        .find(|d| d.parent == participant.id)
    {
        Some(part) => format!(
            "{}\nref {}{}",
            participant.label,
            part.title,
            if part.strict { " strict" } else { "" }
        ),
        None => participant.label.clone(),
    }
}

pub(super) fn label_width(text: &str) -> Result<f64, String> {
    label_width_at(text, FONT_SIZE)
}

pub(super) fn lanes(
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
) -> Result<(Vec<f64>, Vec<f64>, f64), String> {
    let widths: Vec<f64> = diagram
        .participants
        .iter()
        .map(|p| {
            label_width_at(
                &head_label(diagram, p),
                styles::node(sheet, &p.id, &p.classes).font_size,
            )
            .and_then(|width| {
                let mut width = (width + 32.0).max(100.0);
                for child in diagram.decompositions.iter().filter(|d| d.parent == p.id) {
                    let child_sheet = sheet.with_ancestor(&p.id, &p.classes);
                    width = width.max(lanes(&child.diagram, &child_sheet)?.2 + 32.0);
                }
                Ok(width)
            })
        })
        .collect::<Result<_, _>>()?;
    let mut longest = 0.0_f64;
    for event in &diagram.events {
        let width = match event {
            Event::Message(m) => m
                .label
                .as_deref()
                .map(|label| {
                    label_width_at(
                        label,
                        styles::message(sheet, m.name.as_deref(), &m.classes).font_size,
                    )
                })
                .transpose()?,
            Event::Annotation(annotation) => Some(label_width_at(
                &annotation.text,
                styles::annotation(sheet, styles::annotation_kind(annotation.kind)).font_size,
            )?),
            Event::Reference { label, .. } => Some(label_width_at(
                label,
                styles::fragment(sheet, "ref", None).font_size,
            )?),
            Event::Continuation(cont) => Some(
                label_width_at(&cont.text, styles::continuation(sheet, &cont.id).font_size)? + 32.0,
            ),
            Event::Note(note) => {
                Some(label_width_at(&note.text, styles::note(sheet, &note.id).font_size)? + 32.0)
            }
            Event::Activate {
                participant,
                id,
                label: Some(label),
                ..
            } => {
                let classes = &diagram
                    .participants
                    .iter()
                    .find(|p| p.id == *participant)
                    .ok_or_else(|| format!("unknown execution participant '{participant}'"))?
                    .classes;
                let size = styles::activation(sheet, participant, id.as_deref(), classes).font_size;
                Some(label_width_at(label, size)? + 20.0)
            }
            _ => None,
        };
        if let Some(width) = width {
            longest = longest.max(width);
        }
        if let Event::Message(m) = event {
            for endpoint in [&m.from, &m.to] {
                if let Some(gate) = gate_id(endpoint) {
                    let parent = styles::message(sheet, m.name.as_deref(), &m.classes);
                    let font = styles::gate(sheet, gate, &parent).font_size;
                    longest = longest.max(label_width_at(gate, font)? + 24.0);
                }
            }
        }
    }
    for event in &diagram.events {
        if let Event::FrameStart {
            operator, guard, ..
        } = event
        {
            longest = longest.max(label_width(&frame_caption(operator, guard))?);
        }
    }
    let gap = (longest + 36.0).max(72.0);
    let guard_pad = guard_padding(diagram, sheet, widths.first().copied().unwrap_or(0.0))?;
    let mut centers: Vec<f64> = Vec::with_capacity(widths.len());
    for (i, width) in widths.iter().enumerate() {
        let mut x = if i == 0 {
            32.0 + width / 2.0 + longest / 2.0 + guard_pad
        } else {
            centers[i - 1] + widths[i - 1] / 2.0 + gap + width / 2.0
        };
        for rule in &diagram.layout {
            if diagram.participants[i].id == rule.right {
                let from = diagram
                    .participants
                    .iter()
                    .position(|p| p.id == rule.left)
                    .ok_or_else(|| format!("unknown participant '{}'", rule.left))?;
                x = x.max(centers[from] + widths[from] / 2.0 + rule.gap + width / 2.0);
            }
        }
        centers.push(x);
    }
    let right = centers
        .last()
        .zip(widths.last())
        .map(|(x, width)| x + width / 2.0)
        .unwrap_or(32.0);
    Ok((centers, widths, (right + longest + 64.0).max(128.0)))
}

fn guard_padding(
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    first_width: f64,
) -> Result<f64, String> {
    let mut max = 0.0_f64;
    let mut open: Vec<(String, Option<String>)> = Vec::new();
    for event in &diagram.events {
        match event {
            Event::FrameStart {
                operator,
                id,
                guard,
                ..
            } => {
                open.push((operator.clone(), id.clone()));
                if !guard.is_empty() {
                    let font =
                        styles::fragment(sheet, operator_name(operator), id.as_deref()).font_size;
                    max = max.max(
                        label_width_at(operator, font)?
                            + 30.0
                            + label_width_at(&format!("[{guard}]"), font)?,
                    );
                }
            }
            Event::FrameBranch { guard, .. } if !guard.is_empty() => {
                let (operator, id) = open.last().ok_or("fragment branch without frame")?;
                let font =
                    styles::fragment(sheet, operator_name(operator), id.as_deref()).font_size;
                max = max.max(10.0 + label_width_at(&format!("[{guard}]"), font)?);
            }
            Event::FrameEnd { .. } => {
                open.pop();
            }
            _ => {}
        }
    }
    Ok((max + 24.0 - first_width / 2.0 - 16.0).max(0.0))
}

pub(super) fn frame_left(
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    centers: &[f64],
    widths: &[f64],
) -> Result<f64, String> {
    let width = widths.first().copied().unwrap_or(0.0);
    let pad = guard_padding(diagram, sheet, width)?;
    Ok(centers
        .first()
        .map(|x| x - width / 2.0 - 16.0 - pad)
        .unwrap_or(16.0))
}

fn coverage_span(
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    centers: &[f64],
    widths: &[f64],
    id: &str,
) -> Result<Option<(f64, f64)>, String> {
    let Some(coverage) = diagram.frame_layout.iter().find(|p| p.id == id) else {
        return Ok(None);
    };
    let indices: Vec<_> = coverage
        .participants
        .iter()
        .map(|name| {
            diagram
                .participants
                .iter()
                .position(|p| p.id == *name)
                .ok_or_else(|| format!("unknown frame participant '{name}'"))
        })
        .collect::<Result<_, _>>()?;
    let first = *indices.iter().min().ok_or("empty frame coverage")?;
    let last = *indices.iter().max().ok_or("empty frame coverage")?;
    let pad = guard_padding(diagram, sheet, widths[first])?;
    let left = centers[first] - widths[first] / 2.0 - 16.0 - pad;
    let right = centers[last] + widths[last] / 2.0 + 16.0;
    if left < 0.0 {
        return Err("frame guard does not fit to the left of its lifelines".into());
    }
    if first > 0 && left < centers[first - 1] + widths[first - 1] / 2.0 + 8.0 {
        return Err("scoped frame overlaps a preceding lifeline; increase lane gap".into());
    }
    Ok(Some((left, right)))
}

pub(super) fn frame_bounds(
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    centers: &[f64],
    widths: &[f64],
    frame: &super::DrawnFrame,
    default_left: f64,
    default_right: f64,
) -> Result<(f64, f64), String> {
    let (left, mut right) = match frame.id.as_deref() {
        Some(id) => coverage_span(diagram, sheet, centers, widths, id)?
            .unwrap_or((default_left, default_right)),
        None => (default_left, default_right),
    };
    let font =
        styles::fragment(sheet, operator_name(&frame.operator), frame.id.as_deref()).font_size;
    right = right
        .max(left + label_width_at(&frame_caption(&frame.operator, &frame.guard), font)? + 52.0);
    for (guard, _) in &frame.separators {
        right = right.max(left + label_width_at(&format!("[{guard}]"), font)? + 28.0);
    }
    if let Some(id) = frame.id.as_deref()
        && let Some(coverage) = diagram.frame_layout.iter().find(|p| p.id == id)
    {
        let last = coverage
            .participants
            .iter()
            .filter_map(|name| diagram.participants.iter().position(|p| p.id == *name))
            .max()
            .ok_or("empty frame coverage")?;
        if last + 1 < centers.len() && right > centers[last + 1] - widths[last + 1] / 2.0 - 8.0 {
            return Err("scoped frame overlaps a following lifeline; increase lane gap".into());
        }
    }
    Ok((left, right))
}

pub(super) fn canvas_width(
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    centers: &[f64],
    widths: &[f64],
    base: f64,
) -> Result<f64, String> {
    let left = frame_left(diagram, sheet, centers, widths)?;
    let right = centers
        .last()
        .zip(widths.last())
        .map(|(x, w)| x + w / 2.0 + 16.0)
        .unwrap_or(112.0);
    let mut canvas = base.max(frame_right(diagram, left, right, sheet)? + 32.0);
    for coverage in &diagram.frame_layout {
        if let Some((scoped_left, scoped_right)) =
            coverage_span(diagram, sheet, centers, widths, &coverage.id)?
        {
            canvas = canvas.max(frame_right(diagram, scoped_left, scoped_right, sheet)? + 32.0);
        }
    }
    for event in &diagram.events {
        if let Event::Reference { label, .. } = event {
            let font = styles::fragment(sheet, "ref", None).font_size;
            canvas = canvas.max(left + label_width_at(label, font)? + 84.0);
        }
    }
    Ok(canvas)
}

pub(super) fn operator_name(caption: &str) -> &str {
    caption.split(['(', ' ', '{']).next().unwrap_or(caption)
}

pub(super) fn frame_caption(operator: &str, guard: &str) -> String {
    if guard.is_empty() {
        operator.into()
    } else if operator == "ref" {
        format!("ref {guard}")
    } else {
        format!("{operator} [{guard}]")
    }
}

pub(super) fn frame_right(
    diagram: &SequenceDiagram,
    left: f64,
    base_right: f64,
    sheet: &StyleSheet,
) -> Result<f64, String> {
    let mut right = base_right;
    let mut open: Vec<(String, Option<String>)> = Vec::new();
    for event in &diagram.events {
        match event {
            Event::FrameStart {
                operator,
                id,
                guard,
                ..
            } => {
                let size =
                    styles::fragment(sheet, operator_name(operator), id.as_deref()).font_size;
                right =
                    right.max(left + label_width_at(&frame_caption(operator, guard), size)? + 52.0);
                open.push((operator.clone(), id.clone()));
            }
            Event::FrameBranch { guard, .. } => {
                let (operator, id) = open.last().ok_or("fragment branch without frame")?;
                let size =
                    styles::fragment(sheet, operator_name(operator), id.as_deref()).font_size;
                right = right.max(left + label_width_at(&format!("[{guard}]"), size)? + 28.0);
            }
            Event::FrameEnd { .. } => {
                open.pop();
            }
            _ => {}
        }
    }
    Ok(right)
}

pub(super) fn x_of(diagram: &SequenceDiagram, centers: &[f64], id: &str) -> Result<f64, String> {
    let index = diagram
        .participants
        .iter()
        .position(|p| p.id == id)
        .ok_or_else(|| format!("unknown participant '{id}'"))?;
    Ok(centers[index])
}

pub(super) fn message_endpoints(
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    centers: &[f64],
    widths: &[f64],
    m: &super::Message,
) -> Result<(f64, f64), String> {
    let global_left = frame_left(diagram, sheet, centers, widths)?;
    let last = centers.len() - 1;
    let global_right = frame_right(
        diagram,
        global_left,
        centers[last] + widths[last] / 2.0 + 16.0,
        sheet,
    )?;
    let mut current = Vec::new();
    for event in &diagram.events {
        match event {
            Event::FrameStart {
                operator,
                id,
                guard,
                ..
            } => current.push((operator, id, guard)),
            Event::FrameEnd { .. } => {
                current.pop();
            }
            Event::Message(candidate) if std::ptr::eq(candidate, m) => break,
            _ => {}
        }
    }
    let (frame_left, frame_right) = if let Some((operator, id, guard)) = current.last() {
        let frame = super::DrawnFrame {
            operator: (*operator).clone(),
            id: (*id).clone(),
            guard: (*guard).clone(),
            top: 0.0,
            bottom: 0.0,
            separators: Vec::new(),
            messages: Vec::new(),
        };
        frame_bounds(
            diagram,
            sheet,
            centers,
            widths,
            &frame,
            global_left,
            global_right,
        )?
    } else {
        (global_left, global_right)
    };
    let start = if m.from == "?" {
        x_of(diagram, centers, &m.to)? - 48.0
    } else if let Some(id) = gate_id(&m.from) {
        super::gates::x(diagram, centers, m, id, frame_left, frame_right)?
    } else {
        x_of(diagram, centers, &m.from)?
    };
    let mut end = if m.to == "?" {
        start + 48.0
    } else if let Some(id) = gate_id(&m.to) {
        super::gates::x(diagram, centers, m, id, frame_left, frame_right)?
    } else {
        x_of(diagram, centers, &m.to)?
    };
    if let Some((index, _)) = diagram
        .participants
        .iter()
        .enumerate()
        .find(|(_, p)| p.id == m.to && p.created_line == Some(m.line))
    {
        end += if start < end {
            -widths[index] / 2.0
        } else {
            widths[index] / 2.0
        };
    }
    Ok((start, end))
}
