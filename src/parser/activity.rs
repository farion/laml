//! Checked UML activity image notation; no token or guard execution.
use super::{ParseError, error};
use crate::lex::valid_identifier;
use crate::syntax::{
    ActivityFlow, ActivityPinDirection, Diagram, DiagramType, EndSymbol, NodeDecl, NodeKind,
    PortDecl, PortSide, RelationDecl,
};

pub(super) fn validate_node(
    diagram: &Diagram,
    kind: NodeKind,
    parent: Option<&str>,
    block: bool,
    visibility: Option<char>,
    stereotypes: &[String],
    line: usize,
) -> Result<(), ParseError> {
    if diagram.settings.diagram_type != DiagramType::Activity {
        return Ok(());
    }
    if parent.is_some() || visibility.is_some() || !stereotypes.is_empty() {
        return Err(error(
            line,
            "activity nodes cannot be nested or carry classifier modifiers",
        ));
    }
    if block && !matches!(kind, NodeKind::ActivityAction | NodeKind::Note) {
        return Err(error(
            line,
            "only actions and notes can have a body in an activity diagram",
        ));
    }
    Ok(())
}

pub(super) fn pin(text: &str, line: usize, action: &mut NodeDecl) -> Result<(), ParseError> {
    let (direction, name) = if let Some(name) = text.strip_prefix("input ") {
        (ActivityPinDirection::Input, name.trim())
    } else if let Some(name) = text.strip_prefix("output ") {
        (ActivityPinDirection::Output, name.trim())
    } else {
        return Err(error(
            line,
            "action body accepts only input Name or output Name pins",
        ));
    };
    if !valid_identifier(name) || action.ports.iter().any(|port| port.id == name) {
        return Err(error(line, "activity pin requires a unique simple name"));
    }
    action.ports.push(PortDecl {
        id: name.into(),
        label: name.into(),
        explicit: false,
        classes: Vec::new(),
        contract: None,
        activity_pin: Some(direction),
        line,
    });
    Ok(())
}

fn control_vertex(kind: NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::ActivityAction
            | NodeKind::ActivityInitial
            | NodeKind::ActivityFinal
            | NodeKind::ActivityFlowFinal
            | NodeKind::ActivityDecision
            | NodeKind::ActivityMerge
            | NodeKind::ActivityFork
            | NodeKind::ActivityJoin
    )
}

fn guard(label: &str) -> Option<&str> {
    let text = label.strip_prefix('[')?.strip_suffix(']')?.trim();
    if text.is_empty() || text.contains(['[', ']']) {
        None
    } else {
        Some(text)
    }
}

