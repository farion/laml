//! Notation-aware, source-preserving structural transactions for specialized diagrams.
use crate::{
    document::{Document, Edit},
    source::SourceIndex,
    style_edit::code,
};
use std::ops::Range;
pub mod bpmn;
pub mod bpmn_inline;
pub mod catalog;
mod executions;
pub mod fields;
mod identity;
mod properties;
mod references;
mod relations;
pub mod sequence;
pub mod sequence_activation;
pub mod sequence_align;
pub mod sequence_controls;
pub mod sequence_create;
mod sequence_dependencies;
pub mod sequence_destruction;
pub mod sequence_docking;
pub mod sequence_drag;
pub mod sequence_fragments;
mod sequence_horizontal;
pub mod sequence_insert;
pub mod sequence_lifetime;
pub mod sequence_lookup;
pub mod sequence_options;
pub mod sequence_participants;
pub mod sequence_recipes;
pub mod sequence_reconnect;
mod sequence_resize;
pub mod sequence_scope;
pub mod sequence_scoped;
mod sequence_snap;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod sequence_target_geometry;
pub mod sequence_targets;
pub mod timing;
mod transactions;
pub use identity::rename;
pub use properties::*;
pub use relations::relationship_type;
pub use transactions::*;

#[derive(Clone, Debug)]
pub struct Record {
    pub line: usize,
    pub section: String,
    pub heading: String,
    pub range: Range<usize>,
    pub block: Range<usize>,
    pub parent: Option<usize>,
    pub opens: bool,
}
/// Balanced source records; braces in strings/comments and note text are opaque.
/// The compiler remains the authority on whether a structural edit is legal.
pub fn records(source: &str) -> Vec<Record> {
    let index = SourceIndex::new(source);
    let mut result: Vec<Record> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut section = String::new();
    let mut note = None;
    for (i, raw) in source.lines().enumerate() {
        let line = i + 1;
        let text = code(raw).trim();
        let range = index.line(line).unwrap();
        let end = index.line(line + 1).map_or(source.len(), |r| r.start);
        if let Some(note_line) = note {
            if text != "}" {
                continue;
            }
            note = None;
            if let Some(at) = stack.pop() {
                result[at].block.end = end;
            }
            let _ = note_line;
            continue;
        }
        if text.is_empty() {
            continue;
        }
        if text == "}" {
            if let Some(at) = stack.pop() {
                result[at].block.end = end;
            } else {
                section.clear();
            }
            continue;
        }
        if stack.is_empty()
            && ["settings {", "semantics {", "layout {", "styling {"].contains(&text)
        {
            section = text.trim_end_matches(" {").into();
            continue;
        }
        if section.is_empty() {
            continue;
        }
        let tokens = fields::tokens(text);
        let relation = tokens.iter().any(|r| {
            let s = &text[r.clone()];
            crate::relation_edit::OPERATORS.contains(&s) || ["->>", "~~>"].contains(&s)
        });
        let opens = !relation && tokens.last().is_some_and(|r| &text[r.clone()] == "{");
        let at = result.len();
        result.push(Record {
            line,
            section: section.clone(),
            heading: text.trim_end_matches('{').trim().into(),
            range: range.clone(),
            block: range.start..end,
            parent: stack.last().map(|i| result[*i].line),
            opens,
        });
        if opens {
            stack.push(at);
            if text.starts_with("note ") {
                note = Some(line);
            }
        }
    }
    result
}
pub fn record(source: &str, line: usize) -> Result<Record, String> {
    let all = records(source);
    let mut r = all
        .iter()
        .find(|r| r.line == line)
        .cloned()
        .ok_or_else(|| "The selected construct no longer exists".to_owned())?;
    if r.opens && crate::notation::Notation::of(source) == crate::notation::Notation::Sequence {
        let body = r
            .heading
            .split_once(':')
            .map_or(r.heading.as_str(), |(_, tail)| tail.trim());
        if body.starts_with("alt ")
            || body.starts_with("par ")
            || body.contains(" alt ")
            || body.contains(" par ")
        {
            for next in all
                .iter()
                .filter(|s| s.range.start >= r.range.start && s.parent == r.parent)
            {
                if next.line == r.line || next.range.start < r.block.end {
                    continue;
                }
                if next.heading.starts_with("else ") || next.heading.starts_with("and ") {
                    r.block.end = next.block.end;
                } else {
                    break;
                }
            }
        }
    }
    Ok(r)
}
pub fn apply(source: &str, edits: &[Edit]) -> Result<String, String> {
    let mut doc = Document::new(source.into());
    doc.apply(0, "Notation edit", edits.to_vec())
        .map_err(|e| format!("{e:?}"))?;
    Ok(doc.source().into())
}
pub fn insert(
    source: &str,
    section: &str,
    parent: Option<usize>,
    before: Option<usize>,
    content: &str,
) -> Result<Vec<Edit>, String> {
    let newline = crate::element_edit::newline(source);
    let (at, indent) = if let Some(line) = before {
        let r = record(source, line)?;
        let raw = &source[r.range.clone()];
        (
            r.range.start,
            raw[..raw.len() - raw.trim_start().len()].to_owned(),
        )
    } else if let Some(line) = parent {
        let r = record(source, line)?;
        if r.section != section {
            return Err("Choose a container in the same section".into());
        }
        let raw = &source[r.range.clone()];
        if !r.opens {
            let body = code(raw).trim_end();
            let indent = &raw[..raw.len() - raw.trim_start().len()];
            let comment = &raw[body.len()..];
            let contents = content
                .replace("\r\n", "\n")
                .lines()
                .map(|l| format!("{indent}    {l}{newline}"))
                .collect::<String>();
            return Ok(vec![Edit {
                range: r.block,
                replacement: format!("{body} {{{comment}{newline}{contents}{indent}}}{newline}"),
            }]);
        }
        (
            crate::element_edit::closing_start(source, r.block.end),
            format!("{}    ", &raw[..raw.len() - raw.trim_start().len()]),
        )
    } else {
        let mut active = false;
        let mut depth = 0usize;
        let mut found = None;
        let index = SourceIndex::new(source);
        for (i, raw) in source.lines().enumerate() {
            let text = code(raw).trim();
            if !active {
                if text == format!("{section} {{") {
                    active = true;
                }
                continue;
            }
            if text == "}" {
                if depth == 0 {
                    found = Some(index.line(i + 1).unwrap().start);
                    break;
                }
                depth -= 1;
            } else if fields::tokens(text)
                .last()
                .is_some_and(|r| &text[r.clone()] == "{")
            {
                depth += 1;
            }
        }
        if let Some(at) = found {
            (at, "    ".into())
        } else {
            let prefix = if source.is_empty() || source.ends_with('\n') {
                ""
            } else {
                newline
            };
            return Ok(vec![Edit {
                range: source.len()..source.len(),
                replacement: format!(
                    "{prefix}{section} {{{newline}{}{newline}}}{newline}",
                    content
                        .replace("\r\n", "\n")
                        .lines()
                        .map(|l| format!("    {l}"))
                        .collect::<Vec<_>>()
                        .join(newline)
                ),
            }]);
        }
    };
    let content = content
        .replace("\r\n", "\n")
        .lines()
        .map(|l| format!("{indent}{l}{newline}"))
        .collect::<String>();
    Ok(vec![Edit {
        range: at..at,
        replacement: content,
    }])
}

pub fn references_scope_for_creation(source: &str, record: &Record) -> String {
    references::scope(source, record, crate::notation::Notation::of(source))
}

#[cfg(test)]
mod sequence_interaction_tests;

#[cfg(test)]
mod sequence_docking_tests;
