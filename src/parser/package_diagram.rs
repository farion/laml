//! Checked UML package image notation; no namespace import or merge execution.
use super::{ParseError, error};
use crate::syntax::{Diagram, DiagramType, EndSymbol, NodeDecl, NodeKind, RelationDecl};

pub(super) fn node(
    diagram: &Diagram,
    kind: NodeKind,
    parent: Option<&str>,
    visibility: Option<char>,
    stereotypes: &[String],
    line: usize,
) -> Result<(), ParseError> {
    if diagram.settings.diagram_type != DiagramType::Package {
        return Ok(());
    }
    if !stereotypes.is_empty() || visibility.is_some_and(|v| !matches!(v, '+' | '-')) {
        return Err(error(
            line,
            "package diagrams reject unchecked stereotypes and non-public/private visibility",
        ));
    }
    if visibility.is_some() && (kind != NodeKind::Package || parent.is_none()) {
        return Err(error(
            line,
            "only nested packages can show public or private visibility",
        ));
    }
    if let Some(owner) = parent
        && !diagram
            .nodes
            .iter()
            .any(|node| node.id == owner && node.kind == NodeKind::Package)
    {
        return Err(error(line, "package members must be inside a package"));
    }
    Ok(())
}

fn descendant(diagram: &Diagram, child: &NodeDecl, ancestor: &str) -> bool {
    let mut parent = child.parent.as_deref();
    while let Some(id) = parent {
        if id == ancestor {
            return true;
        }
        parent = diagram
            .nodes
            .iter()
            .find(|node| node.id == id)
            .and_then(|node| node.parent.as_deref());
    }
    false
}

pub(super) fn relation(
    diagram: &Diagram,
    relation: &RelationDecl,
    from: &NodeDecl,
    to: &NodeDecl,
) -> Result<(), ParseError> {
    let kind = relation.stereotypes.as_slice();
    let known = kind.is_empty()
        || matches!(kind, [name] if matches!(name.as_str(), "import" | "access" | "merge"));
    if from.kind != NodeKind::Package
        || to.kind != NodeKind::Package
        || relation.from != from.id
        || relation.to != to.id
        || from.id == to.id
        || !relation.appearance.dashed
        || relation.appearance.start != EndSymbol::None
        || relation.appearance.end != EndSymbol::Arrow
        || relation.label.is_some()
        || relation.binding.is_some()
        || relation.connector.is_some()
        || relation.activity_flow.is_some()
        || relation.source_end != Default::default()
        || relation.target_end != Default::default()
        || !known
    {
        return Err(error(
            relation.line,
            "package relationship requires Package ..> Package, optionally <<import>>, <<access>> or <<merge>>, with no extra adornments",
        ));
    }
    if matches!(kind, [name] if matches!(name.as_str(), "import" | "access"))
        && descendant(diagram, to, &from.id)
    {
        return Err(error(
            relation.line,
            "a package cannot import or access its own contained package",
        ));
    }
    Ok(())
}

pub(super) fn validate(diagram: &Diagram) -> Result<(), ParseError> {
    if diagram.settings.diagram_type != DiagramType::Package {
        return Ok(());
    }
    if !diagram
        .nodes
        .iter()
        .any(|node| node.kind == NodeKind::Package)
    {
        return Err(error(1, "package diagram requires a package"));
    }
    // A merge cycle is not a meaningful depiction of receiving versus merged packages.
    let mut graph = std::collections::HashMap::<&str, Vec<&str>>::new();
    for edge in diagram
        .relations
        .iter()
        .filter(|rel| matches!(rel.stereotypes.as_slice(), [name] if name == "merge"))
    {
        let mut frontier = vec![edge.to.as_str()];
        let mut seen = std::collections::HashSet::new();
        while let Some(id) = frontier.pop() {
            if id == edge.from {
                return Err(error(edge.line, "package merge cycle"));
            }
            if seen.insert(id) {
                frontier.extend(graph.get(id).into_iter().flatten().copied());
            }
        }
        graph.entry(&edge.from).or_default().push(&edge.to);
    }
    Ok(())
}
