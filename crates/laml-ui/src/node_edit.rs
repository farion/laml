//! Source-preserving node labels and source reference renaming.
use crate::{
    document::{Document, Edit},
    relation_edit::{quote, tokens},
    source::SourceIndex,
    style_edit::code,
};
use laml::syntax::{Diagram, NodeDecl, NodeKind};
use std::{collections::BTreeMap, ops::Range};
pub(crate) mod rename;
mod style_refs;
#[cfg(test)]
mod tests;
pub use rename::rename_id;

pub(crate) fn diagram(source: &str) -> Result<Diagram, String> {
    let diagram = crate::notation::parse_graph(source).map_err(|e| e.message)?;
    Ok(diagram)
}
pub(crate) fn candidate(source: &str, edits: &[Edit]) -> Result<String, String> {
    let mut doc = Document::new(source.into());
    doc.apply(0, "Object edit", edits.to_vec())
        .map_err(|e| format!("{e:?}"))?;
    Ok(doc.source().into())
}

/// All compiler-recognized node blocks, including notes whose text contains '{'.
pub(crate) fn blocks(
    source: &str,
    diagram: &Diagram,
) -> Result<BTreeMap<usize, Range<usize>>, String> {
    let index = SourceIndex::new(source);
    let nodes: BTreeMap<_, _> = diagram.nodes.iter().map(|n| (n.line, n)).collect();
    let mut stack: Vec<(usize, bool)> = Vec::new();
    let mut result = BTreeMap::new();
    for (i, text) in source.lines().enumerate() {
        let line = i + 1;
        let text = code(text).trim();
        if text == "}"
            && let Some((opening, _)) = stack.pop()
        {
            let start = index.line(opening).ok_or("Missing block start")?.start;
            let end = index.line(line + 1).map_or(source.len(), |r| r.start);
            result.insert(opening, start..end);
        } else if !stack.last().is_some_and(|(_, note)| *note)
            && let Some(node) = nodes.get(&line)
            && text.ends_with('{')
        {
            stack.push((line, node.kind == NodeKind::Note));
        }
    }
    if !stack.is_empty() {
        return Err("Cannot locate object block boundary".into());
    }
    Ok(result)
}

