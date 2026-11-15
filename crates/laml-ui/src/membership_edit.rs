//! Explicit membership transactions. Preserve relations/rules; reject invalid rebasing.
use crate::{
    document::Edit,
    element_edit::{closing_start, newline},
    node_edit,
    properties::port_id,
    relation_edit,
    source::SourceIndex,
    source_patch,
    style_edit::code,
};
use laml::syntax::{Diagram, NodePart, StyleEntry};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

#[derive(Clone, Debug)]
pub struct Move {
    pub id: String,
    pub edits: Vec<Edit>,
    pub changes: Vec<String>,
}

pub fn plan(source: &str, id: &str, parent: Option<&str>) -> Result<Move, String> {
    let diagram = node_edit::diagram(source)?;
    let node = diagram
        .nodes
        .iter()
        .find(|n| n.id == id)
        .ok_or("Missing selected node")?;
    if node.parent.as_deref() == parent {
        return Err("Node already belongs to this container.".into());
    }
    if let Some(parent) = parent
        && !diagram.nodes.iter().any(|n| {
            n.id == parent
                && crate::notation::container_kind(n.kind)
                && node_edit::blocks(source, &diagram).is_ok_and(|b| b.contains_key(&n.line))
        })
    {
        return Err("Choose a notation-compatible container block.".into());
    }
    let mut moved = BTreeSet::from([id.to_string()]);
    for n in &diagram.nodes {
        if n.parent.as_ref().is_some_and(|p| moved.contains(p)) {
            moved.insert(n.id.clone());
        }
    }
    if parent.is_some_and(|p| moved.contains(p)) {
        return Err("A container cannot move into itself or its descendants.".into());
    }
    let index = SourceIndex::new(source);
    let blocks = node_edit::blocks(source, &diagram)?;
    let line = index.line(node.line).ok_or("Missing source declaration")?;
    let range = blocks
        .get(&node.line)
        .cloned()
        .unwrap_or(line.start..index.line(node.line + 1).map_or(source.len(), |r| r.start));
    let mut mapping = BTreeMap::new();
    for n in diagram.nodes.iter().filter(|n| moved.contains(&n.id)) {
        let new = if n.explicit {
            n.id.clone()
        } else if n.id == id {
            let local = n.id.rsplit('.').next().unwrap();
            parent.map_or_else(|| local.into(), |p| format!("{p}.{local}"))
        } else {
            let old_parent = n.parent.as_deref().ok_or("Missing nested parent")?;
            let new_parent = node_edit::rename::mapped(&mapping, old_parent);
            format!(
                "{new_parent}{}",
                n.id.strip_prefix(old_parent)
                    .ok_or("Invalid scoped address")?
            )
        };
        mapping.insert(n.id.clone(), new.clone());
        for p in &n.ports {
            if !p.explicit {
                mapping.insert(port_id(&n.id, p), format!("{new}::{}", p.id));
            }
        }
        for part in &n.parts {
            if let NodePart::Member {
                id: None,
                default_id: Some(name),
                ..
            } = part
            {
                mapping.insert(format!("{}::{name}", n.id), format!("{new}::{name}"));
            }
        }
    }
    // Contextual rules stay contextual. Membership intentionally changes which
    // ancestors match; direct object selectors follow their rebased identity.
    let mut references_diagram = diagram.clone();
    references_diagram
        .styling
        .retain(|entry| !matches!(entry, StyleEntry::Rule(rule) if rule.selector.contains(' ')));
    let refs = node_edit::rename::references(source, &references_diagram, &mapping)?;
    let mut references: BTreeMap<_, _> = refs
        .into_iter()
        .map(|e| ((e.range.start, e.range.end), e))
        .collect();
    for relation in &diagram.relations {
        if !line_inside(&index, relation.line, &range) {
            continue;
        }
        let span = relation_edit::endpoint_spans(source, relation.line)?;
        let reversed = matches!(
            &source[span.operator],
            "<-" | "<--" | "<.." | "<|--" | "<|.."
        );
        let ends = if reversed {
            [(&relation.to, span.from), (&relation.from, span.to)]
        } else {
            [(&relation.from, span.from), (&relation.to, span.to)]
        };
        for (canonical, span) in ends {
            let replacement = node_edit::rename::mapped(&mapping, canonical).to_string();
            if source[span.clone()] != replacement {
                references.insert(
                    (span.start, span.end),
                    Edit {
                        range: span,
                        replacement,
                    },
                );
            }
        }
    }
    for attachment in &diagram.attachments {
        if !line_inside(&index, attachment.line, &range) {
            continue;
        }
        let line = index
            .line(attachment.line)
            .ok_or("Missing attachment source")?;
        let text = code(&source[line.clone()]);
        let tokens = relation_edit::tokens(text);
        let first = if attachment.name.is_some() {
            if text.trim_start().starts_with('@') {
                1
            } else {
                2
            }
        } else {
            0
        };
        let span = tokens.get(first).ok_or("Missing attachment endpoint")?;
        let span = line.start + span.start..line.start + span.end;
        let replacement = node_edit::rename::mapped(&mapping, &attachment.source).to_string();
        if source[span.clone()] != replacement {
            references.insert(
                (span.start, span.end),
                Edit {
                    range: span,
                    replacement,
                },
            );
        }
    }
    let mut inside = Vec::new();
    let mut edits = Vec::new();
    let mut changes = vec![format!("Move {id} → {}", parent.unwrap_or("Root"))];
    for edit in references.into_values() {
        changes.push(format!(
            "Reference {} → {}",
            &source[edit.range.clone()],
            edit.replacement
        ));
        if range.start <= edit.range.start && edit.range.end <= range.end {
            inside.push(Edit {
                range: edit.range.start - range.start..edit.range.end - range.start,
                replacement: edit.replacement,
            });
        } else {
            edits.push(edit);
        }
    }
    let content = node_edit::candidate(&source[range.clone()], &inside)?;
    let old_indent: String = source[line]
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .collect();
    let (at, indent) = if let Some(parent) = parent {
        let destination = diagram.nodes.iter().find(|n| n.id == parent).unwrap();
        let block = blocks
            .get(&destination.line)
            .ok_or("Missing destination block")?;
        let line = index
            .line(destination.line)
            .ok_or("Missing destination line")?;
        let indent: String = source[line]
            .chars()
            .take_while(|ch| ch.is_whitespace())
            .collect();
        (closing_start(source, block.end), format!("{indent}    "))
    } else {
        (
            source_patch::insert_section(source, &diagram, "semantics", "")?
                .range
                .start,
            "    ".into(),
        )
    };
    let newline = newline(source);
    let mut moved_content: String = content
        .split_inclusive('\n')
        .map(|line| {
            let line = line.strip_prefix(&old_indent).unwrap_or(line);
            format!("{indent}{line}")
        })
        .collect();
    if !moved_content.ends_with('\n') {
        moved_content.push_str(newline);
    }
    edits.push(Edit {
        range: range.clone(),
        replacement: String::new(),
    });
    edits.push(Edit {
        range: at..at,
        replacement: moved_content,
    });
    let candidate = node_edit::candidate(source, &edits)?;
    let updated = node_edit::diagram(&candidate).map_err(|error| {
        format!("Membership move rejected: {error}. No relations or rules were removed.")
    })?;
    verify(&diagram, &updated, &mapping, id, parent)?;
    let new_id = node_edit::rename::mapped(&mapping, id).to_string();
    for (old, new) in &mapping {
        if old != new {
            changes.push(format!("Address {old} → {new}"));
        }
    }
    changes.push(format!(
        "Preserve {} relations, {} attachments and all layout rules.",
        diagram.relations.len(),
        diagram.attachments.len()
    ));
    Ok(Move {
        id: new_id,
        edits,
        changes,
    })
}
fn line_inside(index: &SourceIndex, line: usize, range: &Range<usize>) -> bool {
    index
        .line(line)
        .is_some_and(|line| range.contains(&line.start))
}
fn verify(
    before: &Diagram,
    after: &Diagram,
    mapping: &BTreeMap<String, String>,
    moved: &str,
    parent: Option<&str>,
) -> Result<(), String> {
    if before.nodes.len() != after.nodes.len()
        || before.relations.len() != after.relations.len()
        || before.attachments.len() != after.attachments.len()
        || before.layout.len() != after.layout.len()
        || before.alignments.len() != after.alignments.len()
        || before.port_layout.len() != after.port_layout.len()
        || before.relation_layout.len() != after.relation_layout.len()
        || before.relation_end_layout.len() != after.relation_end_layout.len()
    {
        return Err("Membership move would lose elements or rules. Source unchanged.".into());
    }
    for node in &before.nodes {
        let id = node_edit::rename::mapped(mapping, &node.id);
        let expected_parent = if node.id == moved {
            parent
        } else {
            node.parent
                .as_deref()
                .map(|p| node_edit::rename::mapped(mapping, p))
        };
        if !after
            .nodes
            .iter()
            .any(|n| n.id == id && n.parent.as_deref() == expected_parent)
        {
            return Err("Unexpected membership/address change. Source unchanged.".into());
        }
    }
    // Reordering declarations changes relation vector order; compare by name or
    // complete normalized content as a multiset, never by endpoints alone.
    let mut unmatched = after.relations.clone();
    for relation in &before.relations {
        let mut expected = relation.clone();
        expected.from = node_edit::rename::mapped(mapping, &relation.from).into();
        expected.to = node_edit::rename::mapped(mapping, &relation.to).into();
        if let Some(at) = unmatched.iter().position(|r| {
            r.name == expected.name
                && r.from == expected.from
                && r.to == expected.to
                && r.kind == expected.kind
                && r.appearance == expected.appearance
                && r.label == expected.label
                && r.source_end == expected.source_end
                && r.target_end == expected.target_end
                && r.classes == expected.classes
        }) {
            unmatched.remove(at);
        } else {
            return Err(
                "Membership move changes reference resolution. No relations or rules were removed."
                    .into(),
            );
        }
    }
    let mut unmatched = after.attachments.clone();
    for a in &before.attachments {
        if let Some(at) = unmatched.iter().position(|b| {
            a.name == b.name
                && a.classes == b.classes
                && a.target == b.target
                && b.source == node_edit::rename::mapped(mapping, &a.source)
        }) {
            unmatched.remove(at);
        } else {
            return Err(
                "Membership move changes an attachment reference. Source unchanged.".into(),
            );
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests;
