//! Keep all uses of a named gate on one side of its enclosing frame.

use super::{Event, Message, SequenceDiagram, geometry::x_of};
use crate::sequence::messages::gate_id;

fn frame_start(diagram: &SequenceDiagram, current: &Message) -> Result<usize, String> {
    let current_index = diagram
        .events
        .iter()
        .position(|event| matches!(event, Event::Message(m) if std::ptr::eq(m, current)))
        .ok_or("missing gate message")?;
    let mut stack = Vec::new();
    for (index, event) in diagram.events[..=current_index].iter().enumerate() {
        match event {
            Event::FrameStart { .. } => stack.push(index),
            Event::FrameEnd { .. } => {
                stack.pop();
            }
            _ => {}
        }
    }
    stack
        .last()
        .copied()
        .ok_or_else(|| "gate outside frame".into())
}

pub(super) fn align(
    diagram: &SequenceDiagram,
    messages: &[(&Message, f64)],
    anchors: &mut [super::anchors::MessageAnchor],
) -> Result<(), String> {
    for (index, (message, _)) in messages.iter().enumerate() {
        let id = gate_id(&message.from).or_else(|| gate_id(&message.to));
        let Some(id) = id else { continue };
        let frame = frame_start(diagram, message)?;
        let earlier: Vec<_> = messages[..index]
            .iter()
            .enumerate()
            .filter(|(_, (prior, _))| {
                gate_id(&prior.from) == Some(id) || gate_id(&prior.to) == Some(id)
            })
            .filter_map(|(i, (prior, _))| {
                frame_start(diagram, prior)
                    .ok()
                    .filter(|at| *at == frame)
                    .map(|_| i)
            })
            .collect();
        if earlier.is_empty() {
            continue;
        }
        if earlier.len() != 1 {
            return Err(format!("gate '{id}' has too many connections"));
        }
        let prior = messages[earlier[0]].0;
        if gate_id(&prior.from).is_some() == gate_id(&message.from).is_some() {
            return Err(format!(
                "gate '{id}' must connect one incoming and one outgoing message"
            ));
        }
        let at = if gate_id(&prior.from).is_some() {
            anchors[earlier[0]].send.1
        } else {
            anchors[earlier[0]].receive.1
        };
        anchors[index].send.1 = at;
        anchors[index].receive.1 = at;
    }
    Ok(())
}

pub(super) fn x(
    diagram: &SequenceDiagram,
    centers: &[f64],
    current: &Message,
    id: &str,
    left: f64,
    right: f64,
) -> Result<f64, String> {
    let start = frame_start(diagram, current)?;
    let (mut depth, mut outside_left, mut outside_right) = (0_usize, false, false);
    let mut first_inside = None;
    for event in &diagram.events[start..] {
        match event {
            Event::FrameStart { .. } => depth += 1,
            Event::FrameEnd { .. } => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Event::Message(m) if depth == 1 => {
                let other = if gate_id(&m.from) == Some(id) {
                    Some(m.to.as_str())
                } else if gate_id(&m.to) == Some(id) {
                    Some(m.from.as_str())
                } else {
                    None
                };
                if let Some(other) = other {
                    let at = x_of(diagram, centers, other)?;
                    outside_left |= at < left;
                    outside_right |= at > right;
                    first_inside.get_or_insert(at);
                }
            }
            _ => {}
        }
    }
    if outside_left && outside_right {
        return Err(format!(
            "gate '{id}' cannot connect to both sides of a frame"
        ));
    }
    if outside_left {
        return Ok(left);
    }
    if outside_right {
        return Ok(right);
    }
    Ok(
        if first_inside.unwrap_or((left + right) / 2.0) < (left + right) / 2.0 {
            left
        } else {
            right
        },
    )
}
