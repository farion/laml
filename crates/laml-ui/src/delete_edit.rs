//! Explicit, revision-bound deletion proposals; never mutate while showing a preview.
use crate::{
    document::Edit, node_edit, source::SourceIndex, source_patch::remove_line, style_edit::code,
};
use laml::syntax::{NodePart, StyleEntry};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerMode {
    WithContents,
    KeepContents,
}
#[derive(Clone, Debug)]
pub struct Deletion {
    pub edits: Vec<Edit>,
    pub affected: Vec<String>,
}

pub fn plan(
    source: &str,
    ids: &[String],
    relations: &[usize],
    mode: ContainerMode,
) -> Result<Deletion, String> {
    plan_with_ports(source, ids, relations, &[], mode)
}

pub fn plan_with_ports(
    source: &str,
    ids: &[String],
    relations: &[usize],
    ports: &[String],
    mode: ContainerMode,
) -> Result<Deletion, String> {
    let mut diagram = node_edit::diagram(source)?;
    let index = SourceIndex::new(source);
    let blocks = node_edit::blocks(source, &diagram)?;
    let mut deleted: BTreeSet<_> = ids.iter().cloned().collect();
    if deleted
        .iter()
        .any(|id| !diagram.nodes.iter().any(|node| &node.id == id))
    {
        return Err("Selection no longer exists".into());
    }
    let unwrap = if mode == ContainerMode::KeepContents {
        if ids.len() != 1 {
            return Err("Keep contents requires one selected container".into());
        }
        Some(
            diagram
                .nodes
                .iter()
                .find(|n| n.id == ids[0])
                .unwrap()
                .clone(),
        )
    } else {
        None
    };
    if unwrap.is_none() {
        for node in &diagram.nodes {
            if node
                .parent
                .as_ref()
                .is_some_and(|parent| deleted.contains(parent))
            {
                deleted.insert(node.id.clone());
            }
        }
    }
    let mut endpoints = deleted.clone();
    let mut affected = Vec::new();
    for node in &diagram.nodes {
        if !deleted.contains(&node.id) {
            continue;
        }
        affected.push(format!("Node {} · line {}", node.id, node.line));
        for port in &node.ports {
            endpoints.insert(if port.explicit {
                port.id.clone()
            } else {
                format!("{}::{}", node.id, port.id)
            });
        }
        for part in &node.parts {
            if let NodePart::Member { id, default_id, .. } = part {
                if let Some(id) = id {
                    endpoints.insert(id.clone());
                } else if let Some(id) = default_id {
                    endpoints.insert(format!("{}::{id}", node.id));
                }
            }
        }
    }
    let mut lines: BTreeSet<usize> = BTreeSet::new();
    for id in ports {
        let (_, port) =
            crate::port_edit::find(&diagram, id).ok_or("Port selection no longer exists")?;
        endpoints.insert(id.clone());
        lines.insert(port.line);
        affected.push(format!("Port {id} · line {}", port.line));
    }
    let mut removed_names = BTreeSet::new();
    for relation in &diagram.relations {
        if relations.contains(&relation.line)
            || endpoints.contains(&relation.from)
            || endpoints.contains(&relation.to)
        {
            lines.insert(relation.line);
            if let Some(name) = &relation.name {
                removed_names.insert(name.clone());
            }
            affected.push(format!(
                "Relation {} → {} · line {}",
                relation.from, relation.to, relation.line
            ));
        }
    }
    if relations.iter().any(|line| {
        !diagram.relations.iter().any(|r| r.line == *line)
            && !diagram.attachments.iter().any(|a| a.line == *line)
    }) {
        return Err("Relation selection no longer exists".into());
    }
    for attachment in &diagram.attachments {
        if relations.contains(&attachment.line)
            || endpoints.contains(&attachment.source)
            || removed_names.contains(&attachment.target)
        {
            lines.insert(attachment.line);
            if let Some(name) = &attachment.name {
                removed_names.insert(name.clone());
            }
            affected.push(format!("Attachment · line {}", attachment.line));
        }
    }
    for rule in &diagram.layout {
        if endpoints.contains(&rule.first) || endpoints.contains(&rule.second) {
            lines.insert(rule.line);
        }
    }
    for rule in &diagram.alignments {
        if endpoints.contains(&rule.first) || endpoints.contains(&rule.second) {
            lines.insert(rule.line);
        }
    }
    for rule in &diagram.scoped_alignments {
        if endpoints.contains(&rule.first) || endpoints.contains(&rule.second) {
            lines.insert(rule.line);
        }
    }
    for rule in &diagram.relation_layout {
        if removed_names.contains(&rule.name) {
            lines.insert(rule.line);
        }
    }
    for rule in &diagram.relation_end_layout {
        if removed_names.contains(&rule.name) {
            lines.insert(rule.line);
        }
    }
    for rule in &diagram.port_layout {
        if endpoints.contains(&rule.id) {
            lines.insert(rule.line);
        }
    }
    let mut edits = Vec::new();
    let mut ranges = Vec::new();
    for node in &diagram.nodes {
        if deleted.contains(&node.id) {
            if unwrap.is_some() {
                let block = blocks
                    .get(&node.line)
                    .ok_or("Selected node is not a container")?;
                if !diagram
                    .nodes
                    .iter()
                    .any(|child| child.parent.as_deref() == Some(&node.id))
                {
                    return Err("Keep contents requires a container with nested nodes".into());
                }
                lines.insert(node.line);
                let closing = source[..block.end].trim_end_matches(['\r', '\n']);
                let closing_start = closing.rfind('\n').map_or(0, |at| at + 1);
                let closing_line = source[..closing_start]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count()
                    + 1;
                lines.insert(closing_line);
            } else if let Some(block) = blocks.get(&node.line) {
                ranges.push(block.clone());
            } else {
                lines.insert(node.line);
            }
        }
    }
    // Remove direct local rules for removed objects; never rewrite a property value.
    for entry in &diagram.styling {
        let StyleEntry::Rule(rule) = entry else {
            continue;
        };
        let targets_deleted = rule.selector.split_whitespace().take(1).any(|selector| {
            ["node[", "port[", "member[", "relation["]
                .iter()
                .any(|prefix| {
                    selector
                        .strip_prefix(prefix)
                        .and_then(|s| s.strip_suffix(']'))
                        .is_some_and(|id| match *prefix {
                            "node[" => deleted.contains(id),
                            "relation[" => removed_names.contains(id),
                            _ => endpoints.contains(id) && !deleted.contains(id),
                        })
                })
        });
        if targets_deleted {
            if unwrap.is_some() && rule.selector.contains(' ') {
                return Err("This container has contextual styles. Move those styles to its contents before removing the container.".into());
            }
            let opening = index.line(rule.line).ok_or("Missing style rule")?.start;
            let mut end = None;
            for (i, text) in source.lines().enumerate().skip(rule.line) {
                if code(text).trim() == "}" {
                    end = Some(index.line(i + 2).map_or(source.len(), |r| r.start));
                    break;
                }
            }
            ranges.push(opening..end.ok_or("Missing style rule boundary")?);
            affected.push(format!(
                "Local style {} · line {}",
                rule.selector, rule.line
            ));
        }
    }
    for line in &lines {
        let range = index.line(*line).ok_or("Missing affected line")?;
        if !ranges.iter().any(|block| block.contains(&range.start)) {
            edits.push(remove_line(source, *line)?);
            affected.push(format!("Remove source reference · line {line}"));
        }
    }
    ranges.sort_by_key(|r| (r.start, std::cmp::Reverse(r.end)));
    let mut end = 0;
    for range in ranges {
        if range.start < end {
            continue;
        }
        end = range.end;
        edits.push(Edit {
            range,
            replacement: String::new(),
        });
    }
    if let Some(container) = unwrap {
        // Rebase default child addresses; explicit global child IDs remain unchanged.
        let mut mapping = BTreeMap::new();
        for node in &diagram.nodes {
            if deleted.contains(&node.id) {
                continue;
            }
            let parent = if node.parent.as_deref() == Some(&container.id) {
                container.parent.clone()
            } else {
                node.parent
                    .as_ref()
                    .map(|p| mapping.get(p).cloned().unwrap_or_else(|| p.clone()))
            };
            if !node.explicit
                && let Some(old_parent) = &node.parent
            {
                let local = node
                    .id
                    .strip_prefix(&format!("{old_parent}."))
                    .unwrap_or(&node.id);
                let new = parent.map_or_else(|| local.to_string(), |p| format!("{p}.{local}"));
                if new != node.id {
                    mapping.insert(node.id.clone(), new.clone());
                    for port in &node.ports {
                        if !port.explicit {
                            mapping.insert(
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
                            mapping
                                .insert(format!("{}::{name}", node.id), format!("{new}::{name}"));
                        }
                    }
                }
            }
        }
        diagram.relations.retain(|r| !lines.contains(&r.line));
        diagram.attachments.retain(|r| !lines.contains(&r.line));
        diagram.layout.retain(|r| !lines.contains(&r.line));
        diagram.alignments.retain(|r| !lines.contains(&r.line));
        diagram
            .scoped_alignments
            .retain(|r| !lines.contains(&r.line));
        diagram.port_layout.retain(|r| !lines.contains(&r.line));
        diagram.styling.retain(|entry| match entry {
            StyleEntry::Rule(rule) => !edits.iter().any(|e| {
                index
                    .line(rule.line)
                    .is_some_and(|r| e.range.contains(&r.start))
            }),
            _ => true,
        });
        edits.extend(node_edit::rename::references(source, &diagram, &mapping)?);
    }
    let candidate = node_edit::candidate(source, &edits)?;
    node_edit::diagram(&candidate)?;
    affected.sort();
    affected.dedup();
    Ok(Deletion { edits, affected })
}

#[cfg(test)]
mod tests;
