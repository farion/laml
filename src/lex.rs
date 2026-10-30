//! Line-oriented lexical helpers for the initial grammar subset.

pub(crate) fn without_comment(input: &str) -> &str {
    let mut quoted = false;
    let mut escaped = false;
    for (index, ch) in input.char_indices() {
        if escaped {
            escaped = false;
        } else if quoted && ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            quoted = !quoted;
        } else if !quoted {
            // A comment needs a token boundary; https:// in a label is not a comment.
            let slash_comment = input[index..].starts_with("//")
                && (index == 0
                    || input[..index]
                        .chars()
                        .next_back()
                        .is_some_and(char::is_whitespace));
            if slash_comment {
                return &input[..index];
            }
        }
    }
    input
}

/// Split at a delimiter outside double quotes and parentheses.
pub(crate) fn split_top_level(input: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for (index, ch) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quoted && ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            quoted = !quoted;
        } else if !quoted {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth = depth.saturating_sub(1);
            } else if ch == delimiter && depth == 0 {
                parts.push(&input[start..index]);
                start = index + ch.len_utf8();
            }
        }
    }
    parts.push(&input[start..]);
    parts
}

pub(crate) fn split_once_top_level(input: &str, delimiter: char) -> Option<(&str, &str)> {
    let parts = split_top_level(input, delimiter);
    if parts.len() < 2 {
        return None;
    }
    let offset = parts[0].len();
    Some((&input[..offset], &input[offset + delimiter.len_utf8()..]))
}

/// Find a relation arrow outside strings and attribute parentheses.
pub(crate) fn find_arrow(input: &str) -> Option<usize> {
    let mut quoted = false;
    let mut escaped = false;
    let mut depth = 0usize;
    for (index, ch) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quoted && ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            quoted = !quoted;
        } else if !quoted {
            match ch {
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                _ if depth == 0
                    && (input[index..].starts_with("<->")
                        || input[index..].starts_with("->")
                        || input[index..].starts_with("<-")
                        || input[index..].starts_with("--")) =>
                {
                    return Some(index);
                }
                _ => {}
            }
        }
    }
    None
}

pub(crate) fn valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

pub(crate) fn quoted_string(value: &str) -> Result<String, &'static str> {
    let body = value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .ok_or("expected a double-quoted string")?;
    let mut output = String::new();
    let mut chars = body.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }
        let escaped = chars.next().ok_or("unfinished string escape")?;
        output.push(match escaped {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '"' => '"',
            '\\' => '\\',
            _ => return Err("unsupported string escape"),
        });
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::without_comment;

    #[test]
    fn slash_comments_require_a_token_boundary() {
        assert_eq!(without_comment("// whole line"), "");
        assert_eq!(without_comment("A // comment"), "A ");
        assert_eq!(
            without_comment("A -> B : https://example.test/path"),
            "A -> B : https://example.test/path"
        );
        assert_eq!(
            without_comment("A -> B : https://example.test/path // comment"),
            "A -> B : https://example.test/path "
        );
    }

    #[test]
    fn slash_comments_inside_quotes_are_visible_text() {
        assert_eq!(
            without_comment("A(label=\"https://example.test//path\") // comment"),
            "A(label=\"https://example.test//path\") "
        );
        assert_eq!(
            without_comment("A(label=\"escaped \\\" // text\") // comment"),
            "A(label=\"escaped \\\" // text\") "
        );
    }
}
