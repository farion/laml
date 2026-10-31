//! State-machine image notation: vertices, textual behavior and transition validity.
use super::{ParseError, error};
use crate::syntax::{
    Diagram, DiagramType, EndSymbol, MemberKind, NodeDecl, NodeKind, NodePart, RelationDecl,
};

pub(super) fn vertex(kind: NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::State | NodeKind::StateInitial | NodeKind::StateFinal | NodeKind::StateChoice
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
    if diagram.settings.diagram_type != DiagramType::State {
        return Ok(());
    }
    if visibility.is_some() || !stereotypes.is_empty() {
        return Err(error(
            line,
            "state vertices cannot show classifier visibility or unchecked stereotypes",
        ));
    }
    if block
        && matches!(
            kind,
            NodeKind::StateInitial | NodeKind::StateFinal | NodeKind::StateChoice
        )
    {
        return Err(error(
            line,
            "state pseudostates cannot contain declarations",
        ));
    }
    if let Some(id) = parent
        && !diagram
            .nodes
            .iter()
            .any(|node| node.id == id && node.kind == NodeKind::State)
    {
        return Err(error(
            line,
            "only a composite state can contain state-machine vertices",
        ));
    }
    Ok(())
}

pub(super) fn behavior(text: &str, line: usize, node: &mut NodeDecl) -> Result<(), ParseError> {
    let (kind, action) = text
        .split_once('/')
        .ok_or_else(|| error(line, "state body requires entry /, exit / or do / behavior"))?;
    let kind = kind.trim();
    if !matches!(kind, "entry" | "exit" | "do")
        || action.trim().is_empty()
        || action.trim().contains('\n')
    {
        return Err(error(
            line,
            "state body requires entry /, exit / or do / behavior",
        ));
    }
    if node.parts.iter().any(|part| matches!(part, NodePart::Member { text, .. } if text.starts_with(&format!("{kind} /")))) {
        return Err(error(line, format!("duplicate state {kind} behavior")));
    }
    node.parts.push(NodePart::Member {
        id: None,
        default_id: None,
        classes: Vec::new(),
        text: format!("{kind} / {}", action.trim()),
        kind: MemberKind::Text,
        abstract_member: false,
        static_member: false,
        line,
    });
    Ok(())
}

fn parts(label: &str, line: usize) -> Result<(bool, Option<&str>, bool), ParseError> {
    let (head, effect) = match label.split_once('/') {
        Some((head, effect)) if !effect.trim().is_empty() => (head.trim(), true),
        Some(_) => {
            return Err(error(
                line,
                "state transition effect after '/' cannot be empty",
            ));
        }
        None => (label.trim(), false),
    };
    let (trigger, guard) = if let Some((trigger, guard)) = head.split_once('[') {
        let Some(guard) = guard.strip_suffix(']') else {
            return Err(error(line, "state transition guard needs a closing ']'"));
        };
        if guard.trim().is_empty() || guard.contains(['[', ']']) {
            return Err(error(
                line,
                "state transition guard must contain one nonempty expression",
            ));
        }
        (trigger.trim(), Some(guard.trim()))
    } else {
        if head.contains(']') {
            return Err(error(line, "state transition has a stray ']'"));
        }
        (head, None)
    };
    if trigger.is_empty() && guard.is_none() && !effect {
        return Err(error(
            line,
            "state transition needs a trigger, guard or effect",
        ));
    }
    Ok((!trigger.is_empty(), guard, effect))
}

pub(super) fn relation(
    relation: &RelationDecl,
    from: &NodeDecl,
    to: &NodeDecl,
) -> Result<(), ParseError> {
    if !vertex(from.kind)
        || !vertex(to.kind)
        || relation.from != from.id
        || relation.to != to.id
        || from.parent != to.parent
        || relation.appearance.dashed
        || relation.appearance.start != EndSymbol::None
        || relation.appearance.end != EndSymbol::Arrow
        || relation.connector.is_some()
        || relation.binding.is_some()
        || !relation.stereotypes.is_empty()
        || relation.source_end != Default::default()
        || relation.target_end != Default::default()
    {
        return Err(error(
            relation.line,
            "state transition requires State --> State/pseudostate within one region; cross-boundary edges and non-UML markers are unsupported",
        ));
    }
    if let Some(label) = &relation.label {
        let (trigger, guard, _) = parts(label, relation.line)?;
        if from.kind == NodeKind::StateInitial
            || (from.kind == NodeKind::StateChoice && (trigger || guard.is_none()))
        {
            return Err(error(
                relation.line,
                "initial transitions must be unlabeled; choice exits require a guard without a trigger",
            ));
        }
    } else if from.kind == NodeKind::StateChoice {
        return Err(error(relation.line, "choice exits require a guard"));
    }
    Ok(())
}

pub(super) fn validate(diagram: &Diagram) -> Result<(), ParseError> {
    if diagram.settings.diagram_type != DiagramType::State {
        return Ok(());
    }
    let mut initials = std::collections::HashSet::new();
    for node in &diagram.nodes {
        let incoming: Vec<_> = diagram
            .relations
            .iter()
            .filter(|rel| rel.to == node.id && !rel.appearance.dashed)
            .collect();
        let outgoing: Vec<_> = diagram
            .relations
            .iter()
            .filter(|rel| rel.from == node.id && !rel.appearance.dashed)
            .collect();
        match node.kind {
            NodeKind::StateInitial => {
                if !initials.insert(node.parent.as_deref())
                    || !incoming.is_empty()
                    || outgoing.len() != 1
                {
                    return Err(error(
                        node.line,
                        "one initial per region must have exactly one outgoing and no incoming transition",
                    ));
                }
            }
            NodeKind::StateFinal if !outgoing.is_empty() => {
                return Err(error(
                    node.line,
                    "final state cannot have outgoing transitions",
                ));
            }
            NodeKind::StateChoice => {
                if incoming.is_empty() || outgoing.len() < 2 {
                    return Err(error(
                        node.line,
                        "choice requires an incoming transition and at least two guarded exits",
                    ));
                }
                let else_count = outgoing
                    .iter()
                    .filter(|rel| {
                        rel.label.as_deref().is_some_and(|label| {
                            parts(label, rel.line)
                                .ok()
                                .is_some_and(|(_, guard, _)| guard == Some("else"))
                        })
                    })
                    .count();
                if else_count > 1 {
                    return Err(error(node.line, "choice can have only one [else] guard"));
                }
            }
            _ => {}
        }
    }
    Ok(())
}
