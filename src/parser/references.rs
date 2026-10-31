//! Resolve globally named and scoped graph references before layout.
use super::{Diagram, ParseError, error, uml};
use crate::syntax::DiagramType;

pub(super) fn validate_references(diagram: &Diagram) -> Result<(), ParseError> {
    for (index, node) in diagram.nodes.iter().enumerate() {
        if diagram.nodes[..index]
            .iter()
            .any(|other| other.id == node.id)
        {
            return Err(error(node.line, format!("duplicate node '{}'", node.id)));
        }
        if diagram.settings.strict_case && !node.id.starts_with(|ch: char| ch.is_ascii_uppercase())
        {
            return Err(error(node.line, "node identifier must start uppercase"));
        }
    }
    let mut assigned = std::collections::HashSet::new();
    for node in &diagram.nodes {
        assigned.insert(node.id.as_str());
    }
    for node in &diagram.nodes {
        for part in &node.parts {
            if let crate::syntax::NodePart::Member {
                id: Some(id), line, ..
            } = part
                && !assigned.insert(id.as_str())
            {
                return Err(error(*line, format!("duplicate global ID '{id}'")));
            }
        }
    }
    for relation in &diagram.relations {
        if let Some(id) = relation.name.as_deref()
            && !assigned.insert(id)
        {
            return Err(error(relation.line, format!("duplicate global ID '{id}'")));
        }
    }
    for (index, relation) in diagram.relations.iter().enumerate() {
        if let Some(name) = &relation.name {
            if diagram.relations[..index]
                .iter()
                .any(|other| other.name.as_ref() == Some(name))
            {
                return Err(error(relation.line, format!("duplicate relation '{name}'")));
            }
            if diagram.settings.strict_case && !name.starts_with(|ch: char| ch.is_ascii_lowercase())
            {
                return Err(error(relation.line, "relation name must start lowercase"));
            }
        }
        if diagram.settings.diagram_type != DiagramType::Generic
            && relation.appearance.dashed
            && relation.appearance.start == crate::syntax::EndSymbol::None
            && relation.appearance.end == crate::syntax::EndSymbol::None
            && ![&relation.from, &relation.to].iter().any(|id| {
                diagram
                    .nodes
                    .iter()
                    .any(|node| node.id == **id && node.kind == crate::syntax::NodeKind::Note)
            })
        {
            return Err(error(
                relation.line,
                "dotted UML annotation requires a note endpoint",
            ));
        }
        for endpoint in [&relation.from, &relation.to] {
            let targets: usize = diagram
                .nodes
                .iter()
                .map(|node| {
                    usize::from(node.id == *endpoint)
                        + node
                            .parts
                            .iter()
                            .filter(|part| match part {
                                crate::syntax::NodePart::Member { id: Some(id), .. } => {
                                    id == endpoint
                                }
                                crate::syntax::NodePart::Member {
                                    id: None,
                                    default_id: Some(name),
                                    ..
                                } => {
                                    endpoint.strip_prefix(&format!("{}::", node.id))
                                        == Some(name.as_str())
                                }
                                _ => false,
                            })
                            .count()
                })
                .sum();
            if targets == 0 {
                return Err(error(
                    relation.line,
                    format!("unknown node or member '{endpoint}'"),
                ));
            }
            if targets > 1 {
                return Err(error(
                    relation.line,
                    format!("ambiguous member reference '{endpoint}'"),
                ));
            }
        }
        uml::validate_relation(diagram, relation)?;
    }
    for rule in &diagram.layout {
        for id in [&rule.first, &rule.second] {
            if !diagram.nodes.iter().any(|node| &node.id == id) {
                return Err(error(rule.line, format!("unknown node '{id}'")));
            }
        }
    }
    for (index, rule) in diagram.relation_end_layout.iter().enumerate() {
        let target = diagram
            .relations
            .iter()
            .find(|relation| relation.name.as_deref() == Some(rule.name.as_str()))
            .ok_or_else(|| error(rule.line, format!("unknown named relation '{}'", rule.name)))?;
        let end = match rule.end {
            crate::syntax::EndPosition::Source => &target.source_end,
            crate::syntax::EndPosition::Target => &target.target_end,
        };
        if (rule.multiplicity.is_some() && end.multiplicity.is_none())
            || (rule.role.is_some() && end.role.is_none())
        {
            return Err(error(
                rule.line,
                "mandatory end-label layout references missing multiplicity or role",
            ));
        }
        if diagram.relation_end_layout[..index]
            .iter()
            .any(|prior| prior.name == rule.name && prior.end == rule.end)
        {
            return Err(error(rule.line, "duplicate association-end layout"));
        }
    }
    for (index, rule) in diagram.relation_layout.iter().enumerate() {
        let target = diagram
            .relations
            .iter()
            .find(|relation| relation.name.as_deref() == Some(rule.name.as_str()));
        let Some(target) = target else {
            return Err(error(
                rule.line,
                format!("unknown named relation '{}'", rule.name),
            ));
        };
        if target.display_label().is_none() {
            return Err(error(rule.line, "relation has no label to place"));
        }
        if diagram.relation_layout[..index]
            .iter()
            .any(|prior| prior.name == rule.name)
        {
            return Err(error(
                rule.line,
                format!("duplicate relation layout for '{}'", rule.name),
            ));
        }
    }
    Ok(())
}
