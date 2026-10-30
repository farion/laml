//! Nested interaction frames. Branches resolve shorthand against their own
//! incoming context, never against messages in a preceding alternative.

use super::{Event, SequenceDiagram};
use crate::lex::{quoted_string, split_once_top_level, split_top_level, valid_identifier};
use crate::parser::ParseError;

type Calls = Vec<(String, String)>;
type Active = Vec<(String, usize)>;

#[derive(Clone)]
struct Frame {
    operator: String,
    entry_calls: Calls,
    entry_active: Active,
    exits: Vec<(Calls, Active)>,
    pending: bool,
    operand_start: usize,
    start: usize,
    entry_lifetimes: Vec<(String, Option<usize>)>,
    exit_lifetimes: Vec<Vec<bool>>,
}

#[derive(Default)]
pub(super) struct Frames(Vec<Frame>);

fn error(line: usize, message: &str) -> ParseError {
    ParseError {
        line,
        message: message.into(),
    }
}

fn operator(text: &str) -> Option<(&str, &str)> {
    if text.starts_with("loop(") {
        return Some(("loop", &text[4..]));
    }
    let (name, rest) = text.split_once(' ')?;
    if !matches!(
        name,
        "alt"
            | "opt"
            | "loop"
            | "par"
            | "break"
            | "critical"
            | "neg"
            | "assert"
            | "strict"
            | "seq"
            | "ignore"
            | "consider"
            | "ref"
    ) {
        return None;
    }
    Some((name, rest.trim()))
}

fn loop_bound(value: &str, line: usize, minimum: bool) -> Result<String, ParseError> {
    if value.starts_with('"') {
        let expression = quoted_string(value).map_err(|message| error(line, message))?;
        if expression.trim().is_empty() {
            return Err(error(line, "loop bound expression cannot be empty"));
        }
        return Ok(expression);
    }
    if !minimum && value == "*" {
        return Ok("*".into());
    }
    let number: u64 = value
        .parse()
        .map_err(|_| error(line, "loop bound must be an integer or quoted expression"))?;
    Ok(number.to_string())
}

fn loop_caption(
    text: &str,
    line: usize,
    semantic_check: bool,
) -> Result<(String, String), ParseError> {
    let attributes = text
        .strip_suffix('{')
        .ok_or_else(|| error(line, "loop needs '{'"))?
        .trim();
    let body = attributes
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
        .ok_or_else(|| error(line, "expected loop(min=...; max=...; guard=\"...\")"))?;
    let (mut min, mut max, mut guard) = (None, None, None);
    for attribute in split_top_level(body, ';') {
        let (key, value) = split_once_top_level(attribute.trim(), '=')
            .ok_or_else(|| error(line, "expected loop attribute=value"))?;
        let value = value.trim();
        match key.trim() {
            "min" if min.is_none() => min = Some(loop_bound(value, line, true)?),
            "max" if max.is_none() => max = Some(loop_bound(value, line, false)?),
            "guard" if guard.is_none() => {
                guard = Some(quoted_string(value).map_err(|message| error(line, message))?);
            }
            "min" | "max" | "guard" => return Err(error(line, "duplicate loop attribute")),
            _ => return Err(error(line, "unknown loop attribute")),
        }
    }
    if max.is_some() && min.is_none() {
        return Err(error(line, "loop max requires min"));
    }
    if let (Some(low), Some(high)) = (min.as_deref(), max.as_deref())
        && let (Ok(low), Ok(high)) = (low.parse::<u64>(), high.parse::<u64>())
        && semantic_check
        && high < low
    {
        return Err(error(line, "loop max must be at least min"));
    }
    let caption = match (min, max) {
        (Some(low), Some(high)) => format!("loop({low},{high})"),
        (Some(low), None) => format!("loop({low})"),
        (None, None) => "loop".into(),
        (None, Some(_)) => unreachable!(),
    };
    Ok((caption, guard.unwrap_or_default()))
}

fn guard(text: &str, line: usize) -> Result<String, ParseError> {
    let inner = text
        .strip_suffix('{')
        .ok_or_else(|| error(line, "fragment needs '{'"))?
        .trim();
    if inner.is_empty() {
        return Ok(String::new());
    }
    quoted_string(inner).map_err(|message| error(line, message))
}

