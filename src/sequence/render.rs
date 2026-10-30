//! First sequence renderer: lifelines, messages, self-calls and executions.
//! This does not claim complete UML 2.5.1 coverage.

mod anchors;
mod annotations;
mod arrows;
mod continuations;
mod coregions;
mod decompositions;
mod executions;
mod frames;
mod gates;
mod geometry;
mod lifetimes;
mod messages;
mod notes;
mod order;
mod retime;
mod styles;
use super::{Event, Message, MessageKind, SequenceDiagram};
use crate::styles::StyleSheet;
use crate::svg::TextMode;
use crate::text::FONT;
use anchors::MessageAnchor;
use base64::Engine;
use executions::{DrawnExecution, OpenExecution};
use geometry::{frame_right, head_label, label_width_at, lanes, x_of};
use std::fmt::Write;
use styles::escape;

const FONT_SIZE: f64 = 14.0;
const HEADER_Y: f64 = 32.0;
const HEADER_HEIGHT: f64 = 48.0;

type Separator = (String, f64);

struct OpenFrame {
    operator: String,
    id: Option<String>,
    guard: String,
    top: f64,
    separators: Vec<Separator>,
    incoming_execution: Vec<OpenExecution>,
    messages: Vec<usize>,
}

struct DrawnFrame {
    operator: String,
    id: Option<String>,
    guard: String,
    top: f64,
    bottom: f64,
    separators: Vec<Separator>,
    messages: Vec<usize>,
}

fn styled_text(svg: &mut String, x: f64, y: f64, value: &str, size: f64, color: &str) {
    for (index, line) in value.split('\n').enumerate() {
        writeln!(svg, "<text x=\"{x:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"{size:.2}\" fill=\"{}\">{}</text>",
            y + index as f64 * size * 1.25, escape(color), escape(line)).unwrap();
    }
}
/// Render only the currently parsed subset. Other syntax must fail at parse time.
pub fn render(diagram: &SequenceDiagram, mode: TextMode) -> Result<String, String> {
    render_with_styles(diagram, &StyleSheet::default(), mode)
}

