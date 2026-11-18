//! Equal-height constraints between message occurrences and execution boundaries.
use super::{AnnotationEnd, Event, SequenceDiagram, activation_layout, annotations, error};
use crate::{
    lex::{split_once_top_level, valid_identifier},
    parser::ParseError,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Endpoint {
    Message { name: String, end: AnnotationEnd },
    Activation { reference: String, lower: bool },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alignment {
    pub first: Endpoint,
    pub second: Endpoint,
    pub reference_second: bool,
    pub line: usize,
}
impl Endpoint {
    fn parse(text: &str, line: usize) -> Result<Self, ParseError> {
        if let Some(suffix @ ('+' | '-')) = text.chars().last() {
            let reference = &text[..text.len() - 1];
            if reference.is_empty() || !reference.split('.').all(valid_identifier) {
                return Err(error(line, "invalid activation-edge reference"));
            }
            Ok(Self::Activation {
                reference: reference.into(),
                lower: suffix == '-',
            })
        } else {
            let anchor = annotations::anchor(text, line)?;
            Ok(Self::Message {
                name: anchor.message,
                end: anchor.end,
            })
        }
    }
    pub fn source_line(&self, doc: &SequenceDiagram) -> Result<usize, String> {
        match self {
            Self::Message { name, .. } => doc
                .events
                .iter()
                .find_map(|e| match e {
                    Event::Message(m) if m.name.as_deref() == Some(name) => Some(m.line),
                    _ => None,
                })
                .ok_or_else(|| format!("unknown named message '{name}'")),
            Self::Activation { reference, lower } => {
                let e = activation_layout::resolve(doc, reference)?;
                if *lower {
                    e.end
                        .ok_or_else(|| "Close the activation before aligning its lower edge".into())
                } else {
                    Ok(e.start)
                }
            }
        }
    }
}
pub(super) fn parse(text: &str, line: usize) -> Result<Option<Alignment>, ParseError> {
    let Some((pair, attrs)) = split_once_top_level(text, ':') else {
        return Ok(None);
    };
    let words = pair.split_whitespace().collect::<Vec<_>>();
    let [a, "?", b] = words.as_slice() else {
        return Ok(None);
    };
    let message = |s: &str| s.ends_with(".send") || s.ends_with(".receive");
    if !message(a) && !message(b) {
        return Ok(None);
    }
    let first = Endpoint::parse(a, line)?;
    let second = Endpoint::parse(b, line)?;
    if matches!(
        (&first, &second),
        (Endpoint::Message { .. }, Endpoint::Message { .. })
            | (Endpoint::Activation { .. }, Endpoint::Activation { .. })
    ) {
        return Err(error(
            line,
            "align one message occurrence with one activation boundary",
        ));
    }
    let attrs = attrs.split_whitespace().collect::<String>();
    if attrs != "align=center-y" && attrs != "align=center-y;reference=second" {
        return Err(error(
            line,
            "expected align=center-y, optionally with reference=second",
        ));
    }
    Ok(Some(Alignment {
        first,
        second,
        reference_second: attrs.ends_with("reference=second"),
        line,
    }))
}
fn event_line(event: &Event) -> usize {
    match event {
        Event::Message(m) => m.line,
        Event::Note(n) => n.line,
        Event::Continuation(c) => c.line,
        Event::Annotation(a) => a.line,
        Event::Ordering(o) => o.line,
        Event::FrameStart { line, .. }
        | Event::FrameBranch { line, .. }
        | Event::FrameEnd { line }
        | Event::Reference { line, .. }
        | Event::Decomposition { line, .. }
        | Event::Activate { line, .. }
        | Event::Deactivate { line, .. }
        | Event::Destroy { line, .. } => *line,
    }
}
fn scope(doc: &SequenceDiagram, line: usize) -> Vec<(usize, usize)> {
    let mut stack = Vec::new();
    for event in &doc.events {
        let at = event_line(event);
        if at == line {
            return stack;
        }
        match event {
            Event::FrameStart { line, .. } => stack.push((*line, *line)),
            Event::FrameBranch { line, .. } => {
                if let Some(top) = stack.last_mut() {
                    top.1 = *line;
                }
            }
            Event::FrameEnd { .. } => {
                stack.pop();
            }
            _ => {}
        }
    }
    stack
}
pub(super) fn validate(doc: &SequenceDiagram) -> Result<(), ParseError> {
    for rule in &doc.occurrence_layout {
        let a = rule
            .first
            .source_line(doc)
            .map_err(|e| error(rule.line, e))?;
        let b = rule
            .second
            .source_line(doc)
            .map_err(|e| error(rule.line, e))?;
        if scope(doc, a) != scope(doc, b) {
            return Err(error(
                rule.line,
                "occurrence alignment cannot cross fragment operands",
            ));
        }
    }
    Ok(())
}
