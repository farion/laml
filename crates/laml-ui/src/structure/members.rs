use super::*;
use laml::syntax::NodePart;
use std::collections::BTreeMap;
pub const MEMBER_PROPERTIES: [&str; 5] = [
    "color",
    "font-size",
    "display",
    "symbol-rendering",
    "icon-set",
];
pub fn member_address(owner: &str, part: &NodePart) -> Option<String> {
    match part {
        NodePart::Member { id, default_id, .. } => id
            .clone()
            .or_else(|| default_id.as_ref().map(|name| format!("{owner}::{name}"))),
        _ => None,
    }
}
pub fn assign_member_id(source: &str, owner: &str, line: usize) -> Result<Vec<Edit>, String> {
    let diagram = node_edit::diagram(source)?;
    let n = diagram
        .nodes
        .iter()
        .find(|n| n.id == owner)
        .ok_or("Missing owner")?;
    let part = n
        .parts
        .iter()
        .find(|p| matches!(p,NodePart::Member{line:l,..} if *l==line))
        .ok_or("Missing member")?;
    if matches!(part, NodePart::Member { id: Some(_), .. }) {
        return Ok(vec![]);
    }
    let id = crate::semantic_edit::fresh_id(&diagram, "Member");
    let old = member_address(owner, part);
    let mapping = old
        .as_ref()
        .map(|old| BTreeMap::from([(old.clone(), id.clone())]))
        .unwrap_or_default();
    let mut edits = node_edit::rename::references(source, &diagram, &mapping)?;
    let range = SourceIndex::new(source).line(line).unwrap();
    let text = &source[range.clone()];
    let start = range.start + text.len() - text.trim_start().len();
    edits.push(Edit {
        range: start..start,
        replacement: format!("@{id} "),
    });
    let after = node_edit::diagram(&node_edit::candidate(source, &edits)?)?;
    for (a, b) in diagram.relations.iter().zip(&after.relations) {
        if b.from != node_edit::rename::mapped(&mapping, &a.from)
            || b.to != node_edit::rename::mapped(&mapping, &a.to)
        {
            return Err("Member address would capture an unrelated reference".into());
        }
    }
    Ok(edits)
}
pub fn member_value(
    preview: &crate::preview::Preview,
    owner: &str,
    address: &str,
    name: &str,
) -> Option<String> {
    let n = preview.diagram.nodes.iter().find(|n| n.id == owner)?;
    let part = n
        .parts
        .iter()
        .find(|p| member_address(owner, p).as_deref() == Some(address))?;
    let NodePart::Member { classes, kind, .. } = part else {
        return None;
    };
    let sheet = preview
        .styles
        .for_node(&preview.diagram, owner)
        .with_node_ancestor(owner, &n.classes);
    sheet
        .get(
            &laml::styles::Element::Member {
                id: Some(address),
                classes,
                kind: *kind,
            },
            name,
        )
        .map(|d| crate::properties::string(&d.value))
        .or_else(|| match name {
            "font-size" | "color" => preview
                .node_properties
                .get(owner)
                .and_then(|p| p.get(name))
                .and_then(|p| p.value.clone()),
            "display" => Some("block".into()),
            "symbol-rendering" => Some("text".into()),
            _ => None,
        })
}
pub fn member_style(
    source: &str,
    owner: &str,
    address: &str,
    name: &str,
    value: Option<&str>,
) -> Result<Vec<Edit>, String> {
    if !MEMBER_PROPERTIES.contains(&name) {
        return Err("Property is not applicable to members".into());
    }
    let d = node_edit::diagram(source)?;
    if !d.nodes.iter().find(|n| n.id == owner).is_some_and(|n| {
        n.parts
            .iter()
            .any(|p| member_address(owner, p).as_deref() == Some(address))
    }) {
        return Err("Missing member address".into());
    }
    let selectors = [format!("member[{address}]")];
    let edits = crate::style_edit::local_property(source, &selectors, name, value)?;
    if name != "symbol-rendering" || value != Some("icons") {
        return Ok(edits);
    }
    let mut candidate = node_edit::candidate(source, &edits)?;
    let edits =
        crate::style_edit::local_property(&candidate, &selectors, "icon-set", Some("phosphor"))?;
    candidate = node_edit::candidate(&candidate, &edits)?;
    Ok(crate::graph_edit::difference(source, &candidate))
}
pub fn compartment_value(preview: &crate::preview::Preview, owner: &str, kind: &str) -> String {
    let Some(n) = preview.diagram.nodes.iter().find(|n| n.id == owner) else {
        return "block".into();
    };
    preview
        .styles
        .for_node(&preview.diagram, owner)
        .with_node_ancestor(owner, &n.classes)
        .get(&laml::styles::Element::Compartment { kind }, "display")
        .map(|d| crate::properties::string(&d.value))
        .unwrap_or_else(|| "block".into())
}
pub fn compartment(
    source: &str,
    owner: &str,
    kind: &str,
    value: Option<&str>,
) -> Result<Vec<Edit>, String> {
    if !["attributes", "operations", "literals"].contains(&kind)
        || value.is_some_and(|v| !["block", "none"].contains(&v))
    {
        return Err("Invalid compartment visibility".into());
    }
    crate::style_edit::local_property(
        source,
        &[format!("node[{owner}] compartment[{kind}]")],
        "display",
        value,
    )
}
