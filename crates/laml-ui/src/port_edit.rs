//! GNE boundary ports: explicit addresses keep names independent of references.
use crate::{
    document::Edit,
    element_edit::{closing_start, newline},
    node_edit,
    properties::port_id,
    relation_edit::tokens,
    source::SourceIndex,
    source_patch, style_edit,
};
use laml::syntax::{Diagram, NodeDecl, NodeKind, PortDecl};
use std::collections::BTreeMap;

fn identifier(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

pub fn find<'a>(diagram: &'a Diagram, id: &str) -> Option<(&'a NodeDecl, &'a PortDecl)> {
    diagram.nodes.iter().find_map(|node| {
        node.ports
            .iter()
            .find(|port| port_id(&node.id, port) == id)
            .map(|p| (node, p))
    })
}
pub fn add(source: &str, owner: &str, name: &str) -> Result<(String, Vec<Edit>), String> {
    let diagram = node_edit::diagram(source)?;
    let node = diagram
        .nodes
        .iter()
        .find(|n| n.id == owner)
        .ok_or("Missing port owner")?;
    if !matches!(node.kind, NodeKind::Generic | NodeKind::Class) {
        return Err("Ports require a generic node or class".into());
    }
    let id = crate::semantic_edit::fresh_id(&diagram, "Port");
    let name = if name.trim().is_empty() {
        &id
    } else {
        name.trim()
    };
    if !identifier(name) {
        return Err("Port names are identifiers: use letters, digits and underscores, starting with a letter or underscore.".into());
    }
    let declaration = format!("@{id} port {name}");
    let index = SourceIndex::new(source);
    let range = index.line(node.line).ok_or("Missing owner line")?;
    let line = &source[range.clone()];
    let indent: String = line.chars().take_while(|ch| ch.is_whitespace()).collect();
    let newline = newline(source);
    let blocks = node_edit::blocks(source, &diagram)?;
    let mut edits = Vec::new();
    if let Some(block) = blocks.get(&node.line) {
        let at = closing_start(source, block.end);
        edits.push(Edit {
            range: at..at,
            replacement: format!("{indent}    {declaration}{newline}"),
        });
    } else {
        let code = style_edit::code(line);
        let at = range.start + code.trim_end().len();
        edits.push(Edit {
            range: at..at,
            replacement: " {".into(),
        });
        let at = index
            .line(node.line + 1)
            .ok_or("Missing line after owner")?
            .start;
        edits.push(Edit {
            range: at..at,
            replacement: format!("{indent}    {declaration}{newline}{indent}}}{newline}"),
        });
    }
    let candidate = node_edit::candidate(source, &edits)?;
    let updated = node_edit::diagram(&candidate)?;
    if diagram.relations.len() != updated.relations.len()
        || diagram
            .relations
            .iter()
            .zip(&updated.relations)
            .any(|(before, after)| before.from != after.from || before.to != after.to)
    {
        return Err(
            "Created port would capture an existing scoped reference. Source unchanged.".into(),
        );
    }
    Ok((id, edits))
}

pub fn property(
    source: &str,
    ids: &[String],
    name: &str,
    value: Option<&str>,
) -> Result<Vec<Edit>, String> {
    if !crate::properties::PORT_PROPERTIES.contains(&name) {
        return Err("Unsupported port property".into());
    }
    let diagram = node_edit::diagram(source)?;
    for id in ids {
        if find(&diagram, id).is_none() {
            return Err(format!("Unknown port {id}"));
        }
    }
    style_edit::local_property(
        source,
        &ids.iter()
            .map(|id| format!("port[{id}]"))
            .collect::<Vec<_>>(),
        name,
        value,
    )
}
pub fn side(source: &str, ids: &[String], side: Option<&str>) -> Result<Vec<Edit>, String> {
    if side.is_some_and(|side| !["left", "right", "top", "bottom"].contains(&side)) {
        return Err("Invalid port side".into());
    }
    let diagram = node_edit::diagram(source)?;
    let mut edits = Vec::new();
    let mut declarations = Vec::new();
    for id in ids.iter().collect::<std::collections::BTreeSet<_>>() {
        if find(&diagram, id).is_none() {
            return Err("Missing selected port".into());
        }
        let rule = diagram.port_layout.iter().find(|r| &r.id == id);
        match (rule, side) {
            (Some(rule), None) => edits.push(source_patch::remove_line(source, rule.line)?),
            (Some(rule), Some(side)) => {
                let range = SourceIndex::new(source)
                    .line(rule.line)
                    .ok_or("Missing port rule")?;
                let text = style_edit::code(&source[range.clone()]);
                let equal = text.find('=').ok_or("Missing side value")?;
                let tail = &text[equal + 1..];
                let start = range.start + equal + 1 + tail.len() - tail.trim_start().len();
                edits.push(Edit {
                    range: start..range.start + text.trim_end().len(),
                    replacement: side.into(),
                });
            }
            (None, Some(side)) => declarations.push(format!("port[{id}] : side={side}")),
            (None, None) => {}
        }
    }
    if !declarations.is_empty() {
        edits.push(source_patch::insert_section(
            source,
            &diagram,
            "layout",
            &declarations.join(&format!("{}    ", newline(source))),
        )?);
    }
    Ok(edits)
}

