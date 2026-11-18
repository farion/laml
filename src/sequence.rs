//! Sequence-diagram parsing and drawable structural checks.
//! Never feed these messages to the graph-diagram router.
use crate::lex::{quoted_string, split_once_top_level, valid_identifier};
use crate::parser::ParseError;
use crate::syntax::StyleEntry;
pub mod activation_layout;
mod annotations;
mod continuations;
mod coregions;
mod decompositions;
pub mod destruction_layout;
mod document;
mod executions;
mod fragments;
mod frame_layout;
mod identities;
mod layout;
mod layout_scoped;
mod message_parse;
mod messages;
mod notes;
pub mod occurrence_layout;
pub mod render;
pub(crate) mod styling;
pub use annotations::{
    Annotation, End as AnnotationEnd, Kind as AnnotationKind, OrderingEnd, OrderingRule,
};
pub use continuations::{Continuation, Placement as ContinuationPlacement};
pub use coregions::{Coregion, VerticalRule};
pub use decompositions::Decomposition;
pub use document::parse;
pub use frame_layout::FrameCoverage;
use messages::{
    MessageOptions, parse_label, parse_participant, parse_relation_prefix, require_participant,
};
pub use notes::{Note, Placement as NotePlacement};
pub use styling::is_sequence_source;
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MessageKind {
    Sync,
    Async,
    Reply,
    Create,
    Delete,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
        at_send: bool,
        line: usize,
    },
    Deactivate {
        participant: String,
        id: Option<String>,
        at_send: bool,
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
    pub theme: crate::theme::Theme,
    pub participants: Vec<Participant>,
    pub births: Vec<(String, usize)>,
    pub events: Vec<Event>,
    pub layout: Vec<LaneRule>,
    pub note_layout: Vec<NotePlacement>,
    pub continuation_layout: Vec<ContinuationPlacement>,
    pub frame_layout: Vec<FrameCoverage>,
    pub semantic_check_off: bool,
    pub allow_diagonal: bool,
    pub decompositions: Vec<Decomposition>,
    pub coregions: Vec<Coregion>,
    pub vertical_layout: Vec<VerticalRule>,
    pub destruction_layout: Vec<destruction_layout::DestructionAlignment>,
    pub activation_layout: Vec<activation_layout::Alignment>,
    pub occurrence_layout: Vec<occurrence_layout::Alignment>,
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
                at_send: true,
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
    message_parse::parse(
        text,
        line,
        diagram,
        calls,
        active,
        operand_scope,
        has_gate_frame,
    )
}
