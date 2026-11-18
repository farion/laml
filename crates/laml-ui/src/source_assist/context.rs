//! Tolerant lexical context. Never requires compilation of the unfinished buffer.
use std::ops::Range;
#[derive(Default, Debug)]
pub(super) struct Context {
    pub section: String,
    pub heading: String,
    pub block_start: usize,
    pub attributes: Option<usize>,
    pub opaque: bool,
    pub quoted: bool,
    pub quote_start: Option<usize>,
    pub line: String,
}
pub(super) fn context(source: &str, caret: usize) -> Context {
    let mut out = Context::default();
    if caret > source.len() || !source.is_char_boundary(caret) {
        out.opaque = true;
        return out;
    }
    let mut stack: Vec<(usize, String)> = vec![];
    let mut parens = vec![];
    let (mut quote, mut escape, mut comment) = (false, false, false);
    let mut line_start = 0;
    let mut quote_start = None;
    let mut free_note = false;
    for (i, c) in source[..caret].char_indices() {
        if c == '\n' {
            line_start = i + 1;
            comment = false;
            quote = false; // recover unfinished strings at the next line
            escape = false;
            continue;
        }
        if comment {
            continue;
        }
        if free_note {
            if c != '}' || !source[line_start..i].trim().is_empty() {
                continue;
            }
            free_note = false;
        }
        if quote {
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                quote = false;
            }
            continue;
        }
        if source[i..].starts_with("//") {
            comment = true;
            continue;
        }
        if c == '"' {
            quote_start = Some(i);
            quote = true;
            continue;
        }
        match c {
            '(' => parens.push(i),
            ')' => {
                parens.pop();
            }
            '{' => {
                let heading = source[line_start..i].trim().to_owned();
                free_note = heading.split_whitespace().take(2).any(|w| w == "note")
                    && stack.first().is_some_and(|(_, h)| h == "semantics");
                stack.push((i, heading));
            }
            '}' => {
                stack.pop();
            }
            _ => {}
        }
    }
    out.section = stack.first().map(|(_, h)| h.clone()).unwrap_or_default();
    if let Some((i, h)) = stack.last() {
        out.block_start = i + 1;
        out.heading = h.clone();
    }
    out.attributes = parens.last().copied();
    out.line = source[line_start..caret].trim_start().to_owned();
    // Unquoted labels consume the rest of a semantic statement. A named event's
    // initial colon is not a label separator.
    let relation_label = crate::relation_edit::OPERATORS.iter().any(|op| {
        out.line
            .find(op)
            .is_some_and(|i| out.line[i + op.len()..].contains(':'))
    });
    let note_label =
        out.line.contains(": note ") || out.line.starts_with("note ") && out.line.contains(':');
    out.quoted = quote;
    out.quote_start = quote.then_some(quote_start).flatten();
    let structured_string = out.quote_start.is_some_and(|at| {
        let key = source[line_start..at]
            .trim_end()
            .trim_end_matches(['=', ':'])
            .trim_end()
            .rsplit([';', '(', ' ', '\t'])
            .next()
            .unwrap_or("");
        matches!(key, "icon" | "appearance")
    });
    out.opaque = comment
        || quote && !structured_string
        || free_note
        || out.section == "semantics" && (relation_label || note_label);
    out
}
pub(super) fn word(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '@')
}
pub(super) fn token(source: &str, caret: usize) -> Range<usize> {
    let start = source[..caret]
        .char_indices()
        .rev()
        .take_while(|(_, c)| word(*c))
        .last()
        .map_or(caret, |(i, _)| i);
    let end = source[caret..]
        .char_indices()
        .take_while(|(_, c)| word(*c))
        .last()
        .map_or(caret, |(i, c)| caret + i + c.len_utf8());
    start..end
}
/// Lexical text with strings/comments blanked, preserving byte positions.
pub(super) fn code(source: &str) -> String {
    let (mut quote, mut escaped, mut comment) = (false, false, false);
    let mut out = String::with_capacity(source.len());
    for (i, c) in source.char_indices() {
        if c == '\n' {
            quote = false;
            comment = false;
            escaped = false;
            out.push(c);
            continue;
        }
        if !quote && source[i..].starts_with("//") {
            comment = true;
        }
        let hidden = quote || comment || c == '"';
        if !comment {
            if escaped {
                escaped = false;
            } else if quote && c == '\\' {
                escaped = true;
            } else if c == '"' {
                quote = !quote;
            }
        }
        if hidden {
            out.extend(std::iter::repeat_n(' ', c.len_utf8()));
        } else {
            out.push(c);
        }
    }
    out
}