impl Frames {
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(super) fn operand_scope(&self) -> Option<(usize, usize)> {
        self.0
            .last()
            .map(|frame| (frame.start, frame.operand_start))
    }

    fn check_lifetimes(
        frame: &Frame,
        diagram: &SequenceDiagram,
        line: usize,
    ) -> Result<(), ParseError> {
        for (participant, created) in &diagram.births {
            if *created >= frame.operand_start
                && *created < line
                && !diagram.events.iter().any(|later| {
                    matches!(later, Event::Destroy { participant: id, line: ended }
                        if id == participant && *ended > *created && *ended < line)
                })
            {
                return Err(error(
                    line,
                    "lifeline created in a fragment operand must be destroyed within that operand",
                ));
            }
        }
        Ok(())
    }

    fn finish(
        &mut self,
        line: usize,
        diagram: &mut SequenceDiagram,
        calls: &mut Calls,
        active: &mut Active,
    ) -> Result<(), ParseError> {
        let mut frame = self
            .0
            .pop()
            .ok_or_else(|| error(line, "unexpected fragment end"))?;
        Self::check_lifetimes(&frame, diagram, line)?;
        frame.exit_lifetimes.push(
            frame
                .entry_lifetimes
                .iter()
                .map(|(id, start)| {
                    diagram
                        .participants
                        .iter()
                        .find(|p| &p.id == id)
                        .is_some_and(|p| p.destroyed_line != *start)
                })
                .collect(),
        );
        frame.exits.push((calls.clone(), active.clone()));
        // opt and loop may contribute no events. The body of a loop must leave
        // observable execution state unchanged after one iteration.
        if frame
            .operator
            .split_whitespace()
            .any(|name| matches!(name, "opt" | "loop"))
        {
            frame
                .exits
                .push((frame.entry_calls.clone(), frame.entry_active.clone()));
            frame
                .exit_lifetimes
                .push(vec![false; frame.entry_lifetimes.len()]);
        }
        if frame
            .exit_lifetimes
            .iter()
            .any(|state| state != &frame.exit_lifetimes[0])
        {
            return Err(error(
                line,
                "fragment operands leave different lifeline lifetimes",
            ));
        }
        // Divergent calls are drawable. Only a later implicit reply must not
        // guess which branch's call it refers to. Keep explicit replies valid.
        let same_calls = frame
            .exits
            .iter()
            .all(|(branch, _)| branch == &frame.exits[0].0);
        let same_active = frame
            .exits
            .iter()
            .all(|(_, branch)| branch == &frame.exits[0].1);
        if !same_active {
            return Err(error(
                line,
                "fragment operands leave different open execution bars; close them explicitly within each operand",
            ));
        }
        if let Some((next_calls, next_active)) = frame.exits.into_iter().next() {
            *calls = if same_calls { next_calls } else { Vec::new() };
            *active = next_active;
        }
        diagram.events.push(Event::FrameEnd { line });
        Ok(())
    }

