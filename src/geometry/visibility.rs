//! Compute one consistent visible graph before measurement and routing.
use crate::styles::{Element, StyleSheet, Value};
use crate::syntax::{Diagram, DiagramType, MemberKind, NodeKind, NodePart};

fn hidden(sheet: &StyleSheet, element: &Element<'_>) -> bool {
    matches!(
        sheet.get(element, "display").map(|d| &d.value),
        Some(Value::Keyword(value)) if value == "none"
    )
}

pub(crate) fn filtered(diagram: &Diagram, sheet: &StyleSheet) -> Result<Diagram, (usize, String)> {
    let mut visible = diagram.clone();
    let hidden_nodes: std::collections::HashSet<_> = diagram
        .nodes
        .iter()
        .filter(|node| {
            hidden(
                &super::scoped_styles(diagram, node, sheet),
                &Element::Node {
                    id: &node.id,
                    classes: &node.classes,
                },
            )
        })
        .map(|node| node.id.as_str())
        .collect();
    visible.nodes.retain(|node| {
        let mut current = Some(node.id.as_str());
        while let Some(id) = current {
            if hidden_nodes.contains(id) {
                return false;
            }
            current = diagram
                .nodes
                .iter()
                .find(|candidate| candidate.id == id)
                .and_then(|parent| parent.parent.as_deref());
        }
        true
    });
    if diagram.settings.diagram_type == DiagramType::Class {
        for node in &mut visible.nodes {
            let scoped =
                super::scoped_styles(diagram, node, sheet).with_ancestor(&node.id, &node.classes);
            let hide = |kind| hidden(&scoped, &Element::Compartment { kind });
            let (attributes, operations, literals) =
                (hide("attributes"), hide("operations"), hide("literals"));
            if !attributes && !operations && !literals {
                continue;
            }
            let classifier = matches!(
                node.kind,
                NodeKind::Class
                    | NodeKind::Interface
                    | NodeKind::DataType
                    | NodeKind::AssociationClass
            );
            let enumeration = node.kind == NodeKind::Enumeration;
            node.parts.retain(|part| match part {
                NodePart::Member {
                    kind: MemberKind::Field,
                    ..
                } if classifier => !attributes,
                NodePart::Member {
                    kind: MemberKind::Method,
                    ..
                } if classifier => !operations,
                NodePart::Member {
                    kind: MemberKind::Literal,
                    ..
                } if enumeration => !literals,
                NodePart::Separator if classifier && (attributes || operations) => false,
                _ => true,
            });
        }
    }
    visible.relations.retain(|relation| {
        !hidden(
            sheet,
            &Element::Relation {
                name: relation.name.as_deref(),
                classes: &relation.classes,
            },
        )
    });
    visible.attachments.retain(|attachment| {
        !hidden(
            sheet,
            &Element::Relation {
                name: attachment.name.as_deref(),
                classes: &attachment.classes,
            },
        )
    });
    for attachment in &visible.attachments {
        if !visible
            .nodes
            .iter()
            .any(|node| node.id == attachment.source)
        {
            return Err((
                attachment.line,
                format!(
                    "visible edge attachment targets hidden node '{}'",
                    attachment.source
                ),
            ));
        }
        if !visible
            .relations
            .iter()
            .any(|relation| relation.name.as_deref() == Some(attachment.target.as_str()))
        {
            return Err((
                attachment.line,
                format!(
                    "visible edge attachment targets hidden relation '{}'",
                    attachment.target
                ),
            ));
        }
    }
    for rule in &visible.layout {
        for id in [&rule.first, &rule.second] {
            if !visible.nodes.iter().any(|node| &node.id == id) {
                return Err((
                    rule.line,
                    format!("mandatory layout targets hidden node '{id}'"),
                ));
            }
        }
    }
    for rule in &visible.relation_end_layout {
        if !visible
            .relations
            .iter()
            .any(|relation| relation.name.as_deref() == Some(rule.name.as_str()))
        {
            return Err((
                rule.line,
                format!(
                    "mandatory end-label layout targets hidden relation '{}'",
                    rule.name
                ),
            ));
        }
    }
    for rule in &visible.relation_layout {
        if !visible
            .relations
            .iter()
            .any(|relation| relation.name.as_deref() == Some(rule.name.as_str()))
        {
            return Err((
                rule.line,
                format!(
                    "mandatory label layout targets hidden relation '{}'",
                    rule.name
                ),
            ));
        }
    }
    for relation in &visible.relations {
        for id in [&relation.from, &relation.to] {
            if !visible.nodes.iter().any(|node| node.has_endpoint(id)) {
                return Err((
                    relation.line,
                    format!("visible relation targets hidden node or member '{id}'"),
                ));
            }
        }
    }
    Ok(visible)
}
