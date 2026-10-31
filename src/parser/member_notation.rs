//! Normalize deliberately small programming-style UML member shorthand to visible UML text.
use super::{ParseError, error};
use crate::lex::{valid_identifier, valid_reference};
use crate::syntax::MemberKind;

fn head(text: &str) -> (&str, &str) {
    if text.starts_with(['+', '-', '#', '~']) {
        (&text[..1], &text[1..])
    } else {
        ("", text)
    }
}

pub(super) fn normalize(text: &str, kind: MemberKind, line: usize) -> Result<String, ParseError> {
    let (visibility, content) = head(text);
    if kind == MemberKind::Field {
        if let Some((name, ty)) = content.split_once(':') {
            if !valid_identifier(name.trim()) || !valid_reference(ty.trim()) {
                return Err(error(line, "unsupported UML attribute name or type"));
            }
            return Ok(format!("{visibility}{}: {}", name.trim(), ty.trim()));
        }
        if valid_identifier(content.trim()) {
            return Ok(format!("{visibility}{}", content.trim()));
        }
        if content.split_whitespace().count() != 2 {
            return Err(error(line, "unsupported programming-style UML attribute"));
        }
        let mut pieces = content.split_whitespace();
        let ty = pieces.next().unwrap();
        let name = pieces.next().unwrap();
        if !valid_reference(ty) || !valid_identifier(name) {
            return Err(error(line, "unsupported programming-style UML attribute"));
        }
        return Ok(format!("{visibility}{name}: {ty}"));
    }
    if kind != MemberKind::Method {
        return Ok(text.into());
    }
    let (signature, tail) = content
        .split_once('(')
        .ok_or_else(|| error(line, "UML operation requires parentheses"))?;
    let Some((parameters, suffix)) = tail.split_once(')') else {
        return Err(error(line, "unclosed UML operation parameter list"));
    };
    if suffix.contains('(') || suffix.contains(')') {
        return Err(error(line, "unsupported UML operation suffix"));
    }
    let parameters: Vec<_> = if parameters.trim().is_empty() {
        Vec::new()
    } else {
        parameters.split(',').map(str::trim).collect()
    };
    if parameters.iter().any(|part| part.is_empty()) {
        return Err(error(line, "empty UML operation parameter"));
    }
    let words: Vec<_> = signature.split_whitespace().collect();
    if words.len() == 1 {
        let tail = suffix.trim();
        let valid_tail = tail.is_empty()
            || tail
                .strip_prefix(':')
                .is_some_and(|ty| ty.trim().is_empty() || valid_reference(ty.trim()));
        if !valid_identifier(words[0]) || !valid_tail {
            return Err(error(line, "unsupported UML operation name or return type"));
        }
        let mut seen = std::collections::HashSet::new();
        for part in &parameters {
            let (name, ty) = part
                .split_once(':')
                .ok_or_else(|| error(line, "UML parameters require name: Type"))?;
            if !valid_identifier(name.trim())
                || !valid_reference(ty.trim())
                || !seen.insert(name.trim())
            {
                return Err(error(line, "invalid or duplicate UML parameter"));
            }
        }
        return Ok(text.into());
    }
    if words.len() != 2
        || !valid_reference(words[0])
        || !valid_identifier(words[1])
        || !suffix.trim().is_empty()
    {
        return Err(error(line, "unsupported programming-style UML operation"));
    }
    let mut arguments = Vec::new();
    for parameter in &parameters {
        let pair: Vec<_> = parameter.split_whitespace().collect();
        if pair.len() != 2 || !valid_reference(pair[0]) || !valid_identifier(pair[1]) {
            return Err(error(line, "unsupported programming-style UML parameter"));
        }
        arguments.push(format!("{}: {}", pair[1], pair[0]));
    }
    Ok(format!(
        "{visibility}{}({}): {}",
        words[1],
        arguments.join(", "),
        words[0]
    ))
}
