//! Classifier modifiers are independent of their source order.
use super::{ParseError, error};

pub(super) fn take(
    mut source: &str,
    line: usize,
) -> Result<(&str, bool, Option<char>), ParseError> {
    let mut abstract_node = false;
    let mut visibility = None;
    loop {
        if let Some(rest) = source.strip_prefix("abstract")
            && rest.starts_with(char::is_whitespace)
        {
            if abstract_node {
                return Err(error(line, "duplicate abstract classifier modifier"));
            }
            abstract_node = true;
            source = rest.trim_start();
            continue;
        }
        if let Some(prefix) = source
            .chars()
            .next()
            .filter(|ch| matches!(ch, '+' | '-' | '#' | '~'))
        {
            if visibility.is_some() {
                return Err(error(line, "duplicate classifier visibility"));
            }
            visibility = Some(prefix);
            source = source[prefix.len_utf8()..].trim_start();
            continue;
        }
        break;
    }
    Ok((source, abstract_node, visibility))
}
