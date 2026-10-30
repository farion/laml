//! Small lexical helpers for sequence participants and message heads.

use super::{SequenceDiagram, error};
use crate::lex::{quoted_string, split_once_top_level, split_top_level, valid_identifier};
use crate::parser::ParseError;

pub(super) fn require_participant(
    diagram: &SequenceDiagram,
    id: &str,
    line: usize,
) -> Result<(), ParseError> {
    match diagram.participants.iter().find(|p| p.id == id) {
        Some(p) if p.destroyed_line.is_some_and(|destroyed| line >= destroyed) => Err(error(
            line,
            format!("participant '{id}' was already destroyed"),
        )),
        Some(_) => Ok(()),
        None => Err(error(line, format!("unknown participant '{id}'"))),
    }
}

pub(super) fn close(
    active: &mut Vec<(String, usize)>,
    id: &str,
    line: usize,
) -> Result<(), ParseError> {
    let index = active
        .iter()
        .rposition(|(name, _)| name == id)
        .ok_or_else(|| error(line, format!("unmatched activation close for '{id}'")))?;
    active.remove(index);
    Ok(())
}

pub(super) fn gate_id(input: &str) -> Option<&str> {
    let id = input.strip_prefix("gate[")?.strip_suffix(']')?;
    valid_identifier(id).then_some(id)
}

pub(super) fn head_tokens(input: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = None;
    let (mut quoted, mut escaped, mut depth) = (false, false, 0_usize);
    for (index, ch) in input.char_indices() {
        if !quoted && depth == 0 && ch.is_whitespace() {
            if let Some(from) = start.take() {
                result.push(&input[from..index]);
            }
            continue;
        }
        if start.is_none() {
            start = Some(index);
        }
        if escaped {
            escaped = false;
        } else if quoted && ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            quoted = !quoted;
        } else if !quoted {
            if ch == '(' {
                depth += 1;
            }
            if ch == ')' {
                depth = depth.saturating_sub(1);
            }
        }
    }
    if let Some(from) = start {
        result.push(&input[from..]);
    }
    result
}

pub(super) fn parse_label(text: &str, line: usize) -> Result<Option<String>, ParseError> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let value = text
        .strip_prefix(':')
        .ok_or_else(|| error(line, "expected ':' before message text"))?
        .trim();
    if value.is_empty() {
        return Err(error(line, "empty message label"));
    }
    Ok(Some(if value.starts_with('"') {
        quoted_string(value).map_err(|msg| error(line, msg))?
    } else {
        value.into()
    }))
}

#[derive(Default)]
pub(super) struct MessageOptions {
    pub name: Option<String>,
    pub classes: Vec<String>,
    pub create: bool,
    pub arrow: Option<bool>, // filled (true) or open (false), only for replies
}

pub(super) fn parse_relation_prefix(text: &str, line: usize) -> Result<MessageOptions, ParseError> {
    let text = text.trim();
    let (name, attributes) = if let Some((name, tail)) = text.split_once('(') {
        (
            name,
            Some(
                tail.strip_suffix(')')
                    .ok_or_else(|| error(line, "unclosed relation attributes"))?,
            ),
        )
    } else {
        (text, None)
    };
    if !name.is_empty() && !valid_identifier(name) {
        return Err(error(line, "invalid message identifier"));
    }
    let mut options = MessageOptions {
        name: (!name.is_empty()).then(|| name.to_string()),
        ..MessageOptions::default()
    };
    let (mut classes, mut kind, mut arrow) = (false, false, false);
    if let Some(attributes) = attributes {
        for entry in split_top_level(attributes, ';') {
            let (key, value) = split_once_top_level(entry.trim(), '=')
                .ok_or_else(|| error(line, "expected relation attribute=value"))?;
            match (key.trim(), value.trim()) {
                ("style-classes", list) if !classes => {
                    options.classes = list
                        .split(',')
                        .map(str::trim)
                        .map(|class| {
                            if valid_identifier(class) {
                                Ok(class.to_string())
                            } else {
                                Err(error(line, "invalid message class"))
                            }
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    classes = true;
                }
                ("kind", "create") if !kind => {
                    options.create = true;
                    kind = true;
                }
                ("arrow", "filled") if !arrow => {
                    options.arrow = Some(true);
                    arrow = true;
                }
                ("arrow", "open") if !arrow => {
                    options.arrow = Some(false);
                    arrow = true;
                }
                ("style-classes" | "kind" | "arrow", _)
                    if classes && key.trim() == "style-classes"
                        || kind && key.trim() == "kind"
                        || arrow && key.trim() == "arrow" =>
                {
                    return Err(error(line, "duplicate message attribute"));
                }
                _ => return Err(error(line, "unknown or invalid message attribute")),
            }
        }
    }
    Ok(options)
}

pub(super) fn parse_participant(
    text: &str,
    line: usize,
) -> Result<(String, String, Vec<String>), ParseError> {
    if let Some((id, tail)) = text.split_once('(') {
        if !valid_identifier(id) {
            return Err(error(line, "invalid participant identifier"));
        }
        let body = tail
            .strip_suffix(')')
            .ok_or_else(|| error(line, "unclosed participant attributes"))?;
        let (mut label, mut classes, mut selector, mut ty) = (None, None, None, None);
        for entry in split_top_level(body, ';') {
            let (key, value) = split_once_top_level(entry.trim(), '=')
                .ok_or_else(|| error(line, "expected participant attribute=value"))?;
            match key.trim() {
                "label" if label.is_none() => {
                    label = Some(quoted_string(value.trim()).map_err(|msg| error(line, msg))?)
                }
                "style-classes" if classes.is_none() => {
                    let names = value
                        .split(',')
                        .map(str::trim)
                        .map(|name| {
                            if valid_identifier(name) {
                                Ok(name.to_string())
                            } else {
                                Err(error(line, "invalid participant class"))
                            }
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    classes = Some(names);
                }
                "selector" if selector.is_none() => {
                    let value = quoted_string(value.trim()).map_err(|msg| error(line, msg))?;
                    if value.trim().is_empty() {
                        return Err(error(line, "empty lifeline selector"));
                    }
                    selector = Some(value);
                }
                "type" if ty.is_none() => {
                    let value = quoted_string(value.trim()).map_err(|msg| error(line, msg))?;
                    if value.trim().is_empty() {
                        return Err(error(line, "empty lifeline type"));
                    }
                    ty = Some(value);
                }
                "label" | "style-classes" | "selector" | "type" => {
                    return Err(error(line, "duplicate participant attribute"));
                }
                _ => return Err(error(line, "unknown participant attribute")),
            }
        }
        if id == "self" && (selector.is_some() || ty.is_some()) {
            return Err(error(line, "self lifeline cannot have a selector or type"));
        }
        let mut head = label.unwrap_or_else(|| id.into());
        if let Some(selector) = selector {
            head.push_str(&format!("[{selector}]"));
        }
        if let Some(ty) = ty {
            head.push_str(&format!(": {ty}"));
        }
        Ok((id.into(), head, classes.unwrap_or_default()))
    } else if valid_identifier(text) {
        Ok((text.into(), text.into(), Vec::new()))
    } else {
        Err(error(line, "invalid participant declaration"))
    }
}
