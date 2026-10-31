//! Component overview notation: dependencies and explicit interface contracts.
use super::{ParseError, error};
use crate::syntax::{EndSymbol, NodeDecl, NodeKind, RelationDecl};

pub(super) fn validate(
    relation: &RelationDecl,
    from: &NodeDecl,
    to: &NodeDecl,
) -> Result<(), ParseError> {
    if let Some(connector) = relation.connector {
        return super::component_connectors::validate(relation, from, to, connector);
    }
    let dependency = relation.appearance.dashed
        && relation.appearance.start == EndSymbol::None
        && relation.appearance.end == EndSymbol::Arrow;
    let realization = relation.appearance.dashed
        && relation.appearance.start == EndSymbol::None
        && relation.appearance.end == EndSymbol::Triangle;
    let valid = from.kind == NodeKind::Component
        && match to.kind {
            NodeKind::Component => dependency,
            NodeKind::Interface => dependency || realization,
            _ => false,
        };
    if !valid
        || relation.from != from.id
        || relation.to != to.id
        || relation.source_end != Default::default()
        || relation.target_end != Default::default()
        || relation.binding.is_some()
    {
        return Err(error(
            relation.line,
            "component overview requires Component ..> Component/Interface or Component ..|> Interface; port wiring requires explicit assembly/delegation notation",
        ));
    }
    Ok(())
}
