//! Optional local IDs and labels for individual execution bars.

use super::{
    Event, SequenceDiagram, error,
    messages::{close, parse_participant, require_participant},
};
use crate::lex::{quoted_string, split_once_top_level, valid_identifier};
use crate::parser::ParseError;

pub(super) struct Command {
    pub name: Option<String>,
    pub participant: String,
    pub head: String,
    pub declared_attributes: bool,
    pub classes: Vec<String>,
    pub label: Option<String>,
    pub open: bool,
}

pub(super) fn parse(text: &str, line: usize) -> Result<Option<Command>, ParseError> {
    if [" -> ", " <- ", " ->> ", " <<- ", " <-- "]
        .iter()
        .any(|arrow| text.contains(arrow))
    {
        return Ok(None);
    }
    let (name, body) = match split_once_top_level(text, ':') {
        Some((prefix, rest)) if valid_identifier(prefix.trim()) => {
            (Some(prefix.trim().to_string()), rest.trim())
        }
        _ => (None, text),
    };
    let (subject, raw_label) = match split_once_top_level(body, ':') {
        Some((subject, label)) => (subject.trim(), Some(label.trim())),
        None => (body.trim(), None),
    };
    if subject.starts_with("<--") {
        return Ok(None);
    }
    let open = subject.ends_with('+');
    if !open && !subject.ends_with('-') {
        return Ok(None);
    }
    let label = raw_label
        .map(|raw| quoted_string(raw).map_err(|msg| error(line, msg)))
        .transpose()?;
    if label.as_deref() == Some("") {
        return Err(error(line, "execution label must not be empty"));
    }
    let head = subject[..subject.len() - 1].trim();
    let (participant, display, classes) = parse_participant(head, line)?;
    if !open && label.is_some() {
        return Err(error(line, "closed execution cannot have a label"));
    }
    Ok(Some(Command {
        name,
        participant,
        head: display,
        declared_attributes: head.contains('('),
        classes,
        label,
        open,
    }))
}

pub(super) fn apply(
    command: Command,
    line: usize,
    diagram: &mut SequenceDiagram,
    active: &mut Vec<(String, usize)>,
    operand_scope: Option<(usize, usize)>,
) -> Result<(), ParseError> {
    let Command {
        name,
        participant: id,
        head: label,
        declared_attributes,
        classes,
        label: text,
        open,
    } = command;
    let exists = diagram.participants.iter().any(|p| p.id == id);
    let revives = open
        && diagram
            .participants
            .iter()
            .find(|p| p.id == id)
            .is_some_and(|p| {
                operand_scope.is_some_and(|(frame, operand)| {
                    p.created_line
                        .is_some_and(|created| created > frame && created < operand)
                        && p.destroyed_line.is_some_and(|died| died < operand)
                })
            });
    if exists && declared_attributes {
        return Err(error(line, "participant attributes were already declared"));
    }
    if open && (!exists || revives) {
        if revives {
            diagram
                .participants
                .iter_mut()
                .find(|p| p.id == id)
                .expect("revived participant exists")
                .destroyed_line = None;
        } else {
            diagram.participants.push(super::Participant {
                id: id.clone(),
                label,
                classes,
                line,
                created_line: Some(line),
                destroyed_line: None,
            });
        }
        diagram.births.push((id.clone(), line));
    }
    require_participant(diagram, &id, line)?;
    if open {
        if let Some(name) = &name && diagram.events.iter().any(|event| matches!(event,
            Event::Activate { participant, id: Some(existing), .. } if participant == &id && existing == name)) {
            return Err(error(line, format!("duplicate execution '{name}' on '{id}'")));
        }
        active.push((id.clone(), line));
        diagram.events.push(Event::Activate {
            participant: id,
            id: name,
            label: text,
            line,
        });
    } else {
        let (_, opened) = active
            .iter()
            .rev()
            .find(|(p, _)| p == &id)
            .ok_or_else(|| error(line, format!("unmatched execution close for '{id}'")))?;
        let actual = diagram.events.iter().find_map(|event| match event {
            Event::Activate {
                participant,
                id: Some(name),
                line,
                ..
            } if participant == &id && line == opened => Some(name.clone()),
            _ => None,
        });
        if let Some(expected) = &name
            && actual.as_ref() != Some(expected)
        {
            return Err(error(
                line,
                format!("execution '{expected}' is not the open top bar on '{id}'"),
            ));
        }
        close(active, &id, line)?;
        diagram.events.push(Event::Deactivate {
            participant: id,
            id: name.or(actual),
            line,
        });
    }
    Ok(())
}
