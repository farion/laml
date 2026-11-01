//! Named graph ports and explicit UML component interface contracts.
use super::{ParseError, attributes, classes, error};
use crate::lex::valid_reference;
use crate::syntax::{DiagramType, NodeDecl, NodeKind, PortContract, PortDecl};

pub(super) fn parse(
    content: &str,
    line: usize,
    node: &mut NodeDecl,
    diagram_type: DiagramType,
    id: &Option<String>,
    member_classes: &[String],
) -> Result<bool, ParseError> {
    let Some(port) = content.strip_prefix("port ") else {
        return Ok(false);
    };
    if !matches!(
        node.kind,
        NodeKind::Generic | NodeKind::Class | NodeKind::Component | NodeKind::CompositeStructure
    ) {
        return Err(error(
            line,
            "ports are supported only on generic, class or component nodes",
        ));
    }
    let (name, attrs) = attributes(port.trim(), line)?;
    if name.is_empty() {
        return Err(error(line, "port name required"));
    }
    let typed = node.kind == NodeKind::Component || diagram_type == DiagramType::Composite;
    let mut port_classes = member_classes.to_vec();
    let mut contract = None;
    for (key, value) in attrs {
        match key.as_str() {
            "style-classes" => port_classes = classes(&value, line)?,
            "provides" | "requires" if typed => {
                if contract.is_some() || !valid_reference(value.trim()) {
                    return Err(error(
                        line,
                        "port needs exactly one valid interface reference",
                    ));
                }
                contract = Some(if key == "provides" {
                    PortContract::Provides(value.trim().into())
                } else {
                    PortContract::Requires(value.trim().into())
                });
            }
            _ => return Err(error(line, format!("unsupported port attribute '{key}'"))),
        }
    }
    if typed && contract.is_none() {
        return Err(error(
            line,
            "typed port needs provides=Interface or requires=Interface",
        ));
    }
    let port_id = id.clone().unwrap_or_else(|| name.clone());
    if node
        .ports
        .iter()
        .any(|port| port.id == port_id || port.label == name)
    {
        return Err(error(line, "duplicate port name or ID on one node"));
    }
    node.ports.push(PortDecl {
        id: port_id,
        label: name,
        explicit: id.is_some(),
        classes: port_classes,
        contract,
        activity_pin: None,
        line,
    });
    Ok(true)
}
