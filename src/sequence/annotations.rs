//! Visible time, duration and state annotations at named message ends.

use super::{Event, SequenceDiagram, error};
use crate::lex::{quoted_string, split_once_top_level, valid_identifier};
use crate::parser::ParseError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Send,
    Receive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    TimeObservation,
    TimeConstraint,
    DurationObservation,
    DurationConstraint,
    State,
    Action,
    Invariant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub message: String,
    pub end: End,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderingRule {
    pub first: OrderingEnd,
    pub second: OrderingEnd,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrderingEnd {
    Message(Anchor),
    Execution {
        participant: String,
        id: String,
        finish: bool,
    },
}

fn ordering_end(input: &str, line: usize) -> Result<OrderingEnd, ParseError> {
    let text = input.trim();
    if let Some(value) = text.strip_prefix("execution(") {
        let (owner, bound) = value
            .split_once(").")
            .ok_or_else(|| error(line, "expected execution(Participant,id).start or .finish"))?;
        let (participant, id) = owner.split_once(',').ok_or_else(|| {
            error(
                line,
                "execution reference needs participant and local bar ID",
            )
        })?;
        if !valid_identifier(participant.trim()) || !valid_identifier(id.trim()) {
            return Err(error(line, "invalid execution reference"));
        }
        let finish = match bound {
            "start" => false,
            "finish" => true,
            _ => return Err(error(line, "execution endpoint must be start or finish")),
        };
        return Ok(OrderingEnd::Execution {
            participant: participant.trim().into(),
            id: id.trim().into(),
            finish,
        });
    }
    Ok(OrderingEnd::Message(anchor(text, line)?))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Annotation {
    pub first: OrderingEnd,
    pub second: Option<OrderingEnd>,
    pub kind: Kind,
    pub text: String,
    pub line: usize,
}

pub(super) fn anchor(input: &str, line: usize) -> Result<Anchor, ParseError> {
    let (message, end) = input
        .trim()
        .split_once('.')
        .ok_or_else(|| error(line, "expected message.send or message.receive"))?;
    if !valid_identifier(message) {
        return Err(error(line, "invalid named message anchor"));
    }
    let end = match end.trim() {
        "send" => End::Send,
        "receive" => End::Receive,
        _ => return Err(error(line, "expected send or receive endpoint")),
    };
    Ok(Anchor {
        message: message.into(),
        end,
    })
}

pub(super) fn parse_order(text: &str, line: usize) -> Result<Option<OrderingRule>, ParseError> {
    let Some(body) = text.strip_prefix("order[") else {
        return Ok(None);
    };
    let target = body
        .strip_suffix(']')
        .ok_or_else(|| error(line, "expected order[first.send,second.receive]"))?;
    let (first, second) = split_once_top_level(target, ',')
        .ok_or_else(|| error(line, "order requires two event endpoints"))?;
    let (first, second) = (ordering_end(first, line)?, ordering_end(second, line)?);
    if first == second {
        return Err(error(line, "event cannot be ordered before itself"));
    }
    Ok(Some(OrderingRule {
        first,
        second,
        line,
    }))
}

pub(super) fn parse_line(text: &str, line: usize) -> Result<Option<Annotation>, ParseError> {
    let (duration, body) = if let Some(body) = text.strip_prefix("at[") {
        (false, body)
    } else if let Some(body) = text.strip_prefix("between[") {
        (true, body)
    } else {
        return Ok(None);
    };
    let (target, attributes) = body
        .split_once(']')
        .ok_or_else(|| error(line, "unclosed annotation target"))?;
    let (first, second) = if duration {
        let (from, to) = split_once_top_level(target, ',')
            .ok_or_else(|| error(line, "expected between[first.send,second.receive]"))?;
        (ordering_end(from, line)?, Some(ordering_end(to, line)?))
    } else {
        (ordering_end(target, line)?, None)
    };
    let attributes = attributes
        .trim()
        .strip_prefix(':')
        .ok_or_else(|| error(line, "expected ':' after annotation target"))?
        .trim();
    let (key, value) = split_once_top_level(attributes, '=')
        .ok_or_else(|| error(line, "expected annotation property=\"text\""))?;
    let kind = match (duration, key.trim()) {
        (false, "time-observation") => Kind::TimeObservation,
        (false, "time-constraint") => Kind::TimeConstraint,
        (false, "state") => Kind::State,
        (false, "action") => Kind::Action,
        (false, "invariant") => Kind::Invariant,
        (true, "duration-observation") => Kind::DurationObservation,
        (true, "duration-constraint") => Kind::DurationConstraint,
        _ => {
            return Err(error(
                line,
                "annotation property does not match at[] or between[]",
            ));
        }
    };
    if kind == Kind::Action
        && !matches!(
            &first,
            OrderingEnd::Message(Anchor {
                end: End::Receive,
                ..
            })
        )
    {
        return Err(error(
            line,
            "action symbol must attach to a message receive occurrence",
        ));
    }
    let value = quoted_string(value.trim()).map_err(|msg| error(line, msg))?;
    if value.trim().is_empty() {
        return Err(error(line, "annotation text cannot be empty"));
    }
    Ok(Some(Annotation {
        first,
        second,
        kind,
        text: value,
        line,
    }))
}

fn key(anchor: &Anchor) -> String {
    let end = if anchor.end == End::Send {
        "send"
    } else {
        "receive"
    };
    format!("{}.{}", anchor.message, end)
}

fn ordering_key(end: &OrderingEnd) -> String {
    match end {
        OrderingEnd::Message(anchor) => key(anchor),
        OrderingEnd::Execution {
            participant,
            id,
            finish,
        } => format!(
            "execution({participant},{id}).{}",
            if *finish { "finish" } else { "start" }
        ),
    }
}

fn reaches(edges: &[(String, String)], start: &str, goal: &str) -> bool {
    let mut stack = vec![start.to_string()];
    let mut visited = Vec::new();
    while let Some(current) = stack.pop() {
        if current == goal {
            return true;
        }
        if visited.contains(&current) {
            continue;
        }
        visited.push(current.clone());
        stack.extend(
            edges
                .iter()
                .filter(|(from, _)| from == &current)
                .map(|(_, to)| to.clone()),
        );
    }
    false
}

pub(super) fn validate(diagram: &SequenceDiagram) -> Result<(), ParseError> {
    let mut seen = Vec::new();
    let mut edges: Vec<(String, String)> = Vec::new();
    for (index, event) in diagram.events.iter().enumerate() {
        match event {
            Event::Message(message) => {
                if let Some(name) = &message.name {
                    seen.push(name.as_str());
                    edges.push((format!("{name}.send"), format!("{name}.receive")));
                }
            }
            Event::Deactivate {
                participant,
                id: Some(id),
                ..
            } => {
                edges.push((
                    format!("execution({participant},{id}).start"),
                    format!("execution({participant},{id}).finish"),
                ));
            }
            Event::Annotation(annotation) => {
                for reference in [
                    &annotation.first,
                    annotation.second.as_ref().unwrap_or(&annotation.first),
                ] {
                    validate_reference(
                        reference,
                        &diagram.events[..index],
                        &seen,
                        annotation.line,
                    )?;
                }
            }
            Event::Ordering(rule) => {
                for reference in [&rule.first, &rule.second] {
                    validate_reference(reference, &diagram.events[..index], &seen, rule.line)?;
                }
                let (from, to) = (ordering_key(&rule.first), ordering_key(&rule.second));
                if reaches(&edges, &to, &from) {
                    return Err(error(rule.line, "contradictory event ordering cycle"));
                }
                edges.push((from, to));
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_reference(
    reference: &OrderingEnd,
    earlier: &[Event],
    seen: &[&str],
    line: usize,
) -> Result<(), ParseError> {
    match reference {
        OrderingEnd::Message(anchor) if !seen.contains(&anchor.message.as_str()) => Err(error(
            line,
            format!("unknown earlier named message '{}'", anchor.message),
        )),
        OrderingEnd::Execution {
            participant,
            id,
            finish,
        } => {
            let exists = earlier.iter().any(|event| {
                matches!(event,
                Event::Activate { participant: owner, id: Some(name), .. }
                    if owner == participant && name == id)
            });
            let closed = earlier.iter().any(|event| {
                matches!(event,
                Event::Deactivate { participant: owner, id: Some(name), .. }
                    if owner == participant && name == id)
            });
            if !exists || (*finish && !closed) {
                return Err(error(
                    line,
                    format!("unknown earlier execution boundary '{participant}.{id}'"),
                ));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
