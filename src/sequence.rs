//! Sequence-diagram parsing and drawable structural checks.
//! Never feed these messages to the graph-diagram router.

use crate::lex::{find_arrow, quoted_string, split_once_top_level, valid_identifier};
use crate::parser::ParseError;
use crate::syntax::StyleEntry;

mod annotations;
mod continuations;
mod coregions;
mod decompositions;
mod document;
mod executions;
mod fragments;
mod frame_layout;
mod identities;
mod layout;
mod layout_scoped;
mod messages;
mod notes;
pub mod render;
mod styling;
pub use styling::is_sequence_source;

pub use annotations::{
    Annotation, End as AnnotationEnd, Kind as AnnotationKind, OrderingEnd, OrderingRule,
};
pub use continuations::{Continuation, Placement as ContinuationPlacement};
pub use coregions::{Coregion, VerticalRule};
pub use decompositions::Decomposition;
pub use document::parse;
pub use frame_layout::FrameCoverage;
use messages::{
    MessageOptions, close, gate_id, head_tokens, parse_label, parse_participant,
    parse_relation_prefix, require_participant,
};
pub use notes::{Note, Placement as NotePlacement};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Participant {
    pub id: String,
    pub label: String,
    pub classes: Vec<String>,
    pub line: usize,
    /// A first `+` without an earlier declaration starts the visible lifeline here.
    pub created_line: Option<usize>,
    pub destroyed_line: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind {
    Sync,
    Async,
    Reply,
    Create,
    Delete,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub name: Option<String>,
    pub classes: Vec<String>,
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub kind: MessageKind,
    pub filled_reply: bool,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Message(Message),
    Note(Note),
    Continuation(Continuation),
    Decomposition {
        index: usize,
        line: usize,
    },
    Annotation(Annotation),
    Ordering(OrderingRule),
    Reference {
        label: String,
        line: usize,
    },
    FrameStart {
        operator: String,
        id: Option<String>,
        guard: String,
        line: usize,
    },
    FrameBranch {
        guard: String,
        line: usize,
    },
    FrameEnd {
        line: usize,
    },
    Destroy {
        participant: String,
        line: usize,
    },
    Activate {
        participant: String,
        id: Option<String>,
        label: Option<String>,
        line: usize,
    },
    Deactivate {
        participant: String,
        id: Option<String>,
        line: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct LaneRule {
    pub left: String,
    pub right: String,
    pub gap: f64,
    pub line: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SequenceDiagram {
    pub participants: Vec<Participant>,
    pub births: Vec<(String, usize)>,
    pub events: Vec<Event>,
    pub layout: Vec<LaneRule>,
    pub note_layout: Vec<NotePlacement>,
    pub continuation_layout: Vec<ContinuationPlacement>,
    pub frame_layout: Vec<FrameCoverage>,
    pub semantic_check_off: bool,
    pub decompositions: Vec<Decomposition>,
    pub coregions: Vec<Coregion>,
    pub vertical_layout: Vec<VerticalRule>,
    pub styling: Vec<StyleEntry>,
}

pub(super) fn error(line: usize, message: impl Into<String>) -> ParseError {
    ParseError {
        line,
        message: message.into(),
    }
}

pub(super) fn parse_semantic(
    text: &str,
    line: usize,
    diagram: &mut SequenceDiagram,
    calls: &mut Vec<(String, String)>,
    active: &mut Vec<(String, usize)>,
    operand_scope: Option<(usize, usize)>,
    has_gate_frame: bool,
) -> Result<(), ParseError> {
    if let Some(cont) = continuations::parse(text, line)? {
        diagram.events.push(Event::Continuation(cont));
        return Ok(());
    }
    if let Some(note) = notes::parse(text, line)? {
        diagram.events.push(Event::Note(note));
        return Ok(());
    }
    if let Some(id) = text.strip_suffix('!')
        && valid_identifier(id.trim())
    {
        let id = id.trim();
        require_participant(diagram, id, line)?;
        let participant = diagram
            .participants
            .iter_mut()
            .find(|p| p.id == id)
            .expect("participant checked");
        participant.destroyed_line = Some(line);
        active.retain(|(name, _)| name != id);
        calls.retain(|(from, to)| from != id && to != id);
        diagram.events.push(Event::Destroy {
            participant: id.into(),
            line,
        });
        return Ok(());
    }
    if let Some(command) = executions::parse(text, line)? {
        return executions::apply(command, line, diagram, active, operand_scope);
    }
    if let Some(order) = annotations::parse_order(text, line)? {
        diagram.events.push(Event::Ordering(order));
        return Ok(());
    }
    if let Some(annotation) = annotations::parse_line(text, line)? {
        diagram.events.push(Event::Annotation(annotation));
        return Ok(());
    }
    if let Some(reference) = text.strip_prefix("ref ") {
        let label = quoted_string(reference.trim()).map_err(|msg| error(line, msg))?;
        if label.is_empty() {
            return Err(error(line, "reference label cannot be empty"));
        }
        diagram.events.push(Event::Reference { label, line });
        return Ok(());
    }
    let (short_options, short_body) = match split_once_top_level(text, ':') {
        Some((prefix, body)) if body.trim().starts_with("<--") => {
            (parse_relation_prefix(prefix, line)?, body.trim())
        }
        _ => (MessageOptions::default(), text),
    };
    if let Some(rest) = short_body.strip_prefix("<--") {
        if short_options.create {
            return Err(error(line, "reply cannot be a creation message"));
        }
        if let Some(name) = &short_options.name
            && diagram
                .events
                .iter()
                .any(|event| matches!(event, Event::Message(m) if m.name.as_ref() == Some(name)))
        {
            return Err(error(line, format!("duplicate message '{name}'")));
        }
        let (to, from) = calls
            .pop()
            .ok_or_else(|| error(line, "reply has no open call"))?;
        let label = parse_label(rest, line)?;
        let closes_execution = active.last().is_some_and(|(id, _)| id == &from);
        if closes_execution {
            active.pop();
        }
        diagram.events.push(Event::Message(Message {
            name: short_options.name,
            classes: short_options.classes,
            from: from.clone(),
            to,
            label,
            kind: MessageKind::Reply,
            filled_reply: short_options.arrow.unwrap_or(false),
            line,
        }));
        if closes_execution {
            diagram.events.push(Event::Deactivate {
                participant: from,
                id: None,
                line,
            });
        }
        return Ok(());
    }
    if !text.contains(" -> ")
        && !text.contains(" <- ")
        && !text.contains(" ->> ")
        && !text.contains(" <<- ")
        && !text.contains(" <-- ")
    {
        let (id, label, classes) = parse_participant(text, line)?;
        if diagram.participants.iter().any(|p| p.id == id) {
            return Err(error(line, format!("duplicate participant '{id}'")));
        }
        diagram.participants.push(Participant {
            id,
            label,
            classes,
            line,
            created_line: None,
            destroyed_line: None,
        });
        return Ok(());
    }
    let arrow_at = find_arrow(text)
        .or_else(|| text.find(" <<- "))
        .ok_or_else(|| error(line, "expected sequence message arrow"))?;
    let (body, options) = messages::extract_message_prefix(text, arrow_at, line)?;
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
    let (left_raw, right_raw) = (words[0], words[2]);
    let (kind, reverse) = match words[1] {
        "->" => (MessageKind::Sync, false),
        "<-" => (MessageKind::Sync, true),
        "->>" => (MessageKind::Async, false),
        "<<-" => (MessageKind::Async, true),
        "<--" => (MessageKind::Reply, true),
        _ => return Err(error(line, "unknown sequence arrow")),
    };
    let (left_text, left_marker) = match left_raw.chars().last() {
        Some('+' | '-' | '!') if reverse => {
            (&left_raw[..left_raw.len() - 1], left_raw.chars().last())
        }
        _ => (left_raw, None),
    };
    let (right_text, right_marker) = match right_raw.chars().last() {
        Some('+' | '-' | '!') => (&right_raw[..right_raw.len() - 1], right_raw.chars().last()),
        _ => (right_raw, None),
    };
    if left_marker.is_some() && right_marker.is_some() {
        return Err(error(
            line,
            "one message cannot change executions on both ends",
        ));
    }
    let marker = left_marker.or(right_marker);
    let (left_id, left_label, left_classes) = if left_marker.is_some() {
        parse_participant(left_text, line)?
    } else {
        (left_text.into(), left_text.into(), Vec::new())
    };
    let left = left_id.as_str();
    let (right_id, right_label, right_classes) =
        if right_text == "?" || gate_id(right_text).is_some() {
            (right_text.to_string(), right_text.to_string(), Vec::new())
        } else {
            parse_participant(right_text, line)?
        };
    let right = right_id.as_str();
    let target = if left_marker.is_some() { left } else { right };
    let receives_marker =
        (left_marker.is_some() && reverse) || (right_marker.is_some() && !reverse);
    if (left == "?" || gate_id(left).is_some()) && (right == "?" || gate_id(right).is_some()) {
        return Err(error(line, "message needs at least one known participant"));
    }
    if (gate_id(left).is_some() || gate_id(right).is_some()) && !has_gate_frame {
        return Err(error(line, "gate endpoint requires an enclosing frame"));
    }
    if marker.is_some() && (target == "?" || gate_id(target).is_some()) {
        return Err(error(
            line,
            "unknown endpoint cannot have an activation marker",
        ));
    }
    if (right_text.contains('(') && diagram.participants.iter().any(|p| p.id == right))
        || (left_marker.is_some()
            && left_text.contains('(')
            && diagram.participants.iter().any(|p| p.id == left))
    {
        return Err(error(
            line,
            "inline participant label requires a new lifeline",
        ));
    }
    let existed = diagram.participants.iter().any(|p| p.id == target);
    let revives = marker == Some('+')
        && receives_marker
        && diagram
            .participants
            .iter()
            .find(|p| p.id == target)
            .is_some_and(|p| {
                operand_scope.is_some_and(|(frame, operand)| {
                    p.created_line
                        .is_some_and(|created| created > frame && created < operand)
                        && p.destroyed_line.is_some_and(|died| died < operand)
                })
            });
    let creates = marker == Some('+') && receives_marker && (!existed || revives);
    if options.create && (!creates || kind != MessageKind::Sync) {
        return Err(error(
            line,
            "creation message requires -> or <- and a first target+",
        ));
    }
    if options.arrow.is_some() && kind != MessageKind::Reply {
        return Err(error(line, "arrow override applies only to replies"));
    }
    let deletes = marker == Some('!');
    if deletes && (!receives_marker || kind == MessageKind::Reply || options.create) {
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
                .find(|p| p.id == target)
                .expect("revived participant exists")
                .destroyed_line = None;
        } else {
            diagram.participants.push(Participant {
                id: target.into(),
                label: if left_marker.is_some() {
                    left_label
                } else {
                    right_label
                },
                classes: if left_marker.is_some() {
                    left_classes
                } else {
                    right_classes
                },
                line,
                created_line: Some(line),
                destroyed_line: None,
            });
        }
        diagram.births.push((target.into(), line));
    }
    for id in [left, right] {
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
    let (from, to) = if reverse {
        (right, left)
    } else {
        (left, right)
    };
    if kind == MessageKind::Sync
        && !deletes
        && !options.create
        && from != "?"
        && to != "?"
        && gate_id(from).is_none()
        && gate_id(to).is_none()
    {
        calls.push((from.into(), to.into()));
    }
    if kind == MessageKind::Reply
        && calls
            .last()
            .is_some_and(|(caller, callee)| caller == to && callee == from)
    {
        calls.pop();
    }
    match marker {
        Some('+') => active.push((target.into(), line)),
        Some('-') => close(active, target, line)?,
        Some('!') => {
            let participant = diagram
                .participants
                .iter_mut()
                .find(|p| p.id == target)
                .ok_or_else(|| error(line, "deletion requires an existing lifeline"))?;
            participant.destroyed_line = Some(line);
            active.retain(|(name, _)| name != target);
            calls.retain(|(from, to)| from != target && to != target);
        }
        _ => {}
    }
    diagram.events.push(Event::Message(Message {
        name: options.name,
        classes: options.classes,
        from: from.into(),
        to: to.into(),
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
    match marker {
        Some('+') => diagram.events.push(Event::Activate {
            participant: target.into(),
            id: None,
            label: None,
            line,
        }),
        Some('-') => diagram.events.push(Event::Deactivate {
            participant: target.into(),
            id: None,
            line,
        }),
        Some('!') => diagram.events.push(Event::Destroy {
            participant: target.into(),
            line,
        }),
        _ => {}
    }
    Ok(())
}
