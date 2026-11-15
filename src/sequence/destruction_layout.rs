//! Explicit equal-height layout of unambiguous standalone destruction occurrences.
use super::{Event, SequenceDiagram, error};
use crate::{
    lex::{split_once_top_level, valid_identifier},
    parser::ParseError,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DestructionAlignment {
    pub first: String,
    pub second: String,
    pub line: usize,
}

pub(super) fn parse(text: &str, line: usize) -> Result<Option<DestructionAlignment>, ParseError> {
    if !text.contains('?') {
        return Ok(None);
    }
    let (pair, attrs) = split_once_top_level(text, ':')
        .ok_or_else(|| error(line, "expected A! ? B! : align=center-y"))?;
    if attrs.split_whitespace().collect::<String>() != "align=center-y" {
        return Err(error(line, "destruction alignment requires align=center-y"));
    }
    let words: Vec<_> = pair.split_whitespace().collect();
    let [a, "?", b] = words.as_slice() else {
        return Err(error(line, "expected A! ? B! : align=center-y"));
    };
    let endpoint = |word: &str| {
        word.strip_suffix('!')
            .filter(|id| valid_identifier(id))
            .map(str::to_owned)
    };
    let first =
        endpoint(a).ok_or_else(|| error(line, "expected a destruction reference such as A!"))?;
    let second =
        endpoint(b).ok_or_else(|| error(line, "expected a destruction reference such as B!"))?;
    if first == second {
        return Err(error(line, "choose two different destruction occurrences"));
    }
    Ok(Some(DestructionAlignment {
        first,
        second,
        line,
    }))
}

/// A participant address must identify one standalone destruction, not a delete
/// message or an ambiguous lifetime reused in alternative operands.
pub fn standalone_line(diagram: &SequenceDiagram, id: &str) -> Result<usize, String> {
    let ends: Vec<_> = diagram
        .events
        .iter()
        .filter_map(|e| match e {
            Event::Destroy { participant, line } if participant == id => Some(*line),
            _ => None,
        })
        .collect();
    let [line] = ends.as_slice() else {
        return Err(format!(
            "'{id}!' must identify exactly one destruction occurrence"
        ));
    };
    if diagram
        .events
        .iter()
        .any(|e| matches!(e, Event::Message(m) if m.line == *line))
    {
        return Err(format!(
            "'{id}!' belongs to a message; align standalone destruction occurrences instead"
        ));
    }
    Ok(*line)
}

pub(super) fn validate(diagram: &SequenceDiagram) -> Result<(), ParseError> {
    for rule in &diagram.destruction_layout {
        let a = standalone_line(diagram, &rule.first).map_err(|e| error(rule.line, e))?;
        let b = standalone_line(diagram, &rule.second).map_err(|e| error(rule.line, e))?;
        // Crossing a fragment/operand boundary would extend a lifetime into a
        // different semantic scope. Never silently reinterpret such a rule.
        // Deferred fragment closure can share its source line with the next
        // destruction. Event order, not strict source-line inequalities,
        // determines whether a semantic boundary lies between the endpoints.
        let position = |line| {
            diagram.events.iter().position(|event|
            matches!(event, Event::Destroy { line: source_line, .. } if *source_line == line)
        ).expect("validated standalone destruction")
        };
        let (a, b) = (position(a), position(b));
        if diagram.events[a.min(b) + 1..a.max(b)].iter().any(|event| {
            matches!(
                event,
                Event::FrameStart { .. } | Event::FrameBranch { .. } | Event::FrameEnd { .. }
            )
        }) {
            return Err(error(
                rule.line,
                "destruction alignment cannot cross a fragment or operand boundary",
            ));
        }
    }
    Ok(())
}

/// Connected equal-height rules form one group, independent of rule order.
pub fn groups(diagram: &SequenceDiagram) -> Vec<Vec<String>> {
    let mut groups: Vec<Vec<String>> = Vec::new();
    for rule in &diagram.destruction_layout {
        let mut merged = vec![rule.first.clone(), rule.second.clone()];
        let mut i = 0;
        while i < groups.len() {
            if groups[i].iter().any(|id| merged.contains(id)) {
                for id in groups.remove(i) {
                    if !merged.contains(&id) {
                        merged.push(id);
                    }
                }
                i = 0;
            } else {
                i += 1;
            }
        }
        groups.push(merged);
    }
    groups
}
