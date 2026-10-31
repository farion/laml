//! Explicit sequence IDs share one diagram namespace, while execution IDs remain lifeline-scoped.
use super::{Event, SequenceDiagram, error};
use crate::parser::ParseError;
use std::collections::HashMap;

pub(super) fn validate(diagram: &SequenceDiagram) -> Result<(), ParseError> {
    let mut ids: HashMap<String, usize> = HashMap::new();
    let mut check = |id: &str, line: usize| -> Result<(), ParseError> {
        if let Some(prior) = ids.insert(id.to_string(), line) {
            return Err(error(
                line,
                format!("duplicate sequence diagram ID '{id}' (first at line {prior})"),
            ));
        }
        Ok(())
    };
    for participant in &diagram.participants {
        check(&participant.id, participant.line)?;
    }
    for event in &diagram.events {
        match event {
            Event::Message(message) => {
                if let Some(id) = &message.name {
                    check(id, message.line)?;
                }
            }
            Event::Note(note) => check(&note.id, note.line)?,
            Event::Continuation(continuation) => check(&continuation.id, continuation.line)?,
            Event::FrameStart {
                id: Some(id), line, ..
            } => check(id, *line)?,
            _ => {}
        }
    }
    Ok(())
}
