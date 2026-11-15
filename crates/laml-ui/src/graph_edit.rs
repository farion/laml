//! Atomic graph gestures composed from the existing source-preserving editors.
use crate::{document::Edit, element_edit, node_edit, port_edit, relation_edit, semantic_edit};
use laml::syntax::EndPosition;

pub const OPERATORS: &[&str] = &[
    "->", "<->", "--", "..", "-->", "..>", "--^>", "--|>", "..|>", "*--", "--*", "o--", "--o",
    "*-->", "o-->",
];

pub fn identifier(id: &str) -> bool {
    id.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Retain all unmodified bytes, including Unicode, comments and CRLF. One gesture
/// becomes one history transaction even when its component edits change offsets.
pub(crate) fn difference(before: &str, after: &str) -> Vec<Edit> {
    if before == after {
        return vec![];
    }
    let start: usize = before
        .chars()
        .zip(after.chars())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum();
    let suffix: usize = before[start..]
        .chars()
        .rev()
        .zip(after[start..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum();
    vec![Edit {
        range: start..before.len() - suffix,
        replacement: after[start..after.len() - suffix].into(),
    }]
}

pub fn add_node(
    source: &str,
    kind: element_edit::ElementKind,
    id: &str,
    label: &str,
) -> Result<Vec<Edit>, String> {
    add_node_in(source, kind, id, label, None)
}

pub fn add_node_in(source: &str, kind: element_edit::ElementKind, id: &str, label: &str, parent: Option<&str>) -> Result<Vec<Edit>, String> {
    if !identifier(id) {
        return Err("ID must be an identifier (letters, digits, underscores; no leading digit).".into());
    }
    let (fresh, edits) = element_edit::add(source, kind, label, parent, None)?;
    let mut candidate = node_edit::candidate(source, &edits)?;
    if fresh != id {
        let rename = node_edit::rename_id(&candidate, &fresh, id)?;
        candidate = node_edit::candidate(&candidate, &rename)?;
    }
    Ok(difference(source, &candidate))
}

pub fn add_port(
    source: &str,
    owner: &str,
    id: &str,
    name: &str,
    side: &str,
) -> Result<Vec<Edit>, String> {
    if !identifier(id) {
        return Err("Port ID must be an identifier.".into());
    }
    let (fresh, edits) = port_edit::add(source, owner, name)?;
    let mut candidate = node_edit::candidate(source, &edits)?;
    if fresh != id {
        let rename = port_edit::rename_id(&candidate, &fresh, id)?;
        candidate = node_edit::candidate(&candidate, &rename)?;
    }
    let sides = port_edit::side(&candidate, &[id.into()], Some(side))?;
    candidate = node_edit::candidate(&candidate, &sides)?;
    Ok(difference(source, &candidate))
}

fn line(source: &str, id: &str) -> Result<usize, String> {
    node_edit::diagram(source)?
        .relations
        .iter()
        .find(|r| r.name.as_deref() == Some(id))
        .map(|r| r.line)
        .ok_or_else(|| "Relation no longer exists".into())
}

pub fn add_relation(
    source: &str,
    from: &str,
    to: &str,
    operator: &str,
    from_side: Option<&str>,
    to_side: Option<&str>,
) -> Result<(String, Vec<Edit>), String> {
    add_relation_labeled(source, from, to, operator, None, from_side, to_side)
}
pub fn add_relation_labeled(source: &str, from: &str, to: &str, operator: &str, label: Option<&str>, from_side: Option<&str>, to_side: Option<&str>) -> Result<(String, Vec<Edit>), String> {
    if crate::notation::Notation::of(source).relation(operator).is_none() {
        return Err("Unsupported relationship type".into());
    }
    let (id, edits) = semantic_edit::add_relation_operator(source, from, to, operator, label)?;
    let mut candidate = node_edit::candidate(source, &edits)?;
    for (end, side) in [
        (EndPosition::Source, from_side),
        (EndPosition::Target, to_side),
    ] {
        let edits = relation_edit::endpoint_side(&candidate, line(&candidate, &id)?, end, side)?;
        candidate = node_edit::candidate(&candidate, &edits)?;
    }
    Ok((id, difference(source, &candidate)))
}

pub fn reconnect(
    source: &str,
    source_line: usize,
    end: EndPosition,
    endpoint: &str,
    side: Option<&str>,
) -> Result<(String, Vec<Edit>), String> {
    let (diagram, relation) = relation_edit::declaration(source, source_line)?;
    let (id, edits) = relation_edit::named(source, &diagram, &relation)?;
    let mut candidate = node_edit::candidate(source, &edits)?;
    let edits = relation_edit::reconnect(&candidate, line(&candidate, &id)?, end, endpoint)?;
    candidate = node_edit::candidate(&candidate, &edits)?;
    let edits = relation_edit::endpoint_side(&candidate, line(&candidate, &id)?, end, side)?;
    candidate = node_edit::candidate(&candidate, &edits)?;
    Ok((id, difference(source, &candidate)))
}

#[cfg(test)]
mod tests;