pub fn label(source: &str, id: &str, value: &str) -> Result<Vec<Edit>, String> {
    if !identifier(value) {
        return Err("A port name must be one identifier, not free text.".into());
    }
    let diagram = node_edit::diagram(source)?;
    let (_, port) = find(&diagram, id).ok_or("Missing port")?;
    if !port.explicit {
        return Err("Assign an explicit global ID before changing a default port name; its default ID is derived from its name.".into());
    }
    let range = SourceIndex::new(source)
        .line(port.line)
        .ok_or("Missing port declaration")?;
    let text = style_edit::code(&source[range.clone()]);
    let parts = tokens(text);
    let word = parts
        .iter()
        .position(|p| &text[(*p).clone()] == "port")
        .ok_or("Missing port keyword")?;
    let name = parts.get(word + 1).ok_or("Missing port name")?;
    let token = &text[name.clone()];
    let end = token.find('(').unwrap_or(token.len());
    let edits = vec![Edit {
        range: range.start + name.start..range.start + name.start + end,
        replacement: value.into(),
    }];
    let candidate = node_edit::candidate(source, &edits)?;
    let updated = node_edit::diagram(&candidate)?;
    if find(&updated, id).is_none() {
        return Err("Port label edit changed its identity".into());
    }
    Ok(edits)
}

pub fn rename_id(source: &str, id: &str, value: &str) -> Result<Vec<Edit>, String> {
    if !identifier(value) {
        return Err("Use a valid global identifier.".into());
    }
    let diagram = node_edit::diagram(source)?;
    let (_, port) = find(&diagram, id).ok_or("Missing port")?;
    if value == id {
        return Ok(vec![]);
    }
    let range = SourceIndex::new(source)
        .line(port.line)
        .ok_or("Missing port declaration")?;
    let text = style_edit::code(&source[range.clone()]);
    let first = tokens(text)
        .into_iter()
        .next()
        .ok_or("Missing port declaration")?;
    let range = if port.explicit {
        range.start + first.start
            ..range.start + first.start + text[first.clone()].find('(').unwrap_or(first.len())
    } else {
        range.start + first.start..range.start + first.start
    };
    let replacement = if port.explicit {
        format!("@{value}")
    } else {
        format!("@{value} ")
    };
    let mapping = BTreeMap::from([(id.into(), value.into())]);
    let mut edits = node_edit::rename::references(source, &diagram, &mapping)?;
    edits.push(Edit { range, replacement });
    let candidate = node_edit::candidate(source, &edits)?;
    let updated = node_edit::diagram(&candidate)?;
    if find(&updated, value).is_none() {
        return Err("Invalid explicit port ID".into());
    }
    for (before, after) in diagram.relations.iter().zip(&updated.relations) {
        if after.from != node_edit::rename::mapped(&mapping, &before.from)
            || after.to != node_edit::rename::mapped(&mapping, &before.to)
        {
            return Err("New port ID shadows an unrelated endpoint".into());
        }
    }
    Ok(edits)
}
#[cfg(test)]
mod tests;

/// Edit a port declaration (including both decorator/inline class lists), not its scope.
pub fn definition(source: &str, id: &str, value: &str) -> Result<Vec<Edit>, String> {
    if value.contains(['\r', '\n', '{', '}']) {
        return Err("Use one port declaration without braces".into());
    }
    let before = node_edit::diagram(source)?;
    let (owner, port) = find(&before, id).ok_or("Missing port")?;
    let span = SourceIndex::new(source)
        .line(port.line)
        .ok_or("Missing port line")?;
    let text = style_edit::code(&source[span.clone()]);
    let start = span.start + text.len() - text.trim_start().len();
    let edits = vec![Edit {
        range: start..span.start + text.trim_end().len(),
        replacement: value.trim().into(),
    }];
    let after = node_edit::diagram(&node_edit::candidate(source, &edits)?)?;
    if find(&after, id).is_none() {
        return Err("Keep the port ID; use the rename action for identity changes".into());
    }
    crate::structure::stable_topology(&before, &after, &owner.id)?;
    Ok(edits)
}

pub fn definition_text(source: &str, id: &str) -> Result<String, String> {
    let d = node_edit::diagram(source)?;
    let (_, p) = find(&d, id).ok_or("Missing port")?;
    let r = SourceIndex::new(source)
        .line(p.line)
        .ok_or("Missing port line")?;
    Ok(style_edit::code(&source[r]).trim().to_owned())
}