pub fn label(source: &str, id: &str, value: &str) -> Result<Vec<Edit>, String> {
    let diagram = diagram(source)?;
    let node = diagram
        .nodes
        .iter()
        .find(|n| n.id == id)
        .ok_or("Unknown node")?;
    let index = SourceIndex::new(source);
    let range = index.line(node.line).ok_or("Missing node line")?;
    let text = code(&source[range.clone()]);
    if node.kind == NodeKind::Note {
        if text.trim_end().ends_with('{') {
            let block = blocks(source, &diagram)?
                .remove(&node.line)
                .ok_or("Missing note body")?;
            if value.trim().is_empty()
                || value
                    .lines()
                    .any(|line| code(line) != line || code(line).trim() == "}")
            {
                let original = &source[range.clone()];
                let head = code(original).trim_end();
                let suffix = &original[head.len()..];
                let replacement = format!(
                    "{} {}{}{}",
                    head.trim_end_matches('{').trim_end(),
                    quote(value),
                    suffix,
                    if source[block.clone()].ends_with('\n') {
                        if source.contains("\r\n") {
                            "\r\n"
                        } else {
                            "\n"
                        }
                    } else {
                        ""
                    }
                );
                let closing_start = crate::element_edit::closing_start(source, block.end);
                let closing = source[closing_start..block.end].trim_end_matches(['\r', '\n']);
                let comment = &closing[code(closing).len()..];
                let mut replacement = replacement;
                let content_start = index.line(node.line + 1).unwrap().start;
                for annotation in note_comments(source, content_start, closing_start) {
                    let eol = crate::element_edit::newline(source);
                    if !replacement.ends_with('\n') {
                        replacement.push_str(eol);
                    }
                    replacement.push_str(&annotation);
                    replacement.push_str(eol);
                }
                if !comment.is_empty() {
                    let eol = crate::element_edit::newline(source);
                    if !replacement.ends_with('\n') {
                        replacement.push_str(eol);
                    }
                    replacement.push_str(&closing[..closing.len() - closing.trim_start().len()]);
                    replacement.push_str(comment);
                    if source[block.clone()].ends_with('\n') {
                        replacement.push_str(eol);
                    }
                }
                return Ok(vec![Edit {
                    range: block,
                    replacement,
                }]);
            }
            let start = index
                .line(node.line + 1)
                .ok_or("Missing note content")?
                .start;
            let closing = source[..block.end].trim_end_matches(['\r', '\n']);
            let closing_start = closing.rfind('\n').map_or(0, |at| at + 1);
            let newline = if source.contains("\r\n") {
                "\r\n"
            } else {
                "\n"
            };
            let mut content = value
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .split('\n')
                .map(|line| format!("    {line}{newline}"))
                .collect::<String>();
            for annotation in note_comments(source, start, closing_start) {
                content.push_str(&annotation);
                content.push_str(newline);
            }
            return Ok(vec![Edit {
                range: start..closing_start,
                replacement: content,
            }]);
        }
        let at = tokens(text)
            .last()
            .filter(|r| text[(*r).clone()].starts_with('"'))
            .map(|r| r.start)
            .ok_or("Missing inline note text")?;
        return Ok(vec![Edit {
            range: range.start + at..range.start + text.trim_end().len(),
            replacement: quote(value),
        }]);
    }
    let head = node_head(text, node)?;
    let token = &text[head.clone()];
    if let Some(open) = token.find('(') {
        let close = token.rfind(')').ok_or("Missing attributes boundary")?;
        let attrs = &token[open + 1..close];
        let mut offset = open + 1;
        // Attribute separators inside strings must remain opaque.
        let mut quoted = false;
        let mut escaped = false;
        let mut starts = vec![0];
        for (i, ch) in attrs.char_indices() {
            if escaped {
                escaped = false;
            } else if quoted && ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = !quoted;
            } else if !quoted && ch == ';' {
                starts.push(i + 1);
            }
        }
        for (i, start) in starts.iter().enumerate() {
            let end = starts.get(i + 1).map_or(attrs.len(), |next| next - 1);
            let part = &attrs[*start..end];
            if let Some((key, _)) = part.split_once('=')
                && key.trim() == "label"
            {
                let equal = part.find('=').unwrap();
                let tail = &part[equal + 1..];
                let begin = offset + equal + 1 + tail.len() - tail.trim_start().len();
                let end = offset + part.trim_end().len();
                return Ok(vec![Edit {
                    range: range.start + head.start + begin..range.start + head.start + end,
                    replacement: quote(value),
                }]);
            }
            offset += part.len() + 1;
        }
        let at = range.start + head.start + close;
        return Ok(vec![Edit {
            range: at..at,
            replacement: format!(
                "{}label={}",
                if attrs.trim().is_empty() { "" } else { "; " },
                quote(value)
            ),
        }]);
    }
    let at = range.start + head.end;
    Ok(vec![Edit {
        range: at..at,
        replacement: format!("(label={})", quote(value)),
    }])
}

pub(crate) fn node_head(text: &str, node: &NodeDecl) -> Result<Range<usize>, String> {
    let parts = tokens(text);
    let mut first = usize::from(node.explicit);
    while let Some(range) = parts.get(first) {
        let token = &text[range.clone()];
        if matches!(token, "abstract" | "+" | "-" | "#" | "~") {
            first += 1;
        } else {
            break;
        }
    }
    let keywords = [
        "association",
        "class",
        "profile",
        "stereotype",
        "metaclass",
        "package",
        "structure",
        "part",
        "role",
        "component",
        "interface",
        "enum",
        "datatype",
        "device",
        "executionEnvironment",
        "node",
        "artifact",
        "actor",
        "usecase",
        "system",
        "interaction-use",
        "action",
        "objectNode",
        "activityFinal",
        "flowFinal",
        "decision",
        "merge",
        "fork",
        "join",
        "state",
        "initial",
        "final",
        "choice",
        "object",
        "note",
        "junction",
        "external",
        "person",
        "container",
    ];
    while parts.get(first).is_some_and(|r| {
        let token = text[r.clone()].trim_start_matches(['+', '-', '#', '~']);
        keywords.contains(&token)
    }) && first + 1 < parts.len()
    {
        first += 1;
    }
    parts
        .get(first)
        .cloned()
        .ok_or_else(|| "Cannot locate node name".into())
}

fn note_comments(source: &str, start: usize, end: usize) -> Vec<String> {
    source[start..end]
        .lines()
        .filter_map(|line| {
            let body = code(line);
            (body.len() < line.len()).then(|| {
                format!(
                    "{}{}",
                    &line[..line.len() - line.trim_start().len()],
                    &line[body.len()..]
                )
            })
        })
        .collect()
}
