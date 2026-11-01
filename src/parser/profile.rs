//! Checked UML 2.5.1 profile-definition notation, not profile application execution.
use super::{ParseError, error};
use crate::syntax::{Diagram, DiagramType, EndSymbol, NodeDecl, NodeKind, RelationDecl};

const METACLASSES: &[&str] = &[
    "Activity",
    "Artifact",
    "Association",
    "Class",
    "Classifier",
    "Component",
    "Dependency",
    "Element",
    "Interface",
    "NamedElement",
    "Operation",
    "Package",
    "Port",
    "Property",
    "State",
    "UseCase",
];

pub(super) fn node(
    diagram: &Diagram,
    kind: NodeKind,
    parent: Option<&str>,
    block: bool,
    visibility: Option<char>,
    stereotypes: &[String],
    line: usize,
) -> Result<(), ParseError> {
    if diagram.settings.diagram_type != DiagramType::Profile {
        return Ok(());
    }
    let inside_profile = parent.is_some_and(|id| {
        diagram
            .nodes
            .iter()
            .any(|node| node.id == id && node.kind == NodeKind::Profile)
    });
    if visibility.is_some()
        || !stereotypes.is_empty()
        || match kind {
            NodeKind::Profile => parent.is_some() || !block,
            NodeKind::Stereotype => !inside_profile || block,
            NodeKind::Metaclass | NodeKind::Package => parent.is_some() || block,
            _ => true,
        }
    {
        return Err(error(
            line,
            "profile diagram requires a profile block with stereotype definitions, and external leaf metaclasses/packages; no unchecked node modifiers",
        ));
    }
    Ok(())
}

pub(super) fn decorate(node: &mut NodeDecl, name: &str, line: usize) -> Result<(), ParseError> {
    if !matches!(
        node.kind,
        NodeKind::Profile | NodeKind::Stereotype | NodeKind::Metaclass
    ) {
        return Ok(());
    }
    if node.abstract_node || node.label.as_deref().is_some_and(|label| label != name) {
        return Err(error(
            line,
            "profile, stereotype and metaclass names cannot be hidden by modifiers or custom labels",
        ));
    }
    if node.kind == NodeKind::Metaclass && !METACLASSES.contains(&name) {
        return Err(error(
            line,
            format!("'{name}' is outside this checked UML metaclass subset"),
        ));
    }
    let keyword = match node.kind {
        NodeKind::Profile => "profile",
        NodeKind::Stereotype => "stereotype",
        NodeKind::Metaclass => "metaclass",
        _ => unreachable!(),
    };
    node.label = Some(format!("«{keyword}»\n{name}"));
    Ok(())
}

pub(super) fn relation(
    diagram: &Diagram,
    edge: &RelationDecl,
    from: &NodeDecl,
    to: &NodeDecl,
) -> Result<(), ParseError> {
    let extension = edge.appearance.end == EndSymbol::FilledTriangle
        && !edge.appearance.dashed
        && from.kind == NodeKind::Stereotype
        && to.kind == NodeKind::Metaclass
        && edge.stereotypes.is_empty();
    let application = edge.appearance.end == EndSymbol::Arrow
        && edge.appearance.dashed
        && from.kind == NodeKind::Package
        && to.kind == NodeKind::Profile
        && matches!(edge.stereotypes.as_slice(), [name] if matches!(name.as_str(), "apply" | "strict"));
    let reference = edge.appearance.end == EndSymbol::Arrow
        && edge.appearance.dashed
        && from.kind == NodeKind::Profile
        && to.kind == NodeKind::Metaclass
        && edge.stereotypes.as_slice() == ["reference"];
    if !(extension || application || reference)
        || from.id == to.id
        || edge.from != from.id
        || edge.to != to.id
        || edge.appearance.start != EndSymbol::None
        || edge.label.is_some()
        || edge.binding.is_some()
        || edge.connector.is_some()
        || edge.activity_flow.is_some()
        || edge.source_end != Default::default()
        || edge.target_end != Default::default()
    {
        return Err(error(
            edge.line,
            "profile relationships are stereotype --^> metaclass extensions, package ..> profile <<apply>>/<<strict>>, or profile ..> metaclass <<reference>>",
        ));
    }
    if application && from.parent.is_some() {
        return Err(error(
            edge.line,
            "profile application requires an external package",
        ));
    }
    if extension
        && !diagram
            .nodes
            .iter()
            .any(|n| n.id == from.parent.as_deref().unwrap_or("") && n.kind == NodeKind::Profile)
    {
        return Err(error(
            edge.line,
            "extension stereotype must belong to a profile",
        ));
    }
    Ok(())
}

pub(super) fn validate(diagram: &Diagram) -> Result<(), ParseError> {
    if diagram.settings.diagram_type != DiagramType::Profile {
        return Ok(());
    }
    let stereotypes: Vec<_> = diagram
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Stereotype)
        .collect();
    if !diagram.nodes.iter().any(|n| n.kind == NodeKind::Profile) || stereotypes.is_empty() {
        return Err(error(
            1,
            "profile diagram needs a profile with a stereotype definition",
        ));
    }
    for profile in diagram
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Profile)
    {
        if !stereotypes
            .iter()
            .any(|node| node.parent.as_deref() == Some(profile.id.as_str()))
        {
            return Err(error(
                profile.line,
                "each profile in this subset must own a stereotype definition",
            ));
        }
    }
    let mut unique = std::collections::HashSet::new();
    for relation in &diagram.relations {
        if !unique.insert((&relation.from, &relation.to)) {
            return Err(error(
                relation.line,
                "duplicate profile extension, reference or application",
            ));
        }
    }
    for stereotype in stereotypes {
        let extensions: Vec<_> = diagram
            .relations
            .iter()
            .filter(|r| r.from == stereotype.id && r.appearance.end == EndSymbol::FilledTriangle)
            .collect();
        if extensions.is_empty() {
            return Err(error(
                stereotype.line,
                "stereotype needs an explicit extension to a metaclass",
            ));
        }
        // UML extension alone is sufficient; an explicit metaclass reference is optional.
    }
    Ok(())
}
