//! Token-targeted relation edits; source lines identify unnamed and parallel arrows.
use crate::{document::Edit, source::SourceIndex, style_edit::code};
use laml::syntax::{Diagram, EndPosition, RelationDecl};
use std::ops::Range;
mod label_modes;
mod mapping;
mod sides;
mod types;
pub use types::{TYPES, change_direction, change_type, direction, directional, relation_type};
pub use label_modes::label_modes;
pub use mapping::remap_line;
#[cfg(test)]
mod tests;
pub use sides::{endpoint_side, endpoint_sides};

pub const OPERATORS: &[&str] = &[
    "->", "<-", "<->", "--", "..", "-->", "<--", "..>", "<..", "--^>", "--|>", "<|--", "..|>",
    "<|..", "*--", "--*", "o--", "--o", "*-->", "o-->", "==", "=>",
];

pub(crate) fn declaration(source: &str, line: usize) -> Result<(Diagram, RelationDecl), String> {
    let diagram = crate::notation::parse_graph(source).map_err(|e| e.message)?;
    let relation = diagram
        .relations
        .iter()
        .find(|r| r.line == line)
        .cloned()
        .ok_or("Relation declaration no longer exists")?;
    Ok((diagram, relation))
}

/// Lexical ranges only; meaning and validation always come from the compiler.
/// Delimiters in quoted labels, attributes, qualifiers and qualified IDs are opaque.
pub(crate) fn tokens(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let (mut quoted, mut escaped, mut depth, mut start) = (false, false, 0usize, None);
    for (i, ch) in text.char_indices() {
        if !quoted
            && depth == 0
            && (ch.is_whitespace()
                || (ch == ':' && !text[..i].ends_with(':') && !text[i + 1..].starts_with(':')))
        {
            if let Some(s) = start.take() {
                ranges.push(s..i);
            }
            if ch == ':' {
                ranges.push(i..i + 1);
            }
            continue;
        }
        start.get_or_insert(i);
        if escaped {
            escaped = false;
        } else if quoted && ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            quoted = !quoted;
        } else if !quoted && ch == '(' {
            depth += 1;
        } else if !quoted && ch == ')' {
            depth = depth.saturating_sub(1);
        }
    }
    if let Some(s) = start {
        ranges.push(s..text.len());
    }
    ranges
}

pub(crate) struct Spans {
    pub from: Range<usize>,
    pub to: Range<usize>,
    pub operator: Range<usize>,
    pub label: Option<Range<usize>>,
    pub start: usize,
    pub end: usize,
}
pub(crate) fn spans(source: &str, line: usize) -> Result<Spans, String> {
    let range = SourceIndex::new(source)
        .line(line)
        .ok_or("Missing declaration line")?;
    let text = code(&source[range.clone()]);
    let parts = tokens(text);
    let op = parts
        .iter()
        .position(|r| OPERATORS.contains(&&text[r.clone()]))
        .ok_or("Cannot locate relationship operator")?;
    let op_range = parts[op].clone();
    let label_at = parts
        .iter()
        .skip(op + 1)
        .find(|r| &text[(*r).clone()] == ":");
    let end = label_at.map_or(text.trim_end().len(), |r| r.start);
    let tail = parts
        .iter()
        .skip(op + 1)
        .take_while(|r| r.end <= end)
        .take_while(|r| !text[(*r).clone()].starts_with('«') && !text[(*r).clone()].starts_with("<<"))
        .last()
        .ok_or("Missing relation endpoint")?
        .clone();
    let absolute = |r: Range<usize>| range.start + r.start..range.start + r.end;
    Ok(Spans {
        from: absolute(
            parts
                .get(op.wrapping_sub(1))
                .ok_or("Missing relation source")?
                .clone(),
        ),
        // Adornments occur between first endpoint and operator. Find the body start.
        to: absolute(tail),
        operator: absolute(op_range),
        label: label_at.map(|r| range.start + r.start..range.start + text.trim_end().len()),
        start: range.start + text.len() - text.trim_start().len(),
        end: range.start + text.trim_end().len(),
    })
}

pub(crate) fn endpoint_spans(source: &str, line: usize) -> Result<Spans, String> {
    let mut result = spans(source, line)?;
    let range = SourceIndex::new(source)
        .line(line)
        .ok_or("Missing source")?;
    let text = code(&source[range.clone()]);
    let parts = tokens(text);
    let first = if text.trim_start().starts_with('@') {
        1
    } else {
        parts
            .iter()
            .position(|r| &text[r.clone()] == ":" && range.start + r.start < result.operator.start)
            .map_or(0, |i| i + 1)
    };
    let first = first + usize::from(crate::notation::Notation::of(source) == crate::notation::Notation::Activity && parts.get(first).is_some_and(|r| ["control", "object"].contains(&&text[r.clone()])));
    let token = parts.get(first).ok_or("Missing source endpoint")?;
    result.from = range.start + token.start..range.start + token.end;
    Ok(result)
}

