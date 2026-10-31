//! Small checked template formal declarations; substitution never invents a type model.
use super::{ParseError, error};
use crate::lex::{valid_identifier, valid_reference};

pub(super) fn parse(
    name: &str,
    line: usize,
) -> Result<(String, Vec<String>, Option<String>), ParseError> {
    let Some(open) = name.find('<') else {
        return Ok((name.into(), Vec::new(), None));
    };
    let close = name[open + 1..]
        .find('>')
        .ok_or_else(|| error(line, "unclosed template parameters"))?
        + open
        + 1;
    let list = &name[open + 1..close];
    let mut names = Vec::new();
    let mut display = Vec::new();
    for part in list.split(',') {
        let part = part.trim();
        let (head, default) = part
            .split_once('=')
            .map_or((part, None), |(head, default)| (head, Some(default.trim())));
        let (formal, kind) = head
            .split_once(':')
            .map_or((head.trim(), None), |(name, kind)| {
                (name.trim(), Some(kind.trim()))
            });
        if !valid_identifier(formal)
            || names.iter().any(|name| name == formal)
            || kind.is_some_and(|value| !valid_identifier(value))
            || default.is_some_and(|value| !valid_reference(value))
        {
            return Err(error(line, "invalid or duplicate template parameter"));
        }
        names.push(formal.to_string());
        display.push(format!(
            "{formal}{}{}",
            kind.map_or(String::new(), |ty| format!(":{ty}")),
            default.map_or(String::new(), |value| format!("={value}"))
        ));
    }
    let suffix = &name[close + 1..];
    if !suffix.is_empty() && !suffix.starts_with('(') {
        return Err(error(line, "unexpected text after template parameters"));
    }
    let mut prepared = name[..open].to_string();
    prepared.push_str(suffix);
    Ok((prepared, names, Some(display.join(", "))))
}