pub(super) fn relation(
    relation: &RelationDecl,
    from: &NodeDecl,
    to: &NodeDecl,
) -> Result<(), ParseError> {
    if relation.appearance.start != EndSymbol::None
        || relation.appearance.end != EndSymbol::Arrow
        || relation.appearance.dashed
        || relation.connector.is_some()
        || relation.binding.is_some()
        || !relation.stereotypes.is_empty()
        || relation.source_end != Default::default()
        || relation.target_end != Default::default()
    {
        return Err(error(
            relation.line,
            "activity flows need an unadorned solid directed arrow",
        ));
    }
    match relation.activity_flow {
        Some(ActivityFlow::Control) => {
            if from.id != relation.from
                || to.id != relation.to
                || !control_vertex(from.kind)
                || !control_vertex(to.kind)
                || matches!(
                    from.kind,
                    NodeKind::ActivityFinal | NodeKind::ActivityFlowFinal
                )
                || to.kind == NodeKind::ActivityInitial
            {
                return Err(error(
                    relation.line,
                    "control flow requires control vertices, not pins or object nodes",
                ));
            }
            match (&relation.label, from.kind) {
                (Some(text), NodeKind::ActivityDecision) if guard(text).is_some() => {}
                (None, NodeKind::ActivityDecision) => {
                    return Err(error(relation.line, "decision exits need [guard]"));
                }
                (Some(_), NodeKind::ActivityDecision) => {
                    return Err(error(
                        relation.line,
                        "decision exit requires one [guard] or [else]",
                    ));
                }
                (Some(_), _) => {
                    return Err(error(
                        relation.line,
                        "only decision exits carry guards in this activity subset",
                    ));
                }
                _ => {}
            }
        }
        Some(ActivityFlow::Object) => {
            if relation.label.is_some() {
                return Err(error(
                    relation.line,
                    "object flow labels are not supported; use named object nodes and pins",
                ));
            }
            let output = from.kind == NodeKind::ActivityObject && from.id == relation.from
                || from.kind == NodeKind::ActivityAction
                    && from.ports.iter().any(|pin| {
                        pin.activity_pin == Some(ActivityPinDirection::Output)
                            && from.matches_port(pin, &relation.from)
                    });
            let input = to.kind == NodeKind::ActivityObject && to.id == relation.to
                || to.kind == NodeKind::ActivityAction
                    && to.ports.iter().any(|pin| {
                        pin.activity_pin == Some(ActivityPinDirection::Input)
                            && to.matches_port(pin, &relation.to)
                    });
            if !output || !input {
                return Err(error(
                    relation.line,
                    "object flow requires an output pin or object node as source and an input pin or object node as target",
                ));
            }
        }
        None => {
            return Err(error(
                relation.line,
                "activity arrow needs explicit control or object flow kind",
            ));
        }
    }
    Ok(())
}

pub(super) fn validate(diagram: &Diagram) -> Result<(), ParseError> {
    if diagram.settings.diagram_type != DiagramType::Activity {
        return Ok(());
    }
    let mut initials = 0;
    for node in &diagram.nodes {
        let incoming: Vec<_> = diagram
            .relations
            .iter()
            .filter(|rel| rel.activity_flow == Some(ActivityFlow::Control) && rel.to == node.id)
            .collect();
        let outgoing: Vec<_> = diagram
            .relations
            .iter()
            .filter(|rel| rel.activity_flow == Some(ActivityFlow::Control) && rel.from == node.id)
            .collect();
        let bad = match node.kind {
            NodeKind::ActivityInitial => {
                initials += 1;
                !incoming.is_empty() || outgoing.len() != 1
            }
            NodeKind::ActivityFinal | NodeKind::ActivityFlowFinal => !outgoing.is_empty(),
            NodeKind::ActivityDecision => {
                incoming.len() != 1
                    || outgoing.len() < 2
                    || outgoing
                        .iter()
                        .filter(|rel| rel.label.as_deref().and_then(guard) == Some("else"))
                        .count()
                        > 1
            }
            NodeKind::ActivityMerge => incoming.len() < 2 || outgoing.len() != 1,
            NodeKind::ActivityFork => incoming.len() != 1 || outgoing.len() < 2,
            NodeKind::ActivityJoin => incoming.len() < 2 || outgoing.len() != 1,
            _ => false,
        };
        if bad {
            return Err(error(
                node.line,
                "invalid activity-node topology (initial, final, decision, merge, fork or join)",
            ));
        }
    }
    if initials > 1 {
        return Err(error(
            1,
            "activity diagram permits at most one initial node",
        ));
    }
    for rule in &diagram.port_layout {
        for node in &diagram.nodes {
            for pin in &node.ports {
                let id = if pin.explicit {
                    pin.id.clone()
                } else {
                    format!("{}::{}", node.id, pin.id)
                };
                if rule.id == id
                    && pin.activity_pin.is_some_and(|direction| {
                        rule.side
                            != match direction {
                                ActivityPinDirection::Input => PortSide::Left,
                                ActivityPinDirection::Output => PortSide::Right,
                            }
                    })
                {
                    return Err(error(
                        rule.line,
                        "activity input pins stay left and output pins stay right",
                    ));
                }
            }
        }
    }
    Ok(())
}
