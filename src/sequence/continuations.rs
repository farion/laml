//! Named continuation labels can span several participant lifelines.

use super::{Event, SequenceDiagram, error};
use crate::lex::{quoted_string, split_once_top_level, valid_identifier};
use crate::parser::ParseError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Continuation {
    pub id: String,
    pub text: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub id: String,
    pub participants: Vec<String>,
    pub gap: f64,
    pub line: usize,
}

pub(super) fn parse(text: &str, line: usize) -> Result<Option<Continuation>, ParseError> {
    let Some((id, body)) = split_once_top_level(text, ':') else {
        return Ok(None);
    };
    let Some(body) = body.trim().strip_prefix("continuation ") else {
        return Ok(None);
    };
    if !valid_identifier(id.trim()) {
        return Err(error(line, "invalid continuation identifier"));
    }
    let text = quoted_string(body.trim()).map_err(|msg| error(line, msg))?;
    if text.is_empty() {
        return Err(error(line, "continuation label cannot be empty"));
    }
    Ok(Some(Continuation {
        id: id.trim().into(),
        text,
        line,
    }))
}

pub(super) fn parse_placement(text: &str, line: usize) -> Result<Option<Placement>, ParseError> {
    let (placement, attributes) = match split_once_top_level(text, ':') {
        Some((placement, attributes)) => (placement.trim(), Some(attributes.trim())),
        None => (text, None),
    };
    let Some((id, participants)) = placement.split_once(" spans ") else {
        return Ok(None);
    };
    if !valid_identifier(id.trim()) {
        return Err(error(line, "invalid continuation identifier"));
    }
    let participants = participants
        .split(',')
        .map(str::trim)
        .map(|name| {
            if valid_identifier(name) {
                Ok(name.to_string())
            } else {
                Err(error(line, "invalid continuation participant"))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    if participants.is_empty()
        || participants
            .iter()
            .enumerate()
            .any(|(i, p)| participants[..i].contains(p))
    {
        return Err(error(line, "continuation needs distinct participants"));
    }
    let gap = match attributes {
        Some(attributes) => {
            let (key, value) = split_once_top_level(attributes, '=')
                .ok_or_else(|| error(line, "expected gap=number"))?;
            if key.trim() != "gap" {
                return Err(error(
                    line,
                    "only gap is supported for continuation placement",
                ));
            }
            let gap = value
                .trim()
                .parse::<f64>()
                .map_err(|_| error(line, "gap must be a number"))?;
            if !gap.is_finite() || gap < 0.0 {
                return Err(error(line, "gap must be finite and nonnegative"));
            }
            gap
        }
        None => 16.0,
    };
    Ok(Some(Placement {
        id: id.trim().into(),
        participants,
        gap,
        line,
    }))
}

pub(super) fn validate(diagram: &SequenceDiagram) -> Result<(), ParseError> {
    let mut seen = Vec::new();
    for event in &diagram.events {
        if let Event::Continuation(cont) = event {
            if seen.contains(&cont.id)
                || diagram.participants.iter().any(|p| p.id == cont.id)
                || diagram
                    .events
                    .iter()
                    .any(|e| matches!(e, Event::Message(m) if m.name.as_deref() == Some(&cont.id)))
            {
                return Err(error(
                    cont.line,
                    format!("duplicate diagram identifier '{}'", cont.id),
                ));
            }
            seen.push(cont.id.clone());
            if diagram
                .continuation_layout
                .iter()
                .filter(|p| p.id == cont.id)
                .count()
                != 1
            {
                return Err(error(
                    cont.line,
                    format!(
                        "continuation '{}' needs exactly one layout placement",
                        cont.id
                    ),
                ));
            }
        }
    }
    for place in &diagram.continuation_layout {
        let cont = diagram
            .events
            .iter()
            .find_map(|event| {
                if let Event::Continuation(c) = event
                    && c.id == place.id
                {
                    Some(c)
                } else {
                    None
                }
            })
            .ok_or_else(|| error(place.line, format!("unknown continuation '{}'", place.id)))?;
        for id in &place.participants {
            let p = diagram
                .participants
                .iter()
                .find(|p| p.id == *id)
                .ok_or_else(|| error(place.line, format!("unknown participant '{id}'")))?;
            if p.created_line.is_some_and(|created| cont.line < created)
                || p.destroyed_line
                    .is_some_and(|destroyed| cont.line >= destroyed)
            {
                return Err(error(
                    place.line,
                    format!("continuation '{}' spans an inactive participant", place.id),
                ));
            }
        }
    }
    Ok(())
}
