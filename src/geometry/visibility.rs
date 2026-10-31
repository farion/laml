//! Compute one consistent visible graph before measurement and routing.
use crate::styles::{Element, StyleSheet, Value};
use crate::syntax::{Diagram, NodePart};

fn hidden(sheet: &StyleSheet, element: &Element<'_>) -> bool {
    matches!(
        sheet.get(element, "display").map(|d| &d.value),
        Some(Value::Keyword(value)) if value == "none"
    )
}

pub(crate) fn filtered(diagram: &Diagram, sheet: &StyleSheet) -> Result<Diagram, (usize, String)> {
    let mut visible = diagram.clone();
    visible.nodes.retain(|node| {
        !hidden(
            sheet,
            &Element::Node {
                id: &node.id,
                classes: &node.classes,
            },
        )
    });
    visible.relations.retain(|relation| {
        !hidden(
            sheet,
            &Element::Relation {
                name: relation.name.as_deref(),
                classes: &relation.classes,
            },
        )
    });
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
            if !visible.nodes.iter().any(|node| {
                node.id == *id || node.parts.iter().any(|part| {
                matches!(part, NodePart::Member { id: Some(member), .. } if member == id)
            })
            }) {
                return Err((
                    relation.line,
                    format!("visible relation targets hidden node or member '{id}'"),
                ));
            }
        }
    }
    Ok(visible)
}
