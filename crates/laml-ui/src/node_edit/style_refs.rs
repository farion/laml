//! Rewrite selector identities, not labels, classes or similarly spelled IDs.
use super::*;
use laml::syntax::{NodePart, StyleEntry};
use std::collections::BTreeSet;

fn ancestor_matches(diagram: &Diagram, node: &NodeDecl, owner: &str) -> bool {
    let mut parent = node.parent.as_deref();
    while let Some(id) = parent {
        let Some(node) = diagram.nodes.iter().find(|n| n.id == id) else {
            break;
        };
        if owner
            .strip_prefix("node[")
            .and_then(|s| s.strip_suffix(']'))
            .is_some_and(|wanted| wanted == id)
            || owner
                .strip_prefix("node.")
                .is_some_and(|class| node.classes.iter().any(|c| c == class))
        {
            return true;
        }
        parent = node.parent.as_deref();
    }
    false
}

pub(super) fn references(
    source: &str,
    diagram: &Diagram,
    ids: &BTreeMap<String, String>,
) -> Result<Vec<Edit>, String> {
    let index = SourceIndex::new(source);
    let mut edits = Vec::new();
    for entry in &diagram.styling {
        let StyleEntry::Rule(rule) = entry else {
            continue;
        };
        let range = index.line(rule.line).ok_or("Missing style selector")?;
        let text = code(&source[range.clone()]);
        let selector_start = text.find(&rule.selector).ok_or("Missing selector tokens")?;
        for (offset, ch) in rule.selector.char_indices() {
            if ch != '[' {
                continue;
            }
            let Some(end) = rule.selector[offset..].find(']') else {
                continue;
            };
            let bracket = offset + 1..offset + end;
            let raw = &rule.selector[bracket.clone()];
            let prefix = &rule.selector[..offset];
            let kind = prefix.rsplit(' ').next().unwrap_or(prefix);
            let replacement = if kind == "node"
                && let Some((owner, _)) = rule.selector.split_once(' ')
                && prefix.contains(' ')
            {
                let owner_id = owner
                    .strip_prefix("node[")
                    .and_then(|s| s.strip_suffix(']'));
                let matching: Vec<_> = diagram
                    .nodes
                    .iter()
                    .filter(|node| {
                        ancestor_matches(diagram, node, owner)
                            && (node.id == raw
                                || owner_id.is_some_and(|owner| {
                                    node.id
                                        .strip_prefix(&format!("{owner}."))
                                        .is_some_and(|relative| relative == raw)
                                })
                                || owner.starts_with("node.")
                                    && ancestor_matches(diagram, node, owner)
                                    && node.id.rsplit('.').next() == Some(raw))
                    })
                    .collect();
                let values: BTreeSet<_> = matching
                    .iter()
                    .map(|node| {
                        let new = super::rename::mapped(ids, &node.id);
                        if let Some(owner) = owner_id {
                            let parent = super::rename::mapped(ids, owner);
                            new.strip_prefix(&format!("{parent}."))
                                .unwrap_or(new)
                                .to_string()
                        } else {
                            new.rsplit('.').next().unwrap_or(new).to_string()
                        }
                    })
                    .collect();
                if values.len() > 1 {
                    return Err("Rename would change a shared relative selector's targets. Use explicit node selectors first.".into());
                }
                let replacement = values.into_iter().next().filter(|new| new != raw);
                if let Some(new_selector) = &replacement {
                    for node in diagram
                        .nodes
                        .iter()
                        .filter(|node| ancestor_matches(diagram, node, owner))
                    {
                        let new_id = super::rename::mapped(ids, &node.id);
                        let matches_after = if let Some(owner) = owner_id {
                            let new_owner = super::rename::mapped(ids, owner);
                            new_id == new_selector
                                || new_id
                                    .strip_prefix(&format!("{new_owner}."))
                                    .is_some_and(|relative| relative == new_selector)
                        } else {
                            new_id == new_selector
                                || new_id.rsplit('.').next() == Some(new_selector)
                        };
                        if matches_after != matching.iter().any(|before| before.id == node.id) {
                            return Err("New ID would make a relative selector target unrelated nodes. Choose another ID or use explicit selectors.".into());
                        }
                    }
                }
                replacement
            } else {
                let exists = match kind {
                    "node" => diagram.nodes.iter().any(|node| node.id == raw),
                    "port" => diagram
                        .nodes
                        .iter()
                        .any(|node| node.ports.iter().any(|port| node.matches_port(port, raw))),
                    "member" => diagram.nodes.iter().any(|node| {
                        node.parts.iter().any(|part| {
                            matches!(part, NodePart::Member { .. })
                                && node.matches_member(part, raw)
                        })
                    }),
                    _ => false,
                };
                exists
                    .then(|| ids.get(raw).cloned())
                    .flatten()
                    .filter(|new| new != raw)
            };
            if let Some(new) = replacement {
                edits.push(Edit {
                    range: range.start + selector_start + bracket.start
                        ..range.start + selector_start + bracket.end,
                    replacement: new,
                });
            }
        }
    }
    Ok(edits)
}
