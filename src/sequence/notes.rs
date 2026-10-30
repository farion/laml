//! Named notes: semantic text is independent of mandatory layout placement.

use super::{Event, SequenceDiagram, error};
use crate::lex::{quoted_string, split_once_top_level, valid_identifier};
use crate::parser::ParseError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub id: String,
    pub text: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub note: String,
    pub target: String,
    pub gap: f64,
    pub line: usize,
}

pub(super) fn parse(text: &str, line: usize) -> Result<Option<Note>, ParseError> {
    let Some((id, body)) = split_once_top_level(text, ':') else {
        return Ok(None);
    };
    let body = body.trim();
    if body != "note" && !body.starts_with("note ") {
        return Ok(None);
    }
    if !valid_identifier(id.trim()) {
        return Err(error(line, "invalid note identifier"));
    }
    let text = body.strip_prefix("note").unwrap().trim();
    let text = quoted_string(text).map_err(|msg| error(line, msg))?;
    if text.is_empty() {
        return Err(error(line, "note text must not be empty"));
    }
    Ok(Some(Note {
        id: id.trim().into(),
        text,
        line,
    }))
}

pub(super) fn parse_placement(text: &str, line: usize) -> Result<Option<Placement>, ParseError> {
    let (location, attributes) = match split_once_top_level(text, ':') {
        Some((location, attributes)) => (location.trim(), Some(attributes.trim())),
        None => (text, None),
    };
    let Some((id, participant)) = location.split_once(" over ") else {
        return Ok(None);
    };
    let (id, target) = (id.trim(), participant.trim());
    let valid_target = valid_identifier(target)
        || target
            .split_once('.')
            .is_some_and(|(id, end)| valid_identifier(id) && matches!(end, "send" | "receive"));
    if !valid_identifier(id) || !valid_target {
        return Err(error(
            line,
            "expected noteId over participant or message.send/receive",
        ));
    }
    let gap = match attributes {
        Some(attributes) => {
            let (key, value) = split_once_top_level(attributes, '=')
                .ok_or_else(|| error(line, "expected gap=number"))?;
            if key.trim() != "gap" {
                return Err(error(line, "only gap is supported for note layout"));
            }
            let number = value
                .trim()
                .parse::<f64>()
                .map_err(|_| error(line, "gap must be a number"))?;
            if !number.is_finite() || number < 0.0 {
                return Err(error(line, "gap must be finite and nonnegative"));
            }
            number
        }
        None => 16.0,
    };
    Ok(Some(Placement {
        note: id.into(),
        target: target.into(),
        gap,
        line,
    }))
}

pub(super) fn validate(diagram: &SequenceDiagram) -> Result<(), ParseError> {
    let notes: Vec<_> = diagram
        .events
        .iter()
        .filter_map(|event| {
            if let Event::Note(note) = event {
                Some(note)
            } else {
                None
            }
        })
        .collect();
    for (i, note) in notes.iter().enumerate() {
        if notes[..i].iter().any(|previous| previous.id == note.id)
            || diagram.participants.iter().any(|p| p.id == note.id)
            || diagram
                .events
                .iter()
                .any(|e| matches!(e, Event::Message(m) if m.name.as_deref() == Some(&note.id)))
        {
            return Err(error(
                note.line,
                format!("duplicate diagram identifier '{}'", note.id),
            ));
        }
        if !diagram.note_layout.iter().any(|p| p.note == note.id) {
            return Err(error(
                note.line,
                format!("note '{}' needs layout placement", note.id),
            ));
        }
    }
    for (i, place) in diagram.note_layout.iter().enumerate() {
        if !notes.iter().any(|note| note.id == place.note) {
            return Err(error(place.line, format!("unknown note '{}'", place.note)));
        }
        let note = notes
            .iter()
            .find(|note| note.id == place.note)
            .ok_or_else(|| error(place.line, "unknown note"))?;
        validate_target(diagram, place, note)?;
        if diagram.note_layout[..i]
            .iter()
            .any(|previous| previous.note == place.note)
        {
            return Err(error(
                place.line,
                format!("duplicate note placement '{}'", place.note),
            ));
        }
    }
    Ok(())
}

fn validate_target(
    diagram: &SequenceDiagram,
    place: &Placement,
    note: &Note,
) -> Result<(), ParseError> {
    if let Some((name, _end)) = place.target.split_once('.') {
        let matching: Vec<_> = diagram
            .events
            .iter()
            .filter_map(|event| {
                if let Event::Message(m) = event
                    && m.name.as_deref() == Some(name)
                {
                    Some(m)
                } else {
                    None
                }
            })
            .collect();
        if matching.len() != 1 || matching[0].line >= note.line {
            return Err(error(
                place.line,
                format!(
                    "note '{}' needs an unambiguous earlier named message '{}'",
                    note.id, name
                ),
            ));
        }
        return Ok(());
    }
    let participant = diagram
        .participants
        .iter()
        .find(|p| p.id == place.target)
        .ok_or_else(|| {
            error(
                place.line,
                format!("unknown participant '{}'", place.target),
            )
        })?;
    if participant
        .created_line
        .is_some_and(|line| note.line < line)
        || participant
            .destroyed_line
            .is_some_and(|line| note.line >= line)
    {
        return Err(error(
            place.line,
            format!("note '{}' references an inactive participant", note.id),
        ));
    }
    Ok(())
}
