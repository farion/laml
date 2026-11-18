//! Directed messages have independent send/receive activation markers.
use super::messages::{
    close, extract_message_prefix, gate_id, head_tokens, parse_label, parse_participant,
    require_participant,
};
use super::{Event, Message, MessageKind, Participant, SequenceDiagram, error};
use crate::{
    lex::{find_arrow, split_once_top_level},
    parser::ParseError,
};

fn endpoint(text: &str) -> (&str, Option<char>) {
    match text.chars().last() {
        Some(marker @ ('+' | '-' | '!')) => (&text[..text.len() - 1], Some(marker)),
        _ => (text, None),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn parse(
    text: &str,
    line: usize,
    diagram: &mut SequenceDiagram,
    calls: &mut Vec<(String, String)>,
    active: &mut Vec<(String, usize)>,
    operand_scope: Option<(usize, usize)>,
    has_gate_frame: bool,
) -> Result<(), ParseError> {
    let arrow_at = find_arrow(text)
        .or_else(|| text.find(" <<- "))
        .ok_or_else(|| error(line, "expected sequence message arrow"))?;
    let (body, options) = extract_message_prefix(text, arrow_at, line)?;
    let (heads, label) = match split_once_top_level(body, ':') {
        Some((heads, label)) => (
            heads.trim(),
            Some(
                parse_label(&format!(":{label}"), line)?
                    .ok_or_else(|| error(line, "empty message label"))?,
            ),
        ),
        None => (body, None),
    };
    let words = head_tokens(heads);
    if words.len() != 3 {
        return Err(error(line, "expected message: A -> B"));
    }
    let (kind, reverse) = match words[1] {
        "->" => (MessageKind::Sync, false),
        "<-" => (MessageKind::Sync, true),
        "->>" => (MessageKind::Async, false),
        "<<-" => (MessageKind::Async, true),
        "<--" => (MessageKind::Reply, true),
        _ => return Err(error(line, "unknown sequence arrow")),
    };
    let (left_text, left_marker) = endpoint(words[0]);
    let (right_text, right_marker) = endpoint(words[2]);
    let participant = |text: &str| {
        if text == "?" || gate_id(text).is_some() {
            Ok((text.to_string(), text.to_string(), Vec::new()))
        } else {
            parse_participant(text, line)
        }
    };
    let (left, left_label, left_classes) = participant(left_text)?;
    let (right, right_label, right_classes) = participant(right_text)?;
    let (from, to, send_marker, receive_marker, target_label, target_classes) = if reverse {
        (
            &right,
            &left,
            right_marker,
            left_marker,
            left_label,
            left_classes,
        )
    } else {
        (
            &left,
            &right,
            left_marker,
            right_marker,
            right_label,
            right_classes,
        )
    };
    if [left.as_str(), right.as_str()]
        .iter()
        .all(|id| *id == "?" || gate_id(id).is_some())
    {
        return Err(error(line, "message needs at least one known participant"));
    }
    if (gate_id(&left).is_some() || gate_id(&right).is_some()) && !has_gate_frame {
        return Err(error(line, "gate endpoint requires an enclosing frame"));
    }
    for (id, marker) in [(from, send_marker), (to, receive_marker)] {
        if marker.is_some() && (id == "?" || gate_id(id).is_some()) {
            return Err(error(
                line,
                "unknown endpoint cannot have an activation marker",
            ));
        }
    }
    if send_marker == Some('!') {
        return Err(error(
            line,
            "deletion message must point to an existing target",
        ));
    }
    if [(left_text, &left), (right_text, &right)]
        .iter()
        .any(|(text, id)| text.contains('(') && diagram.participants.iter().any(|p| &p.id == *id))
    {
        return Err(error(
            line,
            "inline participant label requires a new lifeline",
        ));
    }
    let existed = diagram.participants.iter().any(|p| &p.id == to);
    let revives = receive_marker == Some('+')
        && diagram
            .participants
            .iter()
            .find(|p| &p.id == to)
            .is_some_and(|p| {
                operand_scope.is_some_and(|(frame, operand)| {
                    p.created_line
                        .is_some_and(|created| created > frame && created < operand)
                        && p.destroyed_line.is_some_and(|died| died < operand)
                })
            });
    let creates = receive_marker == Some('+') && (!existed || revives);
    if options.create && (!creates || kind != MessageKind::Sync) {
        return Err(error(
            line,
            "creation message requires -> or <- and a first target+",
        ));
    }
    if options.arrow.is_some() && kind != MessageKind::Reply {
        return Err(error(line, "arrow override applies only to replies"));
    }
    let deletes = receive_marker == Some('!');
    if deletes && (kind == MessageKind::Reply || options.create) {
        return Err(error(
            line,
            "deletion message must point to an existing target",
        ));
    }
    if creates {
        if revives {
            diagram
                .participants
                .iter_mut()
                .find(|p| &p.id == to)
                .expect("revived participant exists")
                .destroyed_line = None;
        } else {
            diagram.participants.push(Participant {
                id: to.clone(),
                label: target_label,
                classes: target_classes,
                line,
                created_line: Some(line),
                destroyed_line: None,
            });
        }
        diagram.births.push((to.clone(), line));
    }
    for id in [from, to] {
        if id != "?" && gate_id(id).is_none() {
            require_participant(diagram, id, line)?;
        }
    }
    if let Some(name) = &options.name
        && diagram.events.iter().any(
            |event| matches!(event, Event::Message(message) if message.name.as_ref() == Some(name)),
        )
    {
        return Err(error(line, format!("duplicate message '{name}'")));
    }
    if kind == MessageKind::Sync
        && !deletes
        && !options.create
        && from != "?"
        && to != "?"
        && gate_id(from).is_none()
        && gate_id(to).is_none()
    {
        calls.push((from.clone(), to.clone()));
    }
    if kind == MessageKind::Reply
        && calls
            .last()
            .is_some_and(|(caller, callee)| caller == to && callee == from)
    {
        calls.pop();
    }
    let mut changes = Vec::new();
    // Source changes precede receiver changes, including two ends of a self-call.
    for (target, marker, at_send) in [(from, send_marker, true), (to, receive_marker, false)] {
        match marker {
            Some('+') => {
                active.push((target.clone(), line));
                changes.push(Event::Activate {
                    participant: target.clone(),
                    id: None,
                    label: None,
                    at_send,
                    line,
                });
            }
            Some('-') => {
                close(active, target, line)?;
                changes.push(Event::Deactivate {
                    participant: target.clone(),
                    id: None,
                    at_send,
                    line,
                });
            }
            Some('!') => {
                let participant = diagram
                    .participants
                    .iter_mut()
                    .find(|p| &p.id == target)
                    .ok_or_else(|| error(line, "deletion requires an existing lifeline"))?;
                participant.destroyed_line = Some(line);
                active.retain(|(name, _)| name != target);
                calls.retain(|(a, b)| a != target && b != target);
                changes.push(Event::Destroy {
                    participant: target.clone(),
                    line,
                });
            }
            _ => {}
        }
    }
    diagram.events.push(Event::Message(Message {
        name: options.name,
        classes: options.classes,
        from: from.clone(),
        to: to.clone(),
        label,
        kind: if options.create {
            MessageKind::Create
        } else if deletes {
            MessageKind::Delete
        } else {
            kind
        },
        filled_reply: options.arrow.unwrap_or(false),
        line,
    }));
    diagram.events.extend(changes);
    Ok(())
}
