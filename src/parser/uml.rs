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
    let member_end = from.id != relation.from || to.id != relation.to;
    let note_end = from.kind == NodeKind::Note || to.kind == NodeKind::Note;
    let end_label = [&relation.source_end, &relation.target_end]
        .iter()
        .any(|end| end.multiplicity.is_some() || end.role.is_some());
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
    if relation.appearance.end == EndSymbol::Triangle
        && !relation.appearance.dashed
        && relation.from == relation.to
    {
        return Err(error(
            relation.line,
            "a classifier cannot generalize itself",
        ));
    }
    Ok(())
}