pub fn operator(source: &str, line: usize) -> Result<String, String> {
    let span = spans(source, line)?;
    Ok(source[span.operator].into())
}
pub(crate) fn quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\r', "\\r")
            .replace('\n', "\\n")
            .replace('\t', "\\t")
    )
}
pub fn label(source: &str, line: usize, value: &str) -> Result<Vec<Edit>, String> {
    declaration(source, line)?;
    let span = spans(source, line)?;
    let replacement = if value.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", quote(value))
    };
    let (range, replacement) = match span.label {
        Some(range) => (range, replacement),
        None => (
            span.end..span.end,
            if replacement.is_empty() {
                replacement
            } else {
                format!(" {replacement}")
            },
        ),
    };
    Ok(vec![Edit { range, replacement }])
}
pub fn change_operator(source: &str, line: usize, value: &str) -> Result<Vec<Edit>, String> {
    declaration(source, line)?;
    if !OPERATORS.contains(&value) {
        return Err("Unsupported GNE operator".into());
    }
    Ok(vec![Edit {
        range: spans(source, line)?.operator,
        replacement: value.into(),
    }])
}
pub fn reconnect(
    source: &str,
    line: usize,
    end: EndPosition,
    node: &str,
) -> Result<Vec<Edit>, String> {
    let (diagram, relation) = declaration(source, line)?;
    if !diagram.nodes.iter().any(|n| n.has_endpoint(node)) {
        return Err("Unknown target node".into());
    }
    let span = endpoint_spans(source, line)?;
    let reversed = matches!(
        &source[span.operator.clone()],
        "<-" | "<--" | "<.." | "<|--" | "<|.."
    );
    let first = (end == EndPosition::Source) != reversed;
    let range = if first { span.from } else { span.to };
    // Fully qualified IDs resolve root-first; verify the intended endpoint after patching.
    let edits = vec![Edit {
        range,
        replacement: node.into(),
    }];
    let mut candidate = crate::document::Document::new(source.into());
    candidate
        .apply(0, "Reconnect", edits.clone())
        .map_err(|e| format!("{e:?}"))?;
    let (_, updated) = declaration(candidate.source(), relation.line)?;
    let actual = if end == EndPosition::Source {
        updated.from
    } else {
        updated.to
    };
    if actual != node {
        return Err("Target is shadowed in this relation's scope".into());
    }
    Ok(edits)
}

pub(crate) fn named(
    source: &str,
    diagram: &Diagram,
    relation: &RelationDecl,
) -> Result<(String, Vec<Edit>), String> {
    if let Some(name) = &relation.name {
        return Ok((name.clone(), vec![]));
    }
    let prefix = if diagram.settings.strict_case {
        "relation"
    } else {
        "Relation"
    };
    let id = crate::semantic_edit::fresh_id(diagram, prefix);
    let start = spans(source, relation.line)?.start;
    // Legacy unnamed ': A -> B' prefix already occupies the naming slot.
    let replacement = if source[start..].starts_with(':') {
        id.clone()
    } else {
        format!("@{id} ")
    };
    Ok((
        id,
        vec![Edit {
            range: start..start,
            replacement,
        }],
    ))
}

pub fn property(
    source: &str,
    line: usize,
    name: &str,
    value: Option<&str>,
) -> Result<Vec<Edit>, String> {
    properties(source, &[line], name, value)
}

pub fn properties(
    source: &str,
    lines: &[usize],
    name: &str,
    value: Option<&str>,
) -> Result<Vec<Edit>, String> {
    if !crate::properties::RELATION_PROPERTIES.contains(&name) {
        return Err("Unsupported relation property".into());
    }
    let mut diagram = crate::notation::parse_graph(source).map_err(|e| e.message)?;
    let mut edits = Vec::new();
    let mut selectors = Vec::new();
    for line in lines
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
    {
        let (_, relation) = declaration(source, line)?;
        if value.is_none() && relation.name.is_none() {
            continue;
        }
        let (id, naming) = named(source, &diagram, &relation)?;
        if let Some(r) = diagram.relations.iter_mut().find(|r| r.line == line) {
            r.name = Some(id.clone());
        }
        edits.extend(naming);
        selectors.push(format!("relation[{id}]"));
    }
    edits.extend(crate::style_edit::local_property(
        source, &selectors, name, value,
    )?);
    Ok(edits)
}

/// Edit the full existing declaration while retaining its named identity and neighbors.
pub fn definition(source: &str, line: usize, value: &str) -> Result<Vec<Edit>, String> {
    if value.contains(['\r', '\n']) {
        return Err("Expected one relation declaration".into());
    }
    let (before, old) = declaration(source, line)?;
    let range = SourceIndex::new(source)
        .line(line)
        .ok_or("Missing relation")?;
    let text = code(&source[range.clone()]);
    let start = range.start + text.len() - text.trim_start().len();
    let edits = vec![Edit {
        range: start..range.start + text.trim_end().len(),
        replacement: value.trim().into(),
    }];
    let candidate = crate::node_edit::candidate(source, &edits)?;
    let (after, new) = declaration(&candidate, line)?;
    if old.name.is_some() && old.name != new.name {
        return Err(
            "Keep the named relation ID stable; it is used by styles, layout and attachments"
                .into(),
        );
    }
    if before.nodes != after.nodes
        || before
            .relations
            .iter()
            .filter(|r| r.line != line)
            .ne(after.relations.iter().filter(|r| r.line != line))
    {
        return Err("Declaration would change other elements".into());
    }
    Ok(edits)
}

pub fn definition_text(source: &str, line: usize) -> Option<String> {
    let range = SourceIndex::new(source).line(line)?;
    Some(code(&source[range]).trim().into())
}
