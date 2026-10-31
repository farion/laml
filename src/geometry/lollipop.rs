//! Relation-specific compact UML interface realization, expanded consistently for place/route.
use crate::styles::{Element, StyleSheet, Value};
use crate::syntax::{Diagram, DiagramType, EdgeAppearance, EndSymbol, NodeDecl, NodeKind};
use std::collections::HashSet;

pub(super) fn expand(
    diagram: &mut Diagram,
    original: &Diagram,
    sheet: &StyleSheet,
) -> Result<(), (usize, String)> {
    let decorated: Vec<_> = diagram.relations.iter().enumerate().filter_map(|(index, relation)| {
        let element = Element::Relation { name: relation.name.as_deref(), classes: &relation.classes };
        matches!(sheet.get(&element, "interface-notation").map(|d| &d.value), Some(Value::Keyword(mode)) if mode == "lollipop")
            .then_some(index)
    }).collect();
    if decorated.is_empty() {
        return Ok(());
    }
    let mut targets = HashSet::new();
    let mut substitutions = Vec::new();
    for index in decorated.iter().copied() {
        let relation = &diagram.relations[index];
        let bad = || {
            (
                relation.line,
                "lollipop requires a named class-to-interface realization with no extra adornments"
                    .into(),
            )
        };
        let Some(provider) = diagram.nodes.iter().find(|n| n.id == relation.from) else {
            return Err(bad());
        };
        let Some(interface) = diagram.nodes.iter().find(|n| n.id == relation.to) else {
            return Err(bad());
        };
        if diagram.settings.diagram_type != DiagramType::Class
            || relation.name.is_none()
            || provider.kind != NodeKind::Class
            || interface.kind != NodeKind::Interface
            || !relation.appearance.dashed
            || relation.appearance.start != EndSymbol::None
            || relation.appearance.end != EndSymbol::Triangle
            || relation.label.is_some()
            || !relation.stereotypes.is_empty()
            || relation.binding.is_some()
            || relation.source_end != Default::default()
            || relation.target_end != Default::default()
            || original
                .nodes
                .iter()
                .find(|node| node.id == interface.id)
                .is_some_and(|node| !node.parts.is_empty() || !node.ports.is_empty())
            || !interface.stereotypes.is_empty()
            || interface.visibility.is_some()
            || interface
                .label
                .as_deref()
                .is_some_and(|label| label.contains('\n'))
            || diagram
                .layout
                .iter()
                .any(|rule| rule.first == interface.id || rule.second == interface.id)
            || diagram.attachments.iter().any(|a| {
                a.source == interface.id || a.target == relation.name.as_deref().unwrap_or_default()
            })
            || diagram
                .relation_end_layout
                .iter()
                .any(|rule| relation.name.as_deref() == Some(rule.name.as_str()))
            || diagram
                .relation_layout
                .iter()
                .any(|rule| relation.name.as_deref() == Some(rule.name.as_str()))
        {
            return Err(bad());
        }
        targets.insert(interface.id.clone());
        substitutions.push((
            index,
            provider.parent.clone(),
            interface
                .label
                .clone()
                .unwrap_or_else(|| interface.id.clone()),
            interface.classes.clone(),
        ));
    }
    for (index, relation) in diagram.relations.iter().enumerate() {
        if (targets.contains(&relation.from) || targets.contains(&relation.to))
            && !decorated.contains(&index)
        {
            return Err((
                relation.line,
                "lollipop interface box is still referenced by another relation".into(),
            ));
        }
    }
    for (index, parent, label, classes) in substitutions {
        let mut id = format!("_laml_lollipop_{index}");
        while diagram.nodes.iter().any(|node| node.id == id)
            || diagram
                .relations
                .iter()
                .any(|rel| rel.name.as_deref() == Some(id.as_str()))
        {
            id.push('_');
        }
        diagram.nodes.push(NodeDecl {
            id: id.clone(),
            label: Some(label),
            classes,
            stereotypes: Vec::new(),
            kind: NodeKind::Lollipop,
            abstract_node: false,
            visibility: None,
            templates: Vec::new(),
            note_lines: None,
            parts: Vec::new(),
            ports: Vec::new(),
            parent,
            line: diagram.relations[index].line,
        });
        diagram.relations[index].to = id;
        diagram.relations[index].appearance = EdgeAppearance::default();
    }
    diagram.nodes.retain(|node| !targets.contains(&node.id));
    Ok(())
}
