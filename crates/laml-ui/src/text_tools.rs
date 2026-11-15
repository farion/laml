//! Literal search and explicit source commands. All offsets are UTF-8 bytes.
use crate::{document::Edit, source::SourceIndex};
use std::ops::Range;

pub fn matches(source: &str, needle: &str, case_sensitive: bool) -> Vec<Range<usize>> {
    if needle.is_empty() {
        return vec![];
    }
    if case_sensitive {
        return source
            .match_indices(needle)
            .map(|(i, s)| i..i + s.len())
            .collect();
    }
    let mut lowered = String::new();
    let mut boundaries = vec![(0, 0)];
    for (i, c) in source.char_indices() {
        lowered.extend(c.to_lowercase());
        boundaries.push((lowered.len(), i + c.len_utf8()));
    }
    lowered
        .match_indices(
            &needle
                .chars()
                .flat_map(char::to_lowercase)
                .collect::<String>(),
        )
        .filter_map(|(i, s)| {
            let a = boundaries.binary_search_by_key(&i, |b| b.0).ok()?;
            let b = boundaries
                .binary_search_by_key(&(i + s.len()), |b| b.0)
                .ok()?;
            Some(boundaries[a].1..boundaries[b].1)
        })
        .collect()
}
pub fn replacements(source: &str, needle: &str, value: &str, case_sensitive: bool) -> Vec<Edit> {
    matches(source, needle, case_sensitive)
        .into_iter()
        .map(|range| Edit {
            range,
            replacement: value.into(),
        })
        .collect()
}
#[derive(Clone, Copy)]
pub enum Action {
    Comment,
    Indent,
    Outdent,
    Newline,
}
pub struct Change {
    pub edits: Vec<Edit>,
    pub selection: Range<usize>,
}

pub fn command(source: &str, selection: Range<usize>, action: Action) -> Result<Change, String> {
    if selection.start > selection.end
        || selection.end > source.len()
        || !source.is_char_boundary(selection.start)
        || !source.is_char_boundary(selection.end)
    {
        return Err("Invalid source selection".into());
    }
    let index = SourceIndex::new(source);
    let first_line = index.line_at(selection.start).unwrap();
    let first = index.line(first_line).unwrap();
    let last_line = index
        .line_at(if selection.is_empty() {
            selection.end
        } else {
            source[..selection.end]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i)
        })
        .unwrap();
    if matches!(action, Action::Newline) {
        let prefix = &source[first.start..selection.start];
        let indent: String = prefix
            .chars()
            .take_while(|c| matches!(c, ' ' | '\t'))
            .collect();
        let open = last_code_delimiter(prefix);
        let extra = if open.is_some() { "    " } else { "" };
        let eol = source.find('\n').map_or("\n", |p| {
            if p > 0 && source.as_bytes()[p - 1] == b'\r' {
                "\r\n"
            } else {
                "\n"
            }
        });
        let mut replacement = format!("{eol}{indent}{extra}");
        let caret = selection.start + replacement.len();
        if open.is_some_and(|c| {
            source[selection.end..].starts_with(match c {
                '{' => '}',
                '[' => ']',
                _ => ')',
            })
        }) {
            replacement.push_str(&format!("{eol}{indent}"));
        }
        return Ok(Change {
            edits: vec![Edit {
                range: selection,
                replacement,
            }],
            selection: caret..caret,
        });
    }
    if selection.is_empty() && matches!(action, Action::Indent) {
        let column = source[first.start..selection.start]
            .chars()
            .fold(0, |n, c| if c == '\t' { (n / 4 + 1) * 4 } else { n + 1 });
        let replacement = " ".repeat(4 - column % 4);
        let caret = selection.start + replacement.len();
        return Ok(Change {
            edits: vec![Edit {
                range: selection,
                replacement,
            }],
            selection: caret..caret,
        });
    }
    let lines: Vec<_> = (first_line..=last_line)
        .map(|line| index.line(line).unwrap())
        .collect();
    let uncomment = lines
        .iter()
        .filter(|r| !source[(*r).clone()].trim().is_empty())
        .all(|r| {
            source[(*r).clone()]
                .trim_start_matches([' ', '\t'])
                .starts_with("//")
        });
    let mut edits = vec![];
    for line in &lines {
        let text = &source[line.clone()];
        let indent = text.len() - text.trim_start_matches([' ', '\t']).len();
        let start = line.start;
        match action {
            Action::Comment if text.trim().is_empty() && !selection.is_empty() => {}
            Action::Comment if uncomment && text[indent..].starts_with("//") => {
                let n = if text[indent + 2..].starts_with(' ') {
                    3
                } else {
                    2
                };
                edits.push(Edit {
                    range: start + indent..start + indent + n,
                    replacement: String::new(),
                });
            }
            Action::Comment => edits.push(Edit {
                range: start + indent..start + indent,
                replacement: "// ".into(),
            }),
            Action::Indent => edits.push(Edit {
                range: start..start,
                replacement: "    ".into(),
            }),
            Action::Outdent => {
                let n = if text.starts_with('\t') {
                    1
                } else {
                    text.bytes().take_while(|b| *b == b' ').count().min(4)
                };
                if n > 0 {
                    edits.push(Edit {
                        range: start..start + n,
                        replacement: String::new(),
                    });
                }
            }
            Action::Newline => unreachable!(),
        }
    }
    let end = if selection.is_empty() {
        selection.end
    } else {
        lines.last().unwrap().end
    };
    let start = if selection.is_empty() {
        selection.start
    } else {
        first.start
    };
    let map = |offset: usize| {
        let mut delta = 0isize;
        for edit in &edits {
            if edit.range.end <= offset {
                delta += edit.replacement.len() as isize - edit.range.len() as isize;
            } else if edit.range.start < offset {
                return (edit.range.start as isize + delta) as usize;
            }
        }
        (offset as isize + delta) as usize
    };
    let selection = if selection.is_empty() {
        let p = map(start);
        p..p
    } else {
        first.start..map(end)
    };
    Ok(Change { edits, selection })
}
fn last_code_delimiter(text: &str) -> Option<char> {
    let mut quote = false;
    let mut escape = false;
    let mut last = None;
    for (i, c) in text.char_indices() {
        if quote {
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                quote = false;
            }
            last = None;
            continue;
        }
        if text[i..].starts_with("//")
            && (i == 0
                || text[..i]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace))
        {
            return None;
        }
        if c == '"' {
            quote = true;
            last = None;
        } else if !c.is_whitespace() {
            last = Some(c);
        }
    }
    last.filter(|c| matches!(c, '{' | '[' | '('))
}

#[cfg(test)]
mod tests;