pub fn render_with_styles(
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    mode: TextMode,
) -> Result<String, String> {
    let (centers, widths, base_canvas_width) = lanes(diagram, sheet)?;
    let canvas_width =
        geometry::canvas_width(diagram, sheet, &centers, &widths, base_canvas_width)?;
    let nested = decompositions::prepare(diagram, sheet)?;
    let header_height = diagram
        .participants
        .iter()
        .map(|p| {
            let size = styles::node(sheet, &p.id, &p.classes).font_size;
            size * 1.25 * head_label(diagram, p).split('\n').count() as f64 + 20.0
        })
        .fold(HEADER_HEIGHT, f64::max);
    let header_bottom = HEADER_Y + header_height;
    let mut positioned = Vec::new();
    let mut references = Vec::new();
    let mut visible_annotations = Vec::new();
    let mut visible_orders = Vec::new();
    let mut visible_notes = Vec::new();
    let mut visible_continuations = Vec::new();
    let mut visible_decompositions = Vec::new();
    let mut destructions: Vec<(String, f64)> = Vec::new();
    let mut branch_starts = Vec::new();
    let mut event_positions: Vec<(usize, f64)> = Vec::new();
    let mut drawn_frames: Vec<DrawnFrame> = Vec::new();
    let mut frames: Vec<OpenFrame> = Vec::new();
    let mut bars: Vec<DrawnExecution> = Vec::new();
    let mut open: Vec<OpenExecution> = Vec::new();
    let mut y = header_bottom + 52.0;
    let mut previous_line = 0;
    for event in &diagram.events {
        let line = match event {
            Event::Message(m) => m.line,
            Event::Note(note) => note.line,
            Event::Continuation(cont) => cont.line,
            Event::Decomposition { line, .. } => *line,
            Event::Annotation(annotation) => annotation.line,
            Event::Ordering(rule) => rule.line,
            Event::Reference { line, .. }
            | Event::FrameStart { line, .. }
            | Event::FrameBranch { line, .. }
            | Event::FrameEnd { line }
            | Event::Destroy { line, .. }
            | Event::Activate { line, .. }
            | Event::Deactivate { line, .. } => *line,
        };
        if line != previous_line {
            if previous_line != 0 {
                y += 58.0;
            }
            previous_line = line;
        }
        match event {
            Event::Decomposition { index, .. } => {
                visible_decompositions.push((*index, y));
                y += nested
                    .get(*index)
                    .ok_or("missing inline decomposition")?
                    .height
                    + 24.0;
            }
            Event::Continuation(cont) => {
                let placement = diagram
                    .continuation_layout
                    .iter()
                    .find(|p| p.id == cont.id)
                    .ok_or("continuation has no placement")?;
                y += placement.gap;
                visible_continuations.push((cont, y));
                y += continuations::height(cont, sheet);
            }
            Event::Note(note) => {
                let placement = diagram
                    .note_layout
                    .iter()
                    .find(|p| p.note == note.id)
                    .ok_or("note has no placement")?;
                y += placement.gap;
                visible_notes.push((note, y));
                y += notes::height(note, sheet);
            }
            Event::Annotation(annotation) => {
                y = annotations::reserve_row(
                    diagram,
                    annotation,
                    &visible_annotations,
                    &event_positions,
                    sheet,
                    y,
                )?;
                visible_annotations.push(annotation);
            }
            Event::Ordering(rule) => visible_orders.push(rule),
            Event::Destroy { participant, .. } => {
                destructions.push((participant.clone(), y));
                let mut i = 0;
                while i < open.len() {
                    if open[i].participant == *participant {
                        bars.push(open.remove(i).finish(y));
                    } else {
                        i += 1;
                    }
                }
            }
            Event::FrameStart {
                operator,
                id,
                guard,
                ..
            } => {
                frames.push(OpenFrame {
                    operator: operator.clone(),
                    id: id.clone(),
                    guard: guard.clone(),
                    top: y - 22.0,
                    separators: Vec::new(),
                    incoming_execution: open.clone(),
                    messages: Vec::new(),
                });
                let font =
                    styles::fragment(sheet, geometry::operator_name(operator), id.as_deref())
                        .font_size;
                y += (frames::descriptor_height(font) - 26.0).max(0.0);
            }
            Event::FrameBranch { guard, .. } => {
                let frame = frames.last_mut().ok_or("fragment branch without frame")?;
                frame.separators.push((guard.clone(), y - 26.0));
                branch_starts.push(y - 26.0);
                executions::next_operand(&mut open, &frame.incoming_execution, y - 26.0);
                let font = styles::fragment(
                    sheet,
                    geometry::operator_name(&frame.operator),
                    frame.id.as_deref(),
                )
                .font_size;
                y += (frames::descriptor_height(font) - 26.0).max(0.0);
            }
            Event::FrameEnd { .. } => {
                let frame = frames.pop().ok_or("fragment end without frame")?;
                drawn_frames.push(DrawnFrame {
                    operator: frame.operator,
                    id: frame.id,
                    guard: frame.guard,
                    top: frame.top,
                    bottom: y - 24.0,
                    separators: frame.separators,
                    messages: frame.messages,
                });
            }
            Event::Reference { label, .. } => {
                references.push((label.as_str(), y));
                y += frames::reference_height(sheet) - 50.0;
            }
            Event::Message(m) => {
                let lines = m
                    .label
                    .as_deref()
                    .map(|text| text.split('\n').count())
                    .unwrap_or(1);
                if lines > 1 {
                    y += styles::message(sheet, m.name.as_deref(), &m.classes).font_size
                        * 1.25
                        * (lines - 1) as f64;
                }
                for frame in &mut frames {
                    frame.messages.push(positioned.len());
                }
                positioned.push((m, y));
                if !event_positions.iter().any(|(at, _)| *at == line) {
                    event_positions.push((line, y));
                }
                if m.from == m.to {
                    y += 24.0;
                }
            }
            Event::Activate {
                participant,
                id,
                label,
                ..
            } => {
                if !event_positions.iter().any(|(at, _)| *at == line) {
                    event_positions.push((line, y));
                }
                let depth = open
                    .iter()
                    .filter(|bar| bar.participant == *participant)
                    .count();
                let created_here = diagram
                    .participants
                    .iter()
                    .any(|p| p.id == *participant && p.created_line == Some(line));
                let from = if created_here {
                    let incoming = diagram.events.iter().any(|event| {
                        matches!(event,
                        Event::Message(m) if m.line == line && m.to == *participant)
                    });
                    y + if incoming {
                        header_height / 2.0
                    } else {
                        header_height
                    }
                } else {
                    y
                };
                open.push(OpenExecution {
                    participant: participant.clone(),
                    id: id.clone(),
                    label: label.clone(),
                    from,
                    depth,
                });
                if let Some(label) = label {
                    let classes = &diagram
                        .participants
                        .iter()
                        .find(|p| p.id == *participant)
                        .ok_or("unknown execution participant")?
                        .classes;
                    let font =
                        styles::activation(sheet, participant, id.as_deref(), classes).font_size;
                    let text_height = font * 1.25 * label.split('\n').count() as f64 + 12.0;
                    y = y.max(from + text_height - 58.0);
                    y += font * 1.25 * (label.split('\n').count().saturating_sub(1)) as f64;
                }
            }
            Event::Deactivate { participant, .. } => {
                let index = open
                    .iter()
                    .rposition(|bar| bar.participant == *participant)
                    .ok_or_else(|| format!("unmatched execution for '{participant}'"))?;
                bars.push(open.remove(index).finish(y));
            }
        }
    }
    let mut message_anchors = positioned
        .iter()
        .map(|(m, y)| {
            let (from, to) = geometry::message_endpoints(diagram, sheet, &centers, &widths, m)?;
            Ok(MessageAnchor {
                id: m.name.clone().unwrap_or_default(),
                send: (from, *y),
                receive: (to, if m.from == m.to { y + 24.0 } else { *y }),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    gates::align(diagram, &positioned, &mut message_anchors)?;
    for region in &diagram.coregions {
        let (threshold, delta) = coregions::reorder_region(diagram, region, &mut message_anchors)?;
        let mut timeline = retime::Shiftables {
            anchors: &mut message_anchors,
            frames: &mut drawn_frames,
            bars: &mut bars,
            open: &mut open,
            positions: &mut event_positions,
            deaths: &mut destructions,
            branches: &mut branch_starts,
            decompositions: &mut visible_decompositions,
            notes: &mut visible_notes,
            continuations: &mut visible_continuations,
            references: &mut references,
            final_y: &mut y,
        };
        timeline.expand(region, threshold, delta);
    }
    coregions::validate_downward(&message_anchors)?;
    for frame in &drawn_frames {
        for &index in &frame.messages {
            let anchor = &message_anchors[index];
            if anchor.send.1 < frame.top + 20.0 || anchor.receive.1 > frame.bottom - 8.0 {
                return Err("mandatory occurrence layout exceeds its fragment frame".into());
            }
        }
    }
    let bottom = message_anchors
        .iter()
        .fold(y, |max, a| max.max(a.send.1).max(a.receive.1))
        + 64.0;
    for bar in open {
        bars.push(bar.finish(bottom - 16.0));
    }
    let mut svg = String::new();
    writeln!(svg, "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{canvas_width:.2}\" height=\"{bottom:.2}\" viewBox=\"0 0 {canvas_width:.2} {bottom:.2}\">").unwrap();
    if mode == TextMode::Embedded {
        let font = base64::engine::general_purpose::STANDARD.encode(FONT);
        writeln!(svg, "<style>@font-face {{ font-family: 'Inter'; src: url('data:font/otf;base64,{font}') format('opentype'); }}</style>").unwrap();
    }
    svg.push_str("<defs><marker id=\"filled\" markerWidth=\"10\" markerHeight=\"8\" refX=\"9\" refY=\"4\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"><path d=\"M0,0 L10,4 L0,8 Z\" fill=\"#20252b\"/></marker><marker id=\"open\" markerWidth=\"10\" markerHeight=\"8\" refX=\"9\" refY=\"4\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"><path d=\"M0,0 L10,4 L0,8\" fill=\"none\" stroke=\"#20252b\" stroke-width=\"1.5\"/></marker></defs>\n");
    let timeline = lifetimes::Timeline {
        diagram,
        positions: &event_positions,
        deaths: &destructions,
        branches: &branch_starts,
        header_height,
        bottom,
    };
    lifetimes::draw_lines(&mut svg, sheet, &centers, &timeline)?;
    decompositions::draw(
        &mut svg,
        diagram,
        &centers,
        &nested,
        &visible_decompositions,
    )?;
    let frame_left = geometry::frame_left(diagram, sheet, &centers, &widths)?;
    let base_right = centers
        .last()
        .zip(widths.last())
        .map(|(x, w)| x + w / 2.0 + 16.0)
        .unwrap_or(112.0);
    let frame_width = frame_right(diagram, frame_left, base_right, sheet)? - frame_left;
    for frame in drawn_frames {
        let (left, right) = geometry::frame_bounds(
            diagram,
            sheet,
            &centers,
            &widths,
            &frame,
            frame_left,
            frame_left + frame_width,
        )?;
        frames::fragment(&mut svg, frame, left, right - left, sheet)?;
    }
    bars.sort_by_key(|bar| bar.opened.depth);
    executions::draw(&mut svg, diagram, sheet, &centers, &bars)?;
    let mut drawn_gates = Vec::new();
    for ((m, _), anchor) in positioned.into_iter().zip(&message_anchors) {
        messages::draw(&mut svg, sheet, m, anchor, &mut drawn_gates)?;
    }
    coregions::draw(&mut svg, diagram, &centers, &message_anchors, sheet)?;
    annotations::draw(
        &mut svg,
        &visible_annotations,
        &message_anchors,
        sheet,
        diagram,
        &centers,
        &bars,
    )?;
    order::draw(
        &mut svg,
        &visible_orders,
        &message_anchors,
        sheet,
        diagram,
        &centers,
        &bars,
    )?;
    for (id, at) in &destructions {
        let x = x_of(diagram, &centers, id)?;
        writeln!(svg, "<path d=\"M{:.2},{:.2} L{:.2},{:.2} M{:.2},{:.2} L{:.2},{:.2}\" stroke=\"#20252b\" stroke-width=\"2\" fill=\"none\" data-destroy=\"{}\" />",
            x - 7.0, at - 7.0, x + 7.0, at + 7.0,
            x - 7.0, at + 7.0, x + 7.0, at - 7.0, escape(id)).unwrap();
    }
    for (label, y) in references {
        let font = styles::fragment(sheet, "ref", None).font_size;
        let width = (base_right - frame_left).max(label_width_at(label, font)? + 52.0);
        frames::reference(&mut svg, label, frame_left, y, width, sheet)?;
    }
    continuations::draw(&mut svg, diagram, sheet, &centers, &visible_continuations)?;
    notes::draw(
        &mut svg,
        diagram,
        sheet,
        &centers,
        &visible_notes,
        &message_anchors,
    )?;
    lifetimes::draw_heads(&mut svg, sheet, &centers, &widths, &timeline)?;
    svg.push_str("</svg>\n");
    if mode == TextMode::Paths {
        crate::export::outlines(&svg)
    } else {
        Ok(svg)
    }
}
