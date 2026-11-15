//! Symbolic equal-height execution edges, separate from event order.
use super::{Event, SequenceDiagram, error};
use crate::{lex::split_once_top_level, parser::ParseError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alignment {
    pub first: String,
    pub second: String,
    pub lower: bool,
    pub reference_second: bool,
    pub line: usize,
}
#[derive(Clone, Debug)]
pub struct Execution {
    pub owner: String,
    pub name: Option<String>,
    pub start: usize,
    pub end: Option<usize>,
}

pub fn executions(doc: &SequenceDiagram) -> Vec<Execution> {
    let mut open: Vec<Execution> = Vec::new();
    let mut frames = Vec::new();
    let mut out = Vec::new();
    for event in &doc.events {
        match event {
            Event::Activate {
                participant,
                id,
                line,
                ..
            } => {
                let name = id.clone().or_else(|| {
                    doc.events.iter().find_map(|e| match e {
                        Event::Message(m) if m.line == *line => m.name.clone(),
                        _ => None,
                    })
                });
                open.push(Execution {
                    owner: participant.clone(),
                    name,
                    start: *line,
                    end: None,
                });
            }
            Event::Deactivate {
                participant, line, ..
            } => {
                if let Some(at) = open.iter().rposition(|e| &e.owner == participant) {
                    let mut e = open.remove(at);
                    e.end = Some(*line);
                    out.push(e);
                }
            }
            Event::Destroy { participant, line } => {
                let mut i = 0;
                while i < open.len() {
                    if &open[i].owner == participant {
                        let mut e = open.remove(i);
                        e.end = Some(*line);
                        out.push(e);
                    } else {
                        i += 1;
                    }
                }
            }
            Event::FrameStart { .. } => frames.push(open.clone()),
            Event::FrameBranch { .. } => {
                if let Some(incoming) = frames.last() {
                    open = incoming.clone();
                }
            }
            Event::FrameEnd { .. } => {
                frames.pop();
            }
            _ => {}
        }
    }
    out.extend(open);
    out
}

pub fn resolve(doc: &SequenceDiagram, reference: &str) -> Result<Execution, String> {
    let (owner, name) = reference
        .split_once('.')
        .map_or((reference, None), |(a, b)| (a, Some(b)));
    let matches: Vec<_> = executions(doc)
        .into_iter()
        .filter(|e| e.owner == owner && name.is_none_or(|n| e.name.as_deref() == Some(n)))
        .collect();
    if matches.len() != 1 {
        return Err(format!(
            "'{reference}' must identify one activation; use Participant.activationName for multiple activations"
        ));
    }
    Ok(matches[0].clone())
}
pub(super) fn parse(text: &str, line: usize) -> Result<Option<Alignment>, ParseError> {
    if !text.contains('?')
        || !text
            .split_whitespace()
            .next()
            .is_some_and(|s| s.ends_with(['+', '-']))
    {
        return Ok(None);
    }
    let (pair, attrs) = split_once_top_level(text, ':')
        .ok_or_else(|| error(line, "expected A- ? B- : align=center-y"))?;
    let words: Vec<_> = pair.split_whitespace().collect();
    let [a, "?", b] = words.as_slice() else {
        return Err(error(line, "expected two activation-edge references"));
    };
    let lower = a.ends_with('-');
    let suffix = if lower { '-' } else { '+' };
    let attrs = attrs.split_whitespace().collect::<String>();
    let reference_second = attrs == "align=center-y;reference=second";
    if !b.ends_with(suffix) || (!reference_second && attrs != "align=center-y") {
        return Err(error(
            line,
            "align matching activation edges using A+ ? B+ or A- ? B- : align=center-y",
        ));
    }
    let first = a[..a.len() - 1].to_owned();
    let second = b[..b.len() - 1].to_owned();
    if first == second {
        return Err(error(line, "choose two different activations"));
    }
    Ok(Some(Alignment {
        first,
        second,
        lower,
        reference_second,
        line,
    }))
}
pub(super) fn validate(doc: &SequenceDiagram) -> Result<(), ParseError> {
    for rule in &doc.activation_layout {
        let a = resolve(doc, &rule.first).map_err(|e| error(rule.line, e))?;
        let b = resolve(doc, &rule.second).map_err(|e| error(rule.line, e))?;
        let edge = |e: &Execution| {
            if rule.lower {
                e.end.ok_or_else(|| {
                    error(
                        rule.line,
                        "close the activation before aligning its lower edge",
                    )
                })
            } else {
                Ok(e.start)
            }
        };
        let (a, b) = (edge(&a)?, edge(&b)?);
        if doc.events.iter().any(|e| match e {
            Event::FrameStart { line, .. }
            | Event::FrameBranch { line, .. }
            | Event::FrameEnd { line } => *line > a.min(b) && *line < a.max(b),
            _ => false,
        }) {
            return Err(error(
                rule.line,
                "activation alignment cannot cross a fragment or operand boundary",
            ));
        }
    }
    Ok(())
}
pub fn groups(doc: &SequenceDiagram, lower: bool) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    for rule in doc.activation_layout.iter().filter(|r| r.lower == lower) {
        let mut merged = vec![rule.first.clone(), rule.second.clone()];
        let mut i = 0;
        while i < out.len() {
            if out[i].iter().any(|s| merged.contains(s)) {
                for s in out.remove(i) {
                    if !merged.contains(&s) {
                        merged.push(s);
                    }
                }
                i = 0;
            } else {
                i += 1;
            }
        }
        out.push(merged);
    }
    out
}
