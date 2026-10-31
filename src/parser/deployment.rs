//! UML deployment subset: node containment and relation meaning are explicit.
use super::{ParseError, error};
use crate::syntax::{Diagram, DiagramType, EndSymbol, NodeDecl, NodeKind, RelationDecl};

pub(super) fn target(kind: NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::DeploymentNode | NodeKind::Device | NodeKind::ExecutionEnvironment
    )
}

pub(super) fn validate_node(
    diagram: &Diagram,
    kind: NodeKind,
    parent: Option<&str>,
    block: bool,
    visibility: Option<char>,
    stereotypes: &[String],
    line: usize,
) -> Result<(), ParseError> {
    if diagram.settings.diagram_type != DiagramType::Deployment {
        return Ok(());
    }
    if visibility.is_some() && kind != NodeKind::Package {
        return Err(error(
            line,
            "deployment nodes and artifacts do not use classifier visibility",
        ));
    }
    if stereotypes.iter().any(|name| {
        matches!(
            name.as_str(),
            "device" | "executionEnvironment" | "artifact" | "component" | "deploy" | "manifest"
        )
    }) {
        return Err(error(
            line,
            "reserved UML deployment keywords cannot be used as extra stereotypes",
        ));
    }
    if block && matches!(kind, NodeKind::Artifact | NodeKind::Component) {
        return Err(error(
            line,
            "artifact and component have no nested deployment declarations",
        ));
    }
    if let Some(parent) = parent {
        let owner = diagram
            .nodes
            .iter()
            .find(|node| node.id == parent)
            .ok_or_else(|| error(line, "unknown deployment container"))?;
        if target(owner.kind) && !target(kind) && kind != NodeKind::Artifact {
            return Err(error(
                line,
                "node may contain only nodes or deployed artifacts",
            ));
        }
        if !target(owner.kind) && owner.kind != NodeKind::Package {
            return Err(error(
                line,
                "only nodes or packages can contain deployment elements",
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_relation(
    relation: &RelationDecl,
    from: &NodeDecl,
    to: &NodeDecl,
) -> Result<(), ParseError> {
    let plain = !relation.appearance.dashed
        && relation.appearance.start == EndSymbol::None
        && relation.appearance.end == EndSymbol::None;
    let dependency = relation.appearance.dashed
        && relation.appearance.start == EndSymbol::None
        && relation.appearance.end == EndSymbol::Arrow;
    let valid = match (from.kind, to.kind, relation.stereotypes.as_slice()) {
        (a, b, []) if target(a) && target(b) => plain,
        (NodeKind::Artifact, b, [label]) if label == "deploy" && target(b) => dependency,
        (NodeKind::Artifact, NodeKind::Component, [label]) if label == "manifest" => dependency,
        _ => false,
    };
    if !valid
        || relation.connector.is_some()
        || relation.binding.is_some()
        || relation.from != from.id
        || relation.to != to.id
        || relation.source_end != Default::default()
        || relation.target_end != Default::default()
    {
        return Err(error(
            relation.line,
            "deployment requires Node -- Node communication, Artifact ..> Node <<deploy>>, or Artifact ..> Component <<manifest>>; use containment for implicit deployment",
        ));
    }
    Ok(())
}
