//! Checked UML use-case image notation: system boundary, actors and directional dependencies.
use super::{ParseError, error};
use crate::syntax::{
    Diagram, DiagramType, EndPosition, EndSymbol, NodeDecl, NodeKind, PortSide, RelationDecl,
};

pub(super) fn node(
    diagram: &Diagram,
    kind: NodeKind,
    parent: Option<&str>,
    block: bool,
    visibility: Option<char>,
    stereotypes: &[String],
    line: usize,
) -> Result<(), ParseError> {
    if diagram.settings.diagram_type != DiagramType::UseCase {
        return Ok(());
    }
    if visibility.is_some() || !stereotypes.is_empty() {
        return Err(error(
            line,
            "use-case elements do not accept classifier modifiers or unchecked stereotypes",
        ));
    }
    match kind {
        NodeKind::UseCaseSystem if !block || parent.is_some() => Err(error(
            line,
            "system boundary requires a root-level declaration block",
        )),
        NodeKind::UseCaseActor if block || parent.is_some() => Err(error(
            line,
            "actors must be outside the system boundary and have no body",
        )),
        NodeKind::UseCase
            if block
                || parent.is_none_or(|owner| {
                    !diagram
                        .nodes
                        .iter()
                        .any(|n| n.id == owner && n.kind == NodeKind::UseCaseSystem)
                }) =>
        {
            Err(error(
                line,
                "use cases must be declared inside a system boundary",
            ))
        }
        NodeKind::Note if block && parent.is_some() => Err(error(
            line,
            "note blocks inside a system boundary are not supported",
        )),
        _ => Ok(()),
    }
}

pub(super) fn relation(
    relation: &RelationDecl,
    from: &NodeDecl,
    to: &NodeDecl,
) -> Result<(), ParseError> {
    if relation.from != from.id
        || relation.to != to.id
        || relation.label.is_some()
        || relation.source_end != Default::default()
        || relation.target_end != Default::default()
        || relation.binding.is_some()
        || relation.connector.is_some()
        || relation.appearance.start != EndSymbol::None
    {
        return Err(error(
            relation.line,
            "use-case relations require plain nodes and checked UML endpoints",
        ));
    }
    let association = relation.stereotypes.is_empty()
        && !relation.appearance.dashed
        && relation.appearance.end == EndSymbol::None
        && matches!(
            (from.kind, to.kind),
            (NodeKind::UseCaseActor, NodeKind::UseCase)
                | (NodeKind::UseCase, NodeKind::UseCaseActor)
        );
    let dependency = relation.appearance.dashed
        && relation.appearance.end == EndSymbol::Arrow
        && from.kind == NodeKind::UseCase
        && to.kind == NodeKind::UseCase
        && from.parent == to.parent
        && from.id != to.id
        && matches!(relation.stereotypes.as_slice(), [name] if matches!(name.as_str(), "include" | "extend"));
    let generalization = relation.stereotypes.is_empty()
        && !relation.appearance.dashed
        && relation.appearance.end == EndSymbol::Triangle
        && from.kind == to.kind
        && from.id != to.id
        && matches!(from.kind, NodeKind::UseCaseActor | NodeKind::UseCase)
        && from.parent == to.parent;
    if association || dependency || generalization {
        Ok(())
    } else {
        Err(error(
            relation.line,
            "use-case relation requires Actor -- UseCase, UseCase ..> UseCase <<include|extend>>, or a same-kind --|> generalization; arrow direction is semantic",
        ))
    }
}

pub(super) fn validate(diagram: &Diagram) -> Result<(), ParseError> {
    if diagram.settings.diagram_type != DiagramType::UseCase {
        return Ok(());
    }
    if diagram
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::UseCaseSystem)
        .count()
        != 1
    {
        return Err(error(
            1,
            "use-case diagram needs exactly one system boundary",
        ));
    }
    if !diagram.nodes.iter().any(|n| n.kind == NodeKind::UseCase) {
        return Err(error(1, "system boundary needs at least one use case"));
    }
    for rule in &diagram.relation_end_layout {
        if matches!(rule.side, Some(PortSide::Top | PortSide::Bottom)) {
            let relation = diagram
                .relations
                .iter()
                .find(|r| r.name.as_deref() == Some(rule.name.as_str()));
            let actor = relation.and_then(|r| {
                diagram.nodes.iter().find(|n| {
                    n.id.as_str()
                        == match rule.end {
                            EndPosition::Source => r.from.as_str(),
                            EndPosition::Target => r.to.as_str(),
                        }
                })
            });
            if actor.is_some_and(|n| n.kind == NodeKind::UseCaseActor) {
                return Err(error(
                    rule.line,
                    "actor stick-figure attachments support only left or right",
                ));
            }
        }
    }
    for relation in diagram
        .relations
        .iter()
        .filter(|r| r.appearance.end == EndSymbol::Triangle)
    {
        let mut parents = std::collections::HashMap::<&str, Vec<&str>>::new();
        for prior in diagram
            .relations
            .iter()
            .filter(|r| r.appearance.end == EndSymbol::Triangle && r.line <= relation.line)
        {
            parents.entry(&prior.from).or_default().push(&prior.to);
        }
        let mut frontier = vec![relation.to.as_str()];
        let mut visited = std::collections::HashSet::new();
        while let Some(id) = frontier.pop() {
            if id == relation.from {
                return Err(error(relation.line, "use-case generalization cycle"));
            }
            if visited.insert(id) {
                frontier.extend(parents.get(id).into_iter().flatten().copied());
            }
        }
    }
    Ok(())
}
