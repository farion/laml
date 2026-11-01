//! Checked structured-classifier internals: composite parts, referenced roles, and ports.
use super::{ParseError, error};
use crate::syntax::{
    ConnectorKind, Diagram, DiagramType, NodeDecl, NodeKind, PortContract, RelationDecl,
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
    if diagram.settings.diagram_type != DiagramType::Composite {
        return Ok(());
    }
    if visibility.is_some() || !stereotypes.is_empty() {
        return Err(error(
            line,
            "composite structure does not accept unchecked classifier modifiers",
        ));
    }
    let valid = match kind {
        NodeKind::CompositeStructure => block && parent.is_none(),
        NodeKind::CompositePart | NodeKind::CompositeRole => {
            !block
                && parent.is_some_and(|owner| {
                    diagram
                        .nodes
                        .iter()
                        .any(|n| n.id == owner && n.kind == NodeKind::CompositeStructure)
                })
        }
        NodeKind::Class | NodeKind::Interface => parent.is_none(),
        _ => false,
    };
    if !valid {
        return Err(error(
            line,
            "composite structure needs one root structure block with directly owned parts and roles; type definitions stay outside",
        ));
    }
    Ok(())
}

pub(super) fn body_member(
    diagram: &Diagram,
    index: usize,
    text: &str,
    line: usize,
) -> Result<(), ParseError> {
    if diagram.settings.diagram_type == DiagramType::Composite
        && diagram.nodes[index].kind == NodeKind::CompositeStructure
        && !text.starts_with("port ")
    {
        return Err(error(
            line,
            "structured classifier body accepts only typed ports, parts, roles and connectors",
        ));
    }
    Ok(())
}

pub(super) fn relation(
    rel: &RelationDecl,
    from: &NodeDecl,
    to: &NodeDecl,
) -> Result<(), ParseError> {
    if rel.label.is_some()
        || rel.appearance.dashed
        || rel.appearance.start != crate::syntax::EndSymbol::None
        || rel.appearance.end != crate::syntax::EndSymbol::None
        || !rel.stereotypes.is_empty()
        || rel.binding.is_some()
        || rel.source_end != Default::default()
        || rel.target_end != Default::default()
        || rel.from == rel.to
        || rel.from == from.id
        || rel.to == to.id
    {
        return Err(error(
            rel.line,
            "composite connector needs two distinct visible typed ports, no association end adornments",
        ));
    }
    let first = from
        .ports
        .iter()
        .find(|p| from.matches_port(p, &rel.from))
        .and_then(|p| p.contract.as_ref());
    let second = to
        .ports
        .iter()
        .find(|p| to.matches_port(p, &rel.to))
        .and_then(|p| p.contract.as_ref());
    let internal =
        |n: &NodeDecl| matches!(n.kind, NodeKind::CompositePart | NodeKind::CompositeRole);
    let valid = match (rel.connector, first, second) {
        (Some(ConnectorKind::Assembly), Some(a), Some(b)) => {
            internal(from)
                && internal(to)
                && from.id != to.id
                && from.parent.is_some()
                && from.parent == to.parent
                && matches!((a,b),(PortContract::Provides(x),PortContract::Requires(y))
                |(PortContract::Requires(x),PortContract::Provides(y)) if x==y)
        }
        (Some(ConnectorKind::Delegation), Some(a), Some(b)) => {
            from.kind == NodeKind::CompositeStructure
                && internal(to)
                && to.parent.as_deref() == Some(from.id.as_str())
                && matches!((a,b),(PortContract::Provides(x),PortContract::Provides(y))
                |(PortContract::Requires(x),PortContract::Requires(y)) if x==y)
        }
        _ => false,
    };
    if !valid {
        return Err(error(
            rel.line,
            "assembly == requires opposite-role matching interface ports on sibling parts/roles; delegation => requires same-role matching ports from owner to its child",
        ));
    }
    Ok(())
}

pub(super) fn validate(diagram: &Diagram) -> Result<(), ParseError> {
    if diagram.settings.diagram_type != DiagramType::Composite {
        return Ok(());
    }
    let owners: Vec<_> = diagram
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::CompositeStructure)
        .collect();
    if owners.len() != 1 {
        return Err(error(
            1,
            "composite diagram requires exactly one structured classifier",
        ));
    }
    let owner = owners[0];
    if !diagram
        .nodes
        .iter()
        .any(|n| n.parent.as_deref() == Some(owner.id.as_str()))
    {
        return Err(error(
            owner.line,
            "internal structure needs at least one part or role",
        ));
    }
    if diagram.relations.is_empty() {
        return Err(error(
            owner.line,
            "internal structure needs an explicitly typed connector",
        ));
    }
    for node in &diagram.nodes {
        if node.kind == NodeKind::Class && node.id == owner.id {
            return Err(error(
                node.line,
                "a structured classifier cannot shadow its defining class",
            ));
        }
    }
    Ok(())
}
