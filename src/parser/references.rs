//! Resolve globally named and scoped graph references before layout.
use super::{Diagram, ParseError, error, uml};
use crate::syntax::{DiagramType, NodeDecl};

fn resolve(nodes: &[NodeDecl], scope: Option<&str>, raw: &str) -> String {
    let mut current = scope;
    while let Some(owner) = current {
        let candidate = format!("{owner}.{raw}");
        if nodes.iter().any(|node| node.has_endpoint(&candidate)) {
            return candidate;
        }
        current = nodes
            .iter()
            .find(|node| node.id == owner)
            .and_then(|node| node.parent.as_deref());
    }
    raw.into()
}

pub(super) fn resolve_and_validate(diagram: &mut Diagram) -> Result<(), ParseError> {
    for relation in &mut diagram.relations {
        relation.from = resolve(&diagram.nodes, relation.scope.as_deref(), &relation.from);
        relation.to = resolve(&diagram.nodes, relation.scope.as_deref(), &relation.to);
    }
    for attachment in &mut diagram.attachments {
        attachment.source = resolve(
            &diagram.nodes,
            attachment.scope.as_deref(),
            &attachment.source,
        );
    }
    validate_references(diagram)
}

fn validate_references(diagram: &Diagram) -> Result<(), ParseError> {
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
        for port in &node.ports {
            if port.explicit && !assigned.insert(port.id.as_str()) {
                return Err(error(
                    port.line,
                    format!("duplicate global ID '{}'", port.id),
                ));
            }
        }
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
    for attachment in &diagram.attachments {
        if let Some(id) = attachment.name.as_deref()
            && !assigned.insert(id)
        {
            return Err(error(
                attachment.line,
                format!("duplicate global ID '{id}'"),
            ));
        }
        let node = diagram
            .nodes
            .iter()
            .find(|node| node.id == attachment.source)
            .ok_or_else(|| {
                error(
                    attachment.line,
                    format!(
                        "edge attachment source '{}' must be a node",
                        attachment.source
                    ),
                )
            })?;
        if diagram.settings.diagram_type != DiagramType::Generic
            && !matches!(
                node.kind,
                crate::syntax::NodeKind::Note | crate::syntax::NodeKind::AssociationClass
            )
        {
            return Err(error(
                attachment.line,
                "UML edge attachment source must be a note or association class",
            ));
        }
        if !diagram
            .relations
            .iter()
            .any(|relation| relation.name.as_deref() == Some(attachment.target.as_str()))
        {
            return Err(error(
                attachment.line,
                format!("unknown target relation '{}'", attachment.target),
            ));
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
            && ![
                (&relation.from, &relation.to),
                (&relation.to, &relation.from),
            ]
            .iter()
            .any(|(class, junction)| {
                diagram.nodes.iter().any(|node| {
                    node.id == **class && node.kind == crate::syntax::NodeKind::AssociationClass
                }) && diagram.nodes.iter().any(|node| {
                    node.id == **junction && node.kind == crate::syntax::NodeKind::Association
                })
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
                            .ports
                            .iter()
                            .filter(|port| node.matches_port(port, endpoint))
                            .count()
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
    uml::validate_generalization_cycles(diagram)?;
    let mut association_class_targets = std::collections::HashSet::new();
    for node in &diagram.nodes {
        if node.kind == crate::syntax::NodeKind::AssociationClass {
            let attachments: Vec<_> = diagram
                .attachments
                .iter()
                .filter(|link| link.source == node.id)
                .collect();
            let diamond_links: Vec<_> = diagram
                .relations
                .iter()
                .filter(|relation| {
                    (relation.from == node.id
                        && diagram.nodes.iter().any(|n| {
                            n.id == relation.to && n.kind == crate::syntax::NodeKind::Association
                        }))
                        || (relation.to == node.id
                            && diagram.nodes.iter().any(|n| {
                                n.id == relation.from
                                    && n.kind == crate::syntax::NodeKind::Association
                            }))
                })
                .collect();
            if attachments.len() + diamond_links.len() != 1 {
                return Err(error(
                    node.line,
                    "association class must attach to exactly one named binary association or n-ary diamond",
                ));
            }
            if let Some(link) = diamond_links.first() {
                let other = if link.from == node.id {
                    &link.to
                } else {
                    &link.from
                };
                if !association_class_targets.insert(other.as_str()) {
                    return Err(error(
                        link.line,
                        "one association cannot own multiple association classes",
                    ));
                }
                let diamond = diagram
                    .nodes
                    .iter()
                    .find(|n| n.id == *other)
                    .expect("validated diamond");
                if !diamond.label.as_deref().unwrap_or_default().is_empty()
                    && diamond.label.as_deref() != Some(node.label.as_deref().unwrap_or(&node.id))
                {
                    return Err(error(
                        link.line,
                        "association class and n-ary diamond must have the same visible name",
                    ));
                }
                continue;
            }
            let target = diagram
                .relations
                .iter()
                .find(|relation| relation.name.as_deref() == Some(attachments[0].target.as_str()))
                .expect("checked named relation");
            if !association_class_targets.insert(attachments[0].target.as_str()) {
                return Err(error(
                    attachments[0].line,
                    "one association cannot own multiple association classes",
                ));
            }
            if target
                .label
                .as_deref()
                .is_some_and(|label| label != node.label.as_deref().unwrap_or(&node.id))
            {
                return Err(error(
                    attachments[0].line,
                    "association class and association path must have the same visible name",
                ));
            }
            if target.appearance.dashed
                || target.appearance.start != crate::syntax::EndSymbol::None
                || target.appearance.end != crate::syntax::EndSymbol::None
                || target.from == target.to
                || ![&target.from, &target.to].iter().all(|id| {
                    diagram.nodes.iter().any(|other| {
                        &other.id == *id
                            && matches!(
                                other.kind,
                                crate::syntax::NodeKind::Class
                                    | crate::syntax::NodeKind::Interface
                                    | crate::syntax::NodeKind::DataType
                            )
                    })
                })
            {
                return Err(error(
                    attachments[0].line,
                    "association class requires a plain binary association between classifiers",
                ));
            }
        }
        if node.kind != crate::syntax::NodeKind::Association {
            continue;
        }
        let mut participants = std::collections::HashSet::new();
        for relation in &diagram.relations {
            let (other, junction_end) = if relation.from == node.id {
                (&relation.to, &relation.source_end)
            } else if relation.to == node.id {
                (&relation.from, &relation.target_end)
            } else {
                continue;
            };
            let other_node = diagram
                .nodes
                .iter()
                .find(|candidate| candidate.id == *other)
                .ok_or_else(|| {
                    error(
                        relation.line,
                        "association arm must end at a classifier node",
                    )
                })?;
            if other_node.kind == crate::syntax::NodeKind::AssociationClass
                && relation.appearance.dashed
                && relation.appearance.start == crate::syntax::EndSymbol::None
                && relation.appearance.end == crate::syntax::EndSymbol::None
            {
                continue;
            }
            if other_node.kind == crate::syntax::NodeKind::Note
                && relation.appearance.dashed
                && relation.appearance.start == crate::syntax::EndSymbol::None
                && relation.appearance.end == crate::syntax::EndSymbol::None
            {
                continue;
            }
            if !matches!(
                other_node.kind,
                crate::syntax::NodeKind::Class
                    | crate::syntax::NodeKind::Interface
                    | crate::syntax::NodeKind::Enumeration
                    | crate::syntax::NodeKind::DataType
            ) || relation.appearance.dashed
                || relation.appearance.start != crate::syntax::EndSymbol::None
                || relation.appearance.end != crate::syntax::EndSymbol::None
                || relation.label.is_some()
                || !relation.stereotypes.is_empty()
                || junction_end.multiplicity.is_some()
                || junction_end.role.is_some()
                || relation.source_end.qualifier.is_some()
                || relation.target_end.qualifier.is_some()
            {
                return Err(error(
                    relation.line,
                    "n-ary association arms must be plain, unlabeled links to classifiers",
                ));
            }
            participants.insert(other);
        }
        if participants.len() < 3 {
            return Err(error(
                node.line,
                "n-ary association requires at least three distinct classifier arms",
            ));
        }
    }
    for rule in &diagram.layout {
        for id in [&rule.first, &rule.second] {
            if !diagram.nodes.iter().any(|node| &node.id == id) {
                return Err(error(rule.line, format!("unknown node '{id}'")));
            }
        }
    }
    for (index, rule) in diagram.port_layout.iter().enumerate() {
        let targets = diagram
            .nodes
            .iter()
            .flat_map(|node| {
                node.ports
                    .iter()
                    .filter(move |port| node.matches_port(port, &rule.id))
            })
            .count();
        if targets != 1 {
            return Err(error(
                rule.line,
                format!("unknown or ambiguous port '{}'", rule.id),
            ));
        }
        if diagram.port_layout[..index]
            .iter()
            .any(|prior| prior.id == rule.id)
        {
            return Err(error(
                rule.line,
                format!("duplicate port layout for '{}'", rule.id),
            ));
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
        if rule.side.is_some() {
            let endpoint = match rule.end {
                crate::syntax::EndPosition::Source => &target.from,
                crate::syntax::EndPosition::Target => &target.to,
            };
            if !diagram.nodes.iter().any(|node| &node.id == endpoint) {
                return Err(error(
                    rule.line,
                    "mandatory relation side requires a direct node endpoint",
                ));
            }
        }
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
