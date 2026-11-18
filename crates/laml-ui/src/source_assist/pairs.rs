//! Delimiter edits are explicit single document transactions, not egui undo state.
use super::context;
use crate::text_tools::Change;
use std::ops::Range;
pub fn typed(source: &str, selection: Range<usize>, c: char) -> Option<Change> {
    if selection.end > source.len()
        || !source.is_char_boundary(selection.start)
        || !source.is_char_boundary(selection.end)
    {
        return None;
    }
    if !"{}[]()\"".contains(c) {
        return None;
    }
    let ctx = context::context(source, selection.start);
    let next = source[selection.end..].chars().next();
    // Typing an existing closer moves the caret without changing source.
    if selection.is_empty()
        && next == Some(c)
        && ("}])".contains(c) && !ctx.opaque || c == '"' && ctx.quoted)
        && !escaped(source, selection.start)
    {
        return Some(Change {
            edits: vec![],
            selection: selection.end + c.len_utf8()..selection.end + c.len_utf8(),
        });
    }
    if ctx.opaque || ctx.quoted {
        return None;
    }
    let close = match c {
        '{' => '}',
        '[' => ']',
        '(' => ')',
        '"' => '"',
        _ => return None,
    };
    if c == '"' && escaped(source, selection.start) {
        return None;
    }
    // Do not inject closers in front of identifiers while typing ordinary code.
    if selection.is_empty() && next.is_some_and(|n| n.is_alphanumeric() || n == '_') {
        return None;
    }
    let inner = &source[selection.clone()];
    let replacement = format!("{c}{inner}{close}");
    let start = selection.start + c.len_utf8();
    Some(Change {
        selection: start..start + inner.len(),
        edits: vec![crate::document::Edit {
            range: selection,
            replacement,
        }],
    })
}
pub fn backspace(source: &str, selection: Range<usize>) -> Option<Change> {
    if !selection.is_empty()
        || selection.start == 0
        || selection.end >= source.len()
        || !source.is_char_boundary(selection.start)
    {
        return None;
    }
    let p = selection.start;
    let open = source[..p].chars().next_back()?;
    let close = source[p..].chars().next()?;
    if !matches!(
        (open, close),
        ('{', '}') | ('[', ']') | ('(', ')') | ('"', '"')
    ) {
        return None;
    }
    let ctx = context::context(source, p - open.len_utf8());
    if ctx.opaque {
        return None;
    }
    let at = p - open.len_utf8();
    Some(Change {
        edits: vec![crate::document::Edit {
            range: at..p + close.len_utf8(),
            replacement: String::new(),
        }],
        selection: at..at,
    })
}
fn escaped(source: &str, at: usize) -> bool {
    source[..at]
        .chars()
        .rev()
        .take_while(|c| *c == '\\')
        .count()
        % 2
        == 1
}
