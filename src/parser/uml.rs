//! Conservative UML-specific relationship checks; graphics alone do not establish UML meaning.
use super::{ParseError, error};
use crate::syntax::{Diagram, DiagramType, EndSymbol, NodeKind, RelationDecl};

pub(super) fn validate_relation(
    diagram: &Diagram,
    relation: &RelationDecl,
) -> Result<(), ParseError> {
    if diagram.settings.diagram_type == DiagramType::Generic {
        return Ok(());
    }
    let owner = |id: &str| diagram.nodes.iter().find(|node| node.has_endpoint(id));
    let from = owner(&relation.from).expect("endpoints validated before UML notation");
    let to = owner(&relation.to).expect("endpoints validated before UML notation");
    if let Some(bindings) = &relation.binding {
        if diagram.settings.diagram_type != DiagramType::Class
            || from.kind != NodeKind::Class
            || to.kind != NodeKind::Class
            || from.id == to.id
            || to.templates.is_empty()
            || !relation.appearance.dashed
            || relation.appearance.start != EndSymbol::None
            || relation.appearance.end != EndSymbol::Arrow
            || relation.label.is_some()
            || !relation.stereotypes.is_empty()
            || relation.source_end != Default::default()
            || relation.target_end != Default::default()
        {
            return Err(error(
                relation.line,
                "<<bind>> needs a dashed open-arrow dependency from a class to a local template class",
            ));
        }
        for (formal, _) in bindings {
            if !to.templates.contains(formal) {
                return Err(error(
                    relation.line,
                    format!("unknown template parameter '{formal}' on '{}'", to.id),
                ));
            }
        }
    }
    let member_end = from.id != relation.from || to.id != relation.to;
    let note_end = from.kind == NodeKind::Note || to.kind == NodeKind::Note;
    let end_label = [&relation.source_end, &relation.target_end]
        .iter()
        .any(|end| end.multiplicity.is_some() || end.role.is_some() || end.qualifier.is_some());
    let unmarked_dotted = relation.appearance.dashed
        && relation.appearance.start == EndSymbol::None
        && relation.appearance.end == EndSymbol::None;
    if note_end {
        if !unmarked_dotted || from.kind == to.kind || end_label {
            return Err(error(
                relation.line,
                "a UML note attaches only by an unadorned dotted line to a non-note element",
            ));
        }
        return Ok(());
    }
    if member_end {
        return Err(error(
            relation.line,
            "UML relationship cannot target a member except as a note attachment",
        ));
    }
    if from.kind == NodeKind::Package || to.kind == NodeKind::Package {
        return Err(error(
            relation.line,
            "UML package relationships require a separately validated notation",
        ));
    }
    if diagram.settings.diagram_type == DiagramType::Object {
        if relation.appearance.dashed
            || relation.appearance.start != EndSymbol::None
            || relation.appearance.end != EndSymbol::None
        {
            return Err(error(
                relation.line,
                "object link must be an unadorned solid line",
            ));
        }
        return Ok(());
    }
    let whole = if matches!(
        relation.appearance.start,
        EndSymbol::OpenDiamond | EndSymbol::FilledDiamond
    ) {
        Some(from)
    } else if matches!(
        relation.appearance.end,
        EndSymbol::OpenDiamond | EndSymbol::FilledDiamond
    ) {
        Some(to)
    } else {
        None
    };
    if whole.is_some_and(|node| !matches!(node.kind, NodeKind::Class | NodeKind::AssociationClass))
    {
        return Err(error(
            relation.line,
            "aggregation or composition requires a class at the whole end",
        ));
    }
    if end_label
        && (relation.appearance.dashed
            || relation.appearance.start == EndSymbol::Triangle
            || relation.appearance.end == EndSymbol::Triangle)
    {
        return Err(error(
            relation.line,
            "association-end roles and multiplicities require an association",
        ));
    }
    if relation.appearance.end == EndSymbol::Triangle && !relation.appearance.dashed {
        if relation.from == relation.to {
            return Err(error(
                relation.line,
                "a classifier cannot generalize itself",
            ));
        }
        if from.kind != to.kind || matches!(from.kind, NodeKind::Association | NodeKind::Package) {
            return Err(error(
                relation.line,
                "unsupported generalization between different UML classifier kinds",
            ));
        }
    }
    Ok(())
}
