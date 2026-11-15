use super::*;
use laml::syntax::NodePart;

/// IDs are diagram-global addresses. Assigning @NewID preserves the visible name.
pub fn rename_id(source: &str, id: &str, new_id: &str) -> Result<Vec<Edit>, String> {
    if new_id.is_empty()
        || !new_id
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        || !new_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("Use an identifier starting with a letter or underscore".into());
    }
    let diagram = diagram(source)?;
    let node = diagram
        .nodes
        .iter()
        .find(|n| n.id == id)
        .ok_or("Unknown node")?;
    if node.id == new_id {
        return Ok(vec![]);
    }
    let ids = renamed_ids(&diagram, id, new_id);
    let range = SourceIndex::new(source)
        .line(node.line)
        .ok_or("Missing declaration")?;
    let text = code(&source[range.clone()]);
    let first = tokens(text).into_iter().next().ok_or("Missing node ID")?;
    let naming = if node.explicit {
        Edit {
            range: range.start + first.start..range.start + first.end,
            replacement: format!("@{new_id}"),
        }
    } else {
        let at = range.start + first.start;
        Edit {
            range: at..at,
            replacement: format!("@{new_id} "),
        }
    };
    let mut edits = references(source, &diagram, &ids)?;
    edits.push(naming);
    let candidate = candidate(source, &edits)?;
    let updated = super::diagram(&candidate)?;
    // A new global address can shadow previously local references elsewhere.
    // Reject rather than silently changing the meaning of an unrelated edge.
    for (before, after) in diagram.relations.iter().zip(&updated.relations) {
        if after.from != mapped(&ids, &before.from) || after.to != mapped(&ids, &before.to) {
            return Err("New ID changes reference resolution elsewhere. Choose another ID.".into());
        }
    }
    for (before, after) in diagram.attachments.iter().zip(&updated.attachments) {
        if after.source != mapped(&ids, &before.source) {
            return Err("New ID changes an attachment's reference resolution".into());
        }
    }
    for before in &diagram.nodes {
        let after = updated
            .nodes
            .iter()
            .find(|n| n.line == before.line)
            .ok_or("Renamed node is missing")?;
        if after.id != mapped(&ids, &before.id) {
            return Err("Unexpected scoped ID change".into());
        }
    }
    Ok(edits)
}

pub(crate) fn mapped<'a>(ids: &'a BTreeMap<String, String>, id: &'a str) -> &'a str {
    ids.get(id).map_or(id, String::as_str)
}
fn renamed_ids(diagram: &Diagram, id: &str, new_id: &str) -> BTreeMap<String, String> {
    let mut ids = BTreeMap::from([(id.into(), new_id.into())]);
    for node in &diagram.nodes {
        if !node.explicit
            && let Some(parent) = &node.parent
            && let Some(new_parent) = ids.get(parent)
        {
            let suffix = node.id.strip_prefix(parent).unwrap();
            ids.insert(node.id.clone(), format!("{new_parent}{suffix}"));
        }
        if let Some(new) = ids.get(&node.id).cloned() {
            for port in &node.ports {
                if !port.explicit {
                    ids.insert(
                        format!("{}::{}", node.id, port.id),
                        format!("{new}::{}", port.id),
                    );
                }
            }
            for part in &node.parts {
                if let NodePart::Member {
                    id: None,
                    default_id: Some(name),
                    ..
                } = part
                {
                    ids.insert(format!("{}::{name}", node.id), format!("{new}::{name}"));
                }
            }
        }
    }
    ids
}

pub(crate) fn references(
    source: &str,
    diagram: &Diagram,
    ids: &BTreeMap<String, String>,
) -> Result<Vec<Edit>, String> {
    let mut edits = Vec::new();
    let mut replace = |range: Range<usize>, canonical: &str| {
        if let Some(new) = ids.get(canonical)
            && new != canonical
        {
            edits.push(Edit {
                range,
                replacement: new.clone(),
            });
        }
    };
    for relation in &diagram.relations {
        let spans = crate::relation_edit::endpoint_spans(source, relation.line)?;
        let reversed = matches!(
            &source[spans.operator],
            "<-" | "<--" | "<.." | "<|--" | "<|.."
        );
        replace(
            spans.from,
            if reversed {
                &relation.to
            } else {
                &relation.from
            },
        );
        replace(
            spans.to,
            if reversed {
                &relation.from
            } else {
                &relation.to
            },
        );
    }
    let index = SourceIndex::new(source);
    for attachment in &diagram.attachments {
        let range = index
            .line(attachment.line)
            .ok_or("Missing attachment source")?;
        let text = code(&source[range.clone()]);
        let parts = tokens(text);
        let first = usize::from(attachment.name.is_some());
        let first = if attachment.name.is_some() && !text.trim_start().starts_with('@') {
            first + 1
        } else {
            first
        };
        let part = parts.get(first).ok_or("Missing attachment endpoint")?;
        replace(
            range.start + part.start..range.start + part.end,
            &attachment.source,
        );
    }
    let lines: std::collections::BTreeSet<_> = diagram
        .layout
        .iter()
        .map(|r| r.line)
        .chain(diagram.alignments.iter().map(|r| r.line))
        .chain(diagram.scoped_alignments.iter().map(|r| r.line))
        .collect();
    for line in lines {
        let range = index.line(line).ok_or("Missing layout line")?;
        let text = code(&source[range.clone()]);
        for part in tokens(text)
            .into_iter()
            .take_while(|r| &text[r.clone()] != ":")
        {
            let canonical = &text[part.clone()];
            replace(range.start + part.start..range.start + part.end, canonical);
        }
    }
    for rule in &diagram.port_layout {
        let range = index.line(rule.line).ok_or("Missing port rule")?;
        let text = code(&source[range.clone()]);
        if let Some(start) = text.find('[')
            && let Some(end) = text.find(']')
        {
            replace(range.start + start + 1..range.start + end, &rule.id);
        }
    }
    edits.extend(super::style_refs::references(source, diagram, ids)?);
    Ok(edits)
}