    /// Returns false if this line belongs to the enclosing semantics section.
    pub(super) fn consume(
        &mut self,
        text: &str,
        line: usize,
        diagram: &mut SequenceDiagram,
        calls: &mut Calls,
        active: &mut Active,
    ) -> Result<bool, ParseError> {
        if self.0.last().is_some_and(|frame| frame.pending) {
            let branch = if let Some(rest) = text.strip_prefix("else ") {
                Some(("else", rest))
            } else {
                text.strip_prefix("and ").map(|rest| ("and", rest))
            };
            if let Some((kind, rest)) = branch {
                let frame = self.0.last_mut().expect("pending frame");
                let branch_operator = frame
                    .operator
                    .split_whitespace()
                    .find(|name| matches!(*name, "alt" | "par"))
                    .unwrap_or("");
                if (kind == "and") != (branch_operator == "par")
                    || !matches!(branch_operator, "alt" | "par")
                {
                    return Err(error(
                        line,
                        "branch separator does not match fragment operator",
                    ));
                }
                let label = guard(rest, line)?;
                Self::check_lifetimes(frame, diagram, line)?;
                frame.exit_lifetimes.push(
                    frame
                        .entry_lifetimes
                        .iter()
                        .map(|(id, start)| {
                            diagram
                                .participants
                                .iter()
                                .find(|p| &p.id == id)
                                .is_some_and(|p| p.destroyed_line != *start)
                        })
                        .collect(),
                );
                frame.exits.push((calls.clone(), active.clone()));
                for (id, original) in &frame.entry_lifetimes {
                    if let Some(p) = diagram.participants.iter_mut().find(|p| &p.id == id) {
                        p.destroyed_line = *original;
                    }
                }
                *calls = frame.entry_calls.clone();
                *active = frame.entry_active.clone();
                frame.pending = false;
                frame.operand_start = line;
                diagram
                    .events
                    .push(Event::FrameBranch { guard: label, line });
                return Ok(true);
            }
            self.finish(line, diagram, calls, active)?;
        }
        if text == "}" {
            if let Some(frame) = self.0.last_mut() {
                frame.pending = true;
                return Ok(true);
            }
            return Ok(false);
        }
        if text.starts_with("else ") || text.starts_with("and ") {
            return Err(error(line, "branch without preceding fragment operand"));
        }
        let (id, form) = match split_once_top_level(text, ':') {
            Some((id, form)) if valid_identifier(id.trim()) && form.trim().ends_with('{') => {
                (Some(id.trim().to_string()), form.trim())
            }
            _ => (None, text),
        };
        if let Some((outer, mut rest)) = operator(form) {
            let mut operators = vec![outer];
            while !matches!(operators.last(), Some(&"ref" | &"ignore" | &"consider")) {
                let Some((inner, following)) = operator(rest).filter(|(inner, _)| *inner != "ref")
                else {
                    break;
                };
                operators.push(inner);
                rest = following;
            }
            let name = *operators.last().expect("at least one operator");
            if let Some(id) = &id {
                if operator(&format!("{id} {{")).is_some() {
                    return Err(error(line, "fragment ID cannot be a fragment operator"));
                }
                if diagram.events.iter().any(|event| {
                    matches!(event,
                    Event::FrameStart { id: Some(existing), .. } if existing == id)
                }) {
                    return Err(error(line, "duplicate fragment ID"));
                }
            }
            if name == "ref" && !rest.ends_with('{') {
                return Ok(false);
            }
            let (caption, label) = if name == "loop" && rest.starts_with('(') {
                loop_caption(rest, line, !diagram.semantic_check_off)?
            } else if matches!(name, "ignore" | "consider") {
                let messages = guard(rest, line)?;
                if messages.trim().is_empty() {
                    return Err(error(line, "ignore/consider need a visible message list"));
                }
                (format!("{name} {{{messages}}}"), String::new())
            } else if let Some((filter, value)) = rest.split_once(' ')
                && matches!(filter, "ignore" | "consider")
                && !matches!(name, "ref" | "ignore" | "consider")
            {
                let messages = guard(value, line)?;
                if messages.trim().is_empty() {
                    return Err(error(line, "ignore/consider need a visible message list"));
                }
                (format!("{name} {filter} {{{messages}}}"), String::new())
            } else {
                (name.to_string(), guard(rest, line)?)
            };
            let caption = if operators.len() > 1 {
                format!("{} {caption}", operators[..operators.len() - 1].join(" "))
            } else {
                caption
            };
            self.0.push(Frame {
                operator: operators.join(" "),
                entry_calls: calls.clone(),
                entry_active: active.clone(),
                exits: Vec::new(),
                pending: false,
                operand_start: line,
                start: line,
                entry_lifetimes: diagram
                    .participants
                    .iter()
                    .map(|p| (p.id.clone(), p.destroyed_line))
                    .collect(),
                exit_lifetimes: Vec::new(),
            });
            diagram.events.push(Event::FrameStart {
                operator: caption,
                id,
                guard: label,
                line,
            });
            return Ok(true);
        }
        Ok(false)
    }
}
