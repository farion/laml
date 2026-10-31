//! Component connectors only between explicitly owned, typed simple ports.
use super::{ParseError, error};
use crate::syntax::{ConnectorKind, NodeDecl, NodeKind, PortContract, RelationDecl};

pub(super) fn validate(
    relation: &RelationDecl,
    from: &NodeDecl,
    to: &NodeDecl,
    connector: ConnectorKind,
) -> Result<(), ParseError> {
    let fail = |reason| error(relation.line, reason);
    if relation.label.is_some()
        || !relation.stereotypes.is_empty()
        || relation.binding.is_some()
        || relation.source_end != Default::default()
        || relation.target_end != Default::default()
        || relation.from == relation.to
    {
        return Err(fail(
            "assembly/delegation connector cannot have labels or end adornments",
        ));
    }
    let first = from
        .ports
        .iter()
        .find(|port| from.matches_port(port, &relation.from))
        .and_then(|port| port.contract.as_ref());
    let second = to
        .ports
        .iter()
        .find(|port| to.matches_port(port, &relation.to))
        .and_then(|port| port.contract.as_ref());
    match (connector, first, second) {
        (ConnectorKind::Assembly, Some(a), Some(b))
            if from.kind == NodeKind::ComponentPart
                && to.kind == NodeKind::ComponentPart
                && from.id != to.id
                && from.parent.is_some()
                && from.parent == to.parent
                && matches!((a,b),
                    (PortContract::Provides(x), PortContract::Requires(y))
                    | (PortContract::Requires(x), PortContract::Provides(y)) if x == y) =>
        {
            Ok(())
        }
        (ConnectorKind::Delegation, Some(a), Some(b))
            if from.kind == NodeKind::Component
                && to.kind == NodeKind::ComponentPart
                && to.parent.as_deref() == Some(from.id.as_str())
                && matches!((a,b),
                    (PortContract::Provides(x), PortContract::Provides(y))
                    | (PortContract::Requires(x), PortContract::Requires(y)) if x == y) =>
        {
            Ok(())
        }
        _ => Err(fail(
            "assembly needs matching provided/required ports on sibling parts; delegation needs matching same-role ports from containing component to its own part",
        )),
    }
}
