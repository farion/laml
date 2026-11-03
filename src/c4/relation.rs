use super::{err, parse_attributes};
use crate::lex::{split_once_top_level, valid_identifier};
use crate::parser::ParseError;

pub(super) fn relation_with_technology(text: &str, line: usize) -> Result<String, ParseError> {
    let mut quoted = false;
    let mut escaped = false;
    let mut depth = 0;
    let split = text
        .char_indices()
        .find_map(|(i, ch)| {
            if escaped {
                escaped = false;
                return None;
            }
            if quoted && ch == '\\' {
                escaped = true;
                return None;
            }
            if ch == '"' {
                quoted = !quoted;
            } else if !quoted {
                if ch == '(' {
                    depth += 1;
                }
                if ch == ')' {
                    depth -= 1;
                }
                if ch.is_whitespace() && depth == 0 {
                    return Some(i);
                }
            }
            None
        })
        .ok_or_else(|| err(line, "C4 relation needs endpoints"))?;
    let (head, body) = (&text[..split], text[split..].trim());
    let (name, attrs) = parse_attributes(head, line)?;
    if !name.starts_with('@')
        || !valid_identifier(&name[1..])
        || attrs.len() != 1
        || attrs[0].0 != "technology"
    {
        return Err(err(line, "relation accepts only @id(technology=\"...\")"));
    }
    let (ends, label) = split_once_top_level(body, ':')
        .ok_or_else(|| err(line, "C4 relation with technology needs a label"))?;
    if label.trim().is_empty() {
        return Err(err(line, "C4 relation needs a label"));
    }
    Ok(format!("{name} {ends}: {} ({})", label.trim(), attrs[0].1))
}
