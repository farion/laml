//! Typed lexical fields target exact authored tokens, never regenerated declarations.
use crate::{document::Edit, source::SourceIndex, style_edit::code};
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    Quoted,
    Number,
    Reference,
    Choice(Vec<String>),
}
#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    pub value: String,
    pub range: Range<usize>,
    pub kind: Kind,
}

pub fn tokens(text: &str) -> Vec<Range<usize>> {
    let mut result = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let c = text[at..].chars().next().unwrap();
        if c.is_whitespace() {
            at += c.len_utf8();
            continue;
        }
        let start = at;
        if c == '"' {
            at += 1;
            let mut escaped = false;
            while at < text.len() {
                let ch = text[at..].chars().next().unwrap();
                at += ch.len_utf8();
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    break;
                }
            }
        } else if "();={},{}[]".contains(c) || (c == ':' && !text[at..].starts_with("::")) {
            at += c.len_utf8();
        } else {
            while at < text.len() {
                let ch = text[at..].chars().next().unwrap();
                if text[at..].starts_with("::") {
                    at += 2;
                    continue;
                }
                if ch.is_whitespace() || "();={},{}[]:\"".contains(ch) {
                    break;
                }
                at += ch.len_utf8();
            }
            if at == start {
                at += c.len_utf8();
            }
        }
        result.push(start..at);
    }
    result
}
fn decoded(text: &str) -> String {
    if !text.starts_with('"') {
        return text.into();
    }
    let mut out = String::new();
    let mut chars = text[1..text.len().saturating_sub(1)].chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            out.push(match chars.next() {
                Some('n') => '\n',
                Some(c) => c,
                None => '\\',
            });
        } else {
            out.push(c);
        }
    }
    out
}
pub fn of(source: &str, line: usize) -> Vec<Field> {
    let Some(range) = SourceIndex::new(source).line(line) else {
        return vec![];
    };
    let text = code(&source[range.clone()]);
    let ts = tokens(text);
    let raw: Vec<_> = ts.iter().map(|r| &text[r.clone()]).collect();
    let mut result = Vec::new();
    let mut skip = std::collections::BTreeSet::new();
    let section = crate::notation_edit::record(source, line)
        .ok()
        .map(|r| r.section)
        .unwrap_or_default();
    // Attributes and key/value settings retain their authored representation.
    for i in 0..raw.len().saturating_sub(2) {
        if skip.contains(&i) {
            continue;
        }
        if matches!(raw[i + 1], "=" | ":") && !matches!(raw[i], "(" | ")" | "{" | "}") {
            if raw[i + 1] == ":"
                && (!matches!(section.as_str(), "settings" | "styling")
                    && !(section == "layout"
                        && i == 0
                        && ["scale", "lane-gap", "end"].contains(&raw[i])))
            {
                continue;
            }
            if raw[i + 1] == ":" && raw.iter().any(|s| arrow(s)) {
                continue;
            }
            let mut end = i + 3;
            while end < raw.len() && !matches!(raw[end], ";" | ")" | "{" | "}") {
                end += 1;
            }
            let value = &text[ts[i + 2].start..ts[end - 1].end];
            let kind = kind_for(raw[i], value);
            result.push(Field {
                name: title(raw[i]),
                value: decoded(value),
                range: range.start + ts[i + 2].start..range.start + ts[end - 1].end,
                kind,
            });
            skip.extend(i..end);
        }
    }
    if crate::notation::Notation::of(source) == crate::notation::Notation::Timing
        && section == "semantics"
        && let Some(colon) = raw.iter().position(|s| *s == ":")
        && colon + 1 < raw.len()
    {
        let name = match raw.first().copied() {
            Some("lifeline") => "Initial state",
            Some("state") => "State",
            _ => "Label",
        };
        let start = ts[colon + 1].start;
        let end = ts.last().unwrap().end;
        result.push(Field {
            name: name.into(),
            value: text[start..end].into(),
            range: range.start + start..range.start + end,
            kind: Kind::Text,
        });
        skip.extend(colon + 1..raw.len());
    }
    let op = raw.iter().position(|s| arrow(s));
    if let Some(op) = op {
        let choices =
            if crate::notation::Notation::of(source) == crate::notation::Notation::Sequence {
                vec!["->", "<-", "->>", "<<-", "<--", "-->"]
            } else if crate::notation::Notation::of(source) == crate::notation::Notation::Bpmn {
                vec!["-->", "~~>", "..>", ".."]
            } else {
                crate::relation_edit::OPERATORS.to_vec()
            };
        result.push(Field {
            name: "Type".into(),
            value: raw[op].into(),
            range: range.start + ts[op].start..range.start + ts[op].end,
            kind: Kind::Choice(choices.into_iter().map(str::to_owned).collect()),
        });
        skip.insert(op);
        for (i, name) in [
            (op.checked_sub(1), "Source"),
            ((op + 1 < raw.len()).then_some(op + 1), "Target"),
        ] {
            if let Some(i) = i
                && !matches!(raw[i], ":" | "{")
            {
                let gate = if raw[i] == "]" && i >= 3 && raw[i - 3] == "gate" {
                    Some((i - 3, i))
                } else if raw[i] == "gate"
                    && raw.get(i + 1) == Some(&"[")
                    && raw.get(i + 3) == Some(&"]")
                {
                    Some((i, i + 3))
                } else {
                    None
                };
                if let Some((first, last)) = gate {
                    result.push(Field {
                        name: format!("{name} gate"),
                        value: raw[first + 2].into(),
                        range: range.start + ts[first + 2].start..range.start + ts[first + 2].end,
                        kind: Kind::Reference,
                    });
                    skip.extend(first..=last);
                    continue;
                }
                let suffix = raw[i]
                    .chars()
                    .last()
                    .is_some_and(|c| matches!(c, '+' | '-' | '!'))
                    as usize;
                result.push(Field {
                    name: name.into(),
                    value: raw[i][..raw[i].len() - suffix].into(),
                    range: range.start + ts[i].start..range.start + ts[i].end - suffix,
                    kind: Kind::Reference,
                });
                if crate::notation::Notation::of(source) == crate::notation::Notation::Sequence
                    && raw[i] != "?"
                {
                    result.push(Field {
                        name: format!("{name} activation / lifetime"),
                        value: raw[i][raw[i].len() - suffix..].into(),
                        range: range.start + ts[i].end - suffix..range.start + ts[i].end,
                        kind: Kind::Choice(vec!["".into(), "+".into(), "-".into(), "!".into()]),
                    });
                }
                skip.insert(i);
            }
        }
        if let Some(i) = raw
            .iter()
            .enumerate()
            .skip(op + 1)
            .find(|(_, s)| **s == ":")
            .map(|(i, _)| i)
            && i + 1 < raw.len()
        {
            let last = raw.len();
            let value = &text[ts[i + 1].start..ts[last - 1].end];
            result.push(Field {
                name: "Label".into(),
                value: decoded(value),
                range: range.start + ts[i + 1].start..range.start + ts[last - 1].end,
                kind: if value.starts_with('"') {
                    Kind::Quoted
                } else {
                    Kind::Text
                },
            });
            skip.extend(i..last);
        }
    }
    let keywords = [
        "abstract",
        "external",
        "class",
        "interface",
        "enum",
        "datatype",
        "object",
        "component",
        "part",
        "role",
        "structure",
        "profile",
        "stereotype",
        "metaclass",
        "package",
        "actor",
        "system",
        "container",
        "person",
        "usecase",
        "action",
        "state",
        "event",
        "lifeline",
        "at",
        "tick",
        "initial",
        "final",
        "choice",
        "decision",
        "merge",
        "fork",
        "join",
        "ref",
        "note",
        "activate",
        "deactivate",
        "alt",
        "else",
        "loop",
        "opt",
        "par",
        "and",
        "break",
        "critical",
        "neg",
        "assert",
        "strict",
        "seq",
        "ignore",
        "consider",
        "pool",
        "lane",
        "subprocess",
        "message",
        "data",
        "store",
        "node",
        "device",
        "artifact",
        "executionEnvironment",
        "interaction-use",
        "objectNode",
        "flowFinal",
        "activityFinal",
        "between",
        "order",
        "coregion",
        "continuation",
        "decompose",
        "startEvent",
        "endEvent",
        "userTask",
        "serviceTask",
        "sendTask",
        "receiveTask",
        "manualTask",
        "scriptTask",
        "messageStart",
        "timerStart",
        "messageEnd",
        "errorEnd",
        "terminateEnd",
        "intermediateTimer",
        "intermediateMessage",
    ];
    for (i, r) in ts.iter().enumerate() {
        let value = raw[i];
        if skip.contains(&i)
            || keywords.contains(&value)
            || ["(", ")", ";", "=", ":", "{", "}", ",", "[", "]"].contains(&value)
        {
            continue;
        }
        if op.is_none()
            && crate::notation::Notation::of(source) == crate::notation::Notation::Sequence
            && value.ends_with(['+', '-', '!'])
            && crate::graph_edit::identifier(&value[..value.len() - 1])
        {
            result.push(Field {
                name: "Participant".into(),
                value: value[..value.len() - 1].into(),
                range: range.start + r.start..range.start + r.end - 1,
                kind: Kind::Reference,
            });
            result.push(Field {
                name: "Participant activation / lifetime".into(),
                value: value[value.len() - 1..].into(),
                range: range.start + r.end - 1..range.start + r.end,
                kind: Kind::Choice(vec!["+".into(), "-".into(), "!".into()]),
            });
            continue;
        }
        let name = if value.starts_with('"') {
            if raw.iter().any(|s| {
                [
                    "alt", "else", "opt", "par", "and", "critical", "neg", "assert", "break",
                    "strict", "seq",
                ]
                .contains(s)
            }) {
                "Condition"
            } else {
                "Text"
            }
        } else if i > 0 && ["at", "tick"].contains(&raw[i - 1]) {
            "Time"
        } else if value.parse::<f64>().is_ok() {
            "Value"
        } else if i == 0 || (i > 0 && keywords.contains(&raw[i - 1])) {
            "Name / reference"
        } else {
            "Reference"
        };
        result.push(Field {
            name: name.into(),
            value: decoded(value),
            range: range.start + r.start..range.start + r.end,
            kind: kind_for(name, value),
        });
    }
    if crate::notation::Notation::of(source) == crate::notation::Notation::Sequence
        && op.is_some_and(|op| ["<-", "<<-", "<--"].contains(&raw[op]))
    {
        for field in &mut result {
            if field.name.starts_with("Source") {
                field.name = field.name.replacen("Source", "Target", 1);
            } else if field.name.starts_with("Target") {
                field.name = field.name.replacen("Target", "Source", 1);
            }
        }
    }
    result.sort_by_key(|f| f.range.start);
    result.dedup_by(|a, b| a.range == b.range);
    result
}
fn arrow(s: &str) -> bool {
    crate::relation_edit::OPERATORS.contains(&s) || ["->>", "~~>", "==", "=>"].contains(&s)
}
fn kind_for(name: &str, value: &str) -> Kind {
    if value.starts_with('"') {
        Kind::Quoted
    } else if value.parse::<f64>().is_ok() {
        Kind::Number
    } else if matches!(
        name,
        "of" | "ref" | "requires" | "provides" | "Reference" | "Name / reference"
    ) {
        Kind::Reference
    } else if matches!(value, "true" | "false") {
        Kind::Choice(vec!["true".into(), "false".into()])
    } else {
        Kind::Text
    }
}
fn title(key: &str) -> String {
    match key {
        "of" => "Classifier".into(),
        "ref" => "Interaction".into(),
        "min" => "Minimum".into(),
        "max" => "Maximum".into(),
        _ => {
            let mut c = key.chars();
            c.next()
                .map_or(String::new(), |c| c.to_uppercase().collect::<String>())
                + c.as_str()
        }
    }
}
pub fn edit(source: &str, field: &Field, value: &str) -> Result<Vec<Edit>, String> {
    if let Some(result) = super::executions::rename_field(source, field, value) {
        return result;
    }
    if field.range.end > source.len()
        || !source.is_char_boundary(field.range.start)
        || !source.is_char_boundary(field.range.end)
    {
        return Err("Property no longer exists".into());
    }
    let replacement = match &field.kind {
        Kind::Quoted => crate::relation_edit::quote(value),
        Kind::Text
            if field.name == "Label"
                && crate::notation::Notation::of(source) == crate::notation::Notation::Sequence
                && value.contains(['\r', '\n']) =>
        {
            crate::relation_edit::quote(value)
        }
        Kind::Number => {
            if !value.parse::<f64>().is_ok_and(f64::is_finite) {
                return Err("Enter a finite number".into());
            }
            value.into()
        }
        Kind::Choice(choices) => {
            if !choices.iter().any(|s| s == value) {
                return Err("Choose a supported value".into());
            }
            value.into()
        }
        _ => {
            if value.contains(['\r', '\n', '{', '}']) {
                return Err("Enter one property value, not a declaration".into());
            }
            value.into()
        }
    };
    Ok(vec![Edit {
        range: field.range.clone(),
        replacement,
    }])
}

/// Tokens that can carry references; visible relation text is opaque even when
/// the author used an unquoted label.
pub fn references(text: &str) -> Vec<Range<usize>> {
    let tokens = tokens(text);
    let op = tokens
        .iter()
        .position(|r| arrow(&text[r.clone()]) || text[r.clone()].contains("-("));
    let label = op
        .and_then(|op| {
            tokens
                .iter()
                .skip(op + 1)
                .find(|r| &text[(*r).clone()] == ":")
        })
        .map(|r| r.start);
    tokens
        .into_iter()
        .filter(|r| !text[r.clone()].starts_with('"') && label.is_none_or(|at| r.start < at))
        .collect()
}
