//! Structured declarations and body editing without a second semantic model.
use crate::{
    document::Edit,
    element_edit::{closing_start, newline},
    node_edit,
    source::SourceIndex,
    style_edit::code,
};
use laml::syntax::{Diagram, NodeKind};
mod members;
pub use members::*;
#[derive(Clone)]
pub struct Content {
    pub header: String,
    pub body: Option<String>,
}
pub fn content(source: &str, id: &str) -> Result<Content, String> {
    let d = node_edit::diagram(source)?;
    let node = d.nodes.iter().find(|n| n.id == id).ok_or("Missing node")?;
    let index = SourceIndex::new(source);
    let range = index.line(node.line).ok_or("Missing declaration")?;
    let header = code(&source[range]).trim().to_string();
    let body = if matches!(
        node.kind,
        NodeKind::Note | NodeKind::Package | NodeKind::Junction
    ) {
        None
    } else if let Some(block) = node_edit::blocks(source, &d)?.get(&node.line) {
        let start = index.line(node.line + 1).unwrap().start;
        Some(source[start..closing_start(source, block.end)].into())
    } else {
        Some(String::new())
    };
    Ok(Content { header, body })
}
pub(crate) fn stable_topology(
    before: &Diagram,
    after: &Diagram,
    owner: &str,
) -> Result<(), String> {
    let nodes = |d: &Diagram| {
        let mut ids: Vec<_> = d
            .nodes
            .iter()
            .map(|n| (n.id.clone(), n.parent.clone()))
            .collect();
        ids.sort();
        ids
    };
    let edges = |d: &Diagram| {
        d.relations
            .iter()
            .map(|r| {
                let mut r = r.clone();
                r.line = 0;
                r
            })
            .collect::<Vec<_>>()
    };
    macro_rules! same_rules {
        ($field:ident) => {{
            let mut a = before.$field.clone();
            let mut b = after.$field.clone();
            for r in &mut a {
                r.line = 0;
            }
            for r in &mut b {
                r.line = 0;
            }
            a == b
        }};
    }
    let styles = |d: &Diagram| {
        let mut entries = d.styling.clone();
        for e in &mut entries {
            match e {
                laml::syntax::StyleEntry::Import { line, .. } => *line = 0,
                laml::syntax::StyleEntry::Rule(r) => {
                    r.line = 0;
                    for p in &mut r.properties {
                        p.line = 0;
                    }
                }
            }
        }
        entries
    };
    let others = |d: &Diagram| {
        d.nodes
            .iter()
            .filter(|n| n.id != owner)
            .map(|n| {
                let mut n = n.clone();
                n.line = 0;
                for p in &mut n.ports {
                    p.line = 0;
                }
                for p in &mut n.parts {
                    if let laml::syntax::NodePart::Member { line, .. } = p {
                        *line = 0;
                    }
                }
                n
            })
            .collect::<Vec<_>>()
    };
    if before.settings != after.settings
        || styles(before) != styles(after)
        || others(before) != others(after)
        || !same_rules!(layout)
        || !same_rules!(alignments)
        || !same_rules!(scoped_alignments)
        || !same_rules!(attachments)
        || !same_rules!(relation_layout)
        || !same_rules!(relation_end_layout)
        || !same_rules!(port_layout)
    {
        return Err(
            "A body/declaration edit cannot change other elements or global sections".into(),
        );
    }
    if nodes(before) != nodes(after) || edges(before) != edges(after) {
        return Err("Declaration/body editing cannot silently rename or capture endpoint addresses. Assign stable member IDs or use the dedicated ID/membership tools first.".into());
    }
    Ok(())
}
pub fn header(source: &str, id: &str, value: &str) -> Result<Vec<Edit>, String> {
    if value.contains(['\r', '\n']) {
        return Err("Declaration heading must be a single line".into());
    }
    let before = node_edit::diagram(source)?;
    let n = before
        .nodes
        .iter()
        .find(|n| n.id == id)
        .ok_or("Missing node")?;
    let range = SourceIndex::new(source).line(n.line).unwrap();
    let raw = &source[range.clone()];
    let text = code(raw);
    let indent = text.len() - text.trim_start().len();
    let edits = vec![Edit {
        range: range.start + indent..range.start + text.trim_end().len(),
        replacement: value.trim().into(),
    }];
    stable_topology(
        &before,
        &node_edit::diagram(&node_edit::candidate(source, &edits)?)?,
        id,
    )?;
    Ok(edits)
}
pub fn body(source: &str, id: &str, value: &str) -> Result<Vec<Edit>, String> {
    let d = node_edit::diagram(source)?;
    let n = d.nodes.iter().find(|n| n.id == id).ok_or("Missing node")?;
    if content(source, id)?.body.is_none() {
        return Err("This element uses its dedicated note/container controls".into());
    }
    let eol = newline(source);
    let index = SourceIndex::new(source);
    let range = index.line(n.line).unwrap();
    let raw = &source[range.clone()];
    let text = code(raw);
    let end = text.trim_end().len();
    let indent = &raw[..raw.len() - raw.trim_start().len()];
    let value = value.replace("\r\n", "\n").replace('\r', "\n");
    let value = value.trim_end_matches('\n');
    let content = if value.is_empty() {
        String::new()
    } else {
        format!("{}{eol}", value.replace('\n', eol))
    };
    let edits = if let Some(block) = node_edit::blocks(source, &d)?.get(&n.line) {
        vec![Edit {
            range: index.line(n.line + 1).unwrap().start..closing_start(source, block.end),
            replacement: content,
        }]
    } else {
        vec![Edit {
            range: range.start + end..range.end,
            replacement: format!(" {{{}{eol}{content}{indent}}}", &raw[end..]),
        }]
    };
    let after = node_edit::diagram(&node_edit::candidate(source, &edits)?)?;
    stable_topology(&d, &after, id)?;
    Ok(edits)
}
pub fn endpoints(diagram: &Diagram) -> Vec<String> {
    let mut ids = vec![];
    for n in &diagram.nodes {
        ids.push(n.id.clone());
        for p in &n.ports {
            ids.push(crate::properties::port_id(&n.id, p));
        }
        for part in &n.parts {
            if let Some(id) = member_address(&n.id, part) {
                ids.push(id);
            }
        }
    }
    ids.sort();
    ids.dedup();
    ids
}
pub fn attach(source: &str, note: &str, line: usize) -> Result<Vec<Edit>, String> {
    let d = node_edit::diagram(source)?;
    if !d
        .nodes
        .iter()
        .any(|n| n.id == note && n.kind == NodeKind::Note)
    {
        return Err("Select a semantic note".into());
    }
    let r = d
        .relations
        .iter()
        .find(|r| r.line == line)
        .ok_or("Missing relation")?;
    let (name, mut edits) = crate::relation_edit::named(source, &d, r)?;
    edits.push(crate::source_patch::insert_section(
        source,
        &d,
        "semantics",
        &format!("{note} .. relation[{name}]"),
    )?);
    let candidate = node_edit::candidate(source, &edits)?;
    node_edit::diagram(&candidate)?;
    Ok(edits)
}
pub fn remove_attachment(source: &str, line: usize) -> Result<Vec<Edit>, String> {
    let d = node_edit::diagram(source)?;
    if !d.attachments.iter().any(|a| a.line == line) {
        return Err("Missing attachment".into());
    }
    Ok(vec![crate::source_patch::remove_line(source, line)?])
}

#[cfg(test)]
mod tests;
