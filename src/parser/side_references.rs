//! Side constraints for direct nodes, named boundary ports and member rows.
use super::{ParseError, error};
use crate::syntax::{Diagram, EndPosition, PortSide, RelationDecl, RelationEndRule};

fn endpoint(relation: &RelationDecl, end: EndPosition) -> &str {
    match end {
        EndPosition::Source => &relation.from,
        EndPosition::Target => &relation.to,
    }
}
pub(super) fn validate(
    diagram: &Diagram,
    rule: &RelationEndRule,
    relation: &RelationDecl,
    index: usize,
) -> Result<(), ParseError> {
    let Some(side) = rule.side else {
        return Ok(());
    };
    let address = endpoint(relation, rule.end);
    if diagram.nodes.iter().any(|node| node.id == address) {
        return Ok(());
    }
    let named_port = diagram.nodes.iter().any(|node| {
        node.ports
            .iter()
            .any(|port| node.matches_port(port, address))
    });
    if named_port {
        if diagram
            .port_layout
            .iter()
            .any(|own| own.id == address && own.side != side)
        {
            return Err(error(
                rule.line,
                "relation side conflicts with mandatory named-port side",
            ));
        }
        for prior in &diagram.relation_end_layout[..index] {
            let Some(prior_side) = prior.side else {
                continue;
            };
            let Some(other) = diagram
                .relations
                .iter()
                .find(|relation| relation.name.as_deref() == Some(prior.name.as_str()))
            else {
                continue;
            };
            if endpoint(other, prior.end) == address && prior_side != side {
                return Err(error(
                    rule.line,
                    "conflicting relation sides on one named port",
                ));
            }
        }
    } else if !matches!(side, PortSide::Left | PortSide::Right) {
        return Err(error(
            rule.line,
            "member-row relation side must be left or right",
        ));
    }
    Ok(())
}
