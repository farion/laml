//! Optional mandatory lifeline coverage for named combined fragments.

use super::{Event, SequenceDiagram, error};
use crate::lex::valid_identifier;
use crate::parser::ParseError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameCoverage {
    pub id: String,
    pub participants: Vec<String>,
    pub line: usize,
}

pub(super) fn parse(text: &str, line: usize) -> Result<Option<FrameCoverage>, ParseError> {
    let Some((id, members)) = text.split_once(" covers ") else {
        return Ok(None);
    };
    if !valid_identifier(id.trim()) {
        return Err(error(line, "invalid frame ID in layout"));
    }
    let participants: Vec<String> = members
        .split(',')
        .map(str::trim)
        .map(|id| {
            if valid_identifier(id) {
                Ok(id.into())
            } else {
                Err(error(line, "invalid participant in frame coverage"))
            }
        })
        .collect::<Result<_, _>>()?;
    if participants.is_empty()
        || participants
            .iter()
            .any(|p| participants.iter().filter(|q| *q == p).count() > 1)
    {
        return Err(error(line, "frame coverage needs distinct lifelines"));
    }
    Ok(Some(FrameCoverage {
        id: id.trim().into(),
        participants,
        line,
    }))
}

pub(super) fn validate(diagram: &SequenceDiagram) -> Result<(), ParseError> {
    for (index, coverage) in diagram.frame_layout.iter().enumerate() {
        if diagram.frame_layout[..index]
            .iter()
            .any(|p| p.id == coverage.id)
        {
            return Err(error(coverage.line, "duplicate frame coverage"));
        }
        if !diagram
            .events
            .iter()
            .any(|e| matches!(e, Event::FrameStart { id: Some(name), .. } if name == &coverage.id))
        {
            return Err(error(
                coverage.line,
                format!("unknown named frame '{}'", coverage.id),
            ));
        }
        let mut indices = Vec::new();
        for id in &coverage.participants {
            let index = diagram
                .participants
                .iter()
                .position(|p| &p.id == id)
                .ok_or_else(|| error(coverage.line, format!("unknown frame participant '{id}'")))?;
            indices.push(index);
        }
        let (first, last) = (
            *indices.iter().min().unwrap(),
            *indices.iter().max().unwrap(),
        );
        if coverage.participants.len() != last - first + 1 {
            return Err(error(
                coverage.line,
                "frame coverage must list every lifeline between its endpoints",
            ));
        }
    }
    // The frame itself is the visible scope; do not leave a message drawn on
    // a lifeline outside it. Nested frames are checked against their own scope.
    let mut open: Vec<Option<&FrameCoverage>> = Vec::new();
    for event in &diagram.events {
        match event {
            Event::FrameStart { id, .. } => open.push(
                id.as_deref()
                    .and_then(|id| diagram.frame_layout.iter().find(|p| p.id == id)),
            ),
            Event::FrameEnd { .. } => {
                open.pop();
            }
            Event::Message(m) => {
                for coverage in open.iter().flatten() {
                    for id in [&m.from, &m.to] {
                        if id != "?"
                            && !id.starts_with("gate[")
                            && !coverage.participants.iter().any(|p| p == id)
                            && !m.from.starts_with("gate[")
                            && !m.to.starts_with("gate[")
                        {
                            return Err(error(
                                m.line,
                                format!(
                                    "message endpoint '{id}' lies outside frame '{}'",
                                    coverage.id
                                ),
                            ));
                        }
                    }
                }
            }
            Event::Decomposition { index, line } => {
                for coverage in open.iter().flatten() {
                    let parent = &diagram.decompositions[*index].parent;
                    if !coverage.participants.contains(parent) {
                        return Err(error(
                            *line,
                            format!(
                                "decomposition of '{parent}' lies outside frame '{}'",
                                coverage.id
                            ),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
