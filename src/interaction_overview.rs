//! Checked UML interaction-overview notation: ref frames and structured control flow.
mod render;
pub use render::render;

use crate::lex::{valid_identifier, without_comment};
use crate::parser::{self, ParseError};
use crate::syntax::{Diagram, DiagramType, EndSymbol, NodeDecl, NodeKind, RelationDecl};

#[derive(Clone, Debug)]
pub struct Document {
    pub graph: Diagram,
    pub interaction: String,
}
fn error(line: usize, message: impl Into<String>) -> ParseError {
    ParseError {
        line,
        message: message.into(),
    }
}

pub fn is_overview_source(source: &str) -> bool {
    let mut settings = false;
    for raw in source.lines() {
        let text = without_comment(raw).trim();
        if text == "settings {" {
            settings = true;
        } else if settings && text == "}" {
            settings = false;
        } else if settings && text == "type: interaction-overview" {
            return true;
        }
    }
    false
}

pub fn parse(source: &str) -> Result<Document, ParseError> {
    let mut stripped = String::new();
    let mut settings = false;
    let mut interaction = None;
    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        let text = without_comment(raw).trim();
        if text == "settings {" {
            settings = true;
        } else if settings && text == "}" {
            settings = false;
        } else if settings && text.starts_with("interaction:") {
            let name = text.trim_start_matches("interaction:").trim();
            if !valid_identifier(name) || interaction.is_some() {
                return Err(error(
                    line,
                    "interaction: Name requires one valid interaction name",
                ));
            }
            interaction = Some(name.to_owned());
            stripped.push('\n');
            continue;
        }
        stripped.push_str(raw);
        stripped.push('\n');
    }
    let interaction =
        interaction.ok_or_else(|| error(1, "interaction-overview requires interaction: Name"))?;
    let graph = parser::parse_interaction_graph(&stripped)?;
    if graph.settings.diagram_type != DiagramType::InteractionOverview {
        return Err(error(1, "expected type: interaction-overview"));
    }
    validate_topology(&graph)?;
    Ok(Document { graph, interaction })
}

pub fn validate_node(
    diagram: &Diagram,
    kind: NodeKind,
    parent: Option<&str>,
    block: bool,
    visibility: Option<char>,
    stereotypes: &[String],
    line: usize,
) -> Result<(), ParseError> {
    if diagram.settings.diagram_type != DiagramType::InteractionOverview {
        return Ok(());
    }
    if parent.is_some()
        || block
        || visibility.is_some()
        || !stereotypes.is_empty()
        || !matches!(
            kind,
            NodeKind::InteractionUse
                | NodeKind::ActivityInitial
                | NodeKind::ActivityFinal
                | NodeKind::ActivityDecision
                | NodeKind::ActivityMerge
                | NodeKind::ActivityFork
                | NodeKind::ActivityJoin
        )
    {
        return Err(error(
            line,
            "interaction overview accepts only flat ref uses and control vertices without bodies or classifier modifiers",
        ));
    }
    Ok(())
}

pub fn validate_relation(
    rel: &RelationDecl,
    from: &NodeDecl,
    to: &NodeDecl,
) -> Result<(), ParseError> {
    if rel.from != from.id
        || rel.to != to.id
        || rel.from == rel.to
        || rel.appearance.start != EndSymbol::None
        || rel.appearance.end != EndSymbol::Arrow
        || rel.appearance.dashed
        || rel.binding.is_some()
        || rel.connector.is_some()
        || !rel.stereotypes.is_empty()
        || rel.source_end != Default::default()
        || rel.target_end != Default::default()
        || from.kind == NodeKind::ActivityFinal
        || to.kind == NodeKind::ActivityInitial
    {
        return Err(error(
            rel.line,
            "interaction overview requires plain directed control flow between distinct vertices",
        ));
    }
    if from.kind == NodeKind::ActivityDecision {
        let valid = rel
            .label
            .as_deref()
            .and_then(|s| s.strip_prefix('['))
            .and_then(|s| s.strip_suffix(']'))
            .is_some_and(|guard| !guard.trim().is_empty() && !guard.contains(['[', ']']));
        if !valid {
            return Err(error(
                rel.line,
                "decision exits require a visible [guard] or [else]",
            ));
        }
    } else if rel.label.is_some() {
        return Err(error(
            rel.line,
            "only decision exits may carry [guard] labels",
        ));
    }
    Ok(())
}

fn validate_topology(graph: &Diagram) -> Result<(), ParseError> {
    let nodes = &graph.nodes;
    let initial: Vec<_> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == NodeKind::ActivityInitial)
        .map(|(i, _)| i)
        .collect();
    let final_nodes: Vec<_> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == NodeKind::ActivityFinal)
        .map(|(i, _)| i)
        .collect();
    if initial.len() != 1
        || final_nodes.len() != 1
        || !nodes.iter().any(|n| n.kind == NodeKind::InteractionUse)
    {
        return Err(error(
            1,
            "interaction overview needs exactly one initial, one final and at least one interaction-use",
        ));
    }
    let mut incoming = vec![Vec::new(); nodes.len()];
    let mut outgoing = vec![Vec::new(); nodes.len()];
    for rel in &graph.relations {
        let from = nodes
            .iter()
            .position(|n| n.id == rel.from)
            .ok_or_else(|| error(rel.line, "unknown flow source"))?;
        let to = nodes
            .iter()
            .position(|n| n.id == rel.to)
            .ok_or_else(|| error(rel.line, "unknown flow target"))?;
        if outgoing[from].contains(&to) {
            return Err(error(rel.line, "duplicate overview control flow"));
        }
        outgoing[from].push(to);
        incoming[to].push(from);
    }
    for (i, node) in nodes.iter().enumerate() {
        let (ins, outs) = (incoming[i].len(), outgoing[i].len());
        let valid = match node.kind {
            NodeKind::ActivityInitial => ins == 0 && outs == 1,
            NodeKind::ActivityFinal => ins == 1 && outs == 0,
            NodeKind::ActivityDecision | NodeKind::ActivityFork => ins == 1 && outs >= 2,
            NodeKind::ActivityMerge | NodeKind::ActivityJoin => ins >= 2 && outs == 1,
            NodeKind::InteractionUse => ins == 1 && outs == 1,
            _ => false,
        };
        if !valid {
            return Err(error(
                node.line,
                "invalid interaction-overview control vertex topology",
            ));
        }
        if node.kind == NodeKind::ActivityDecision {
            let else_count = graph
                .relations
                .iter()
                .filter(|r| r.from == node.id && r.label.as_deref() == Some("[else]"))
                .count();
            if else_count > 1 {
                return Err(error(node.line, "only one [else] branch is allowed"));
            }
        }
    }
    // Topological propagation of split stacks. Merges must consume the same most-recent
    // split on every incoming path; crossing, partial and unrelated joins are rejected.
    let mut degrees: Vec<usize> = incoming.iter().map(Vec::len).collect();
    let mut pending = std::collections::VecDeque::from([initial[0]]);
    let mut stacks: Vec<Vec<Vec<usize>>> = vec![Vec::new(); nodes.len()];
    let mut count = 0;
    while let Some(i) = pending.pop_front() {
        count += 1;
        let mut stack = if i == initial[0] {
            Vec::new()
        } else {
            let paths = &stacks[i];
            if paths.len() != incoming[i].len() || paths.iter().any(|p| p != &paths[0]) {
                return Err(error(
                    nodes[i].line,
                    "interaction-overview branches must be properly nested",
                ));
            }
            paths[0].clone()
        };
        match nodes[i].kind {
            NodeKind::ActivityDecision | NodeKind::ActivityFork => stack.push(i),
            NodeKind::ActivityMerge | NodeKind::ActivityJoin => {
                let Some(split) = stack.pop() else {
                    return Err(error(nodes[i].line, "join without a matching split"));
                };
                let expected = if nodes[i].kind == NodeKind::ActivityMerge {
                    NodeKind::ActivityDecision
                } else {
                    NodeKind::ActivityFork
                };
                if nodes[split].kind != expected || outgoing[split].len() != incoming[i].len() {
                    return Err(error(
                        nodes[i].line,
                        "merge/join must match all branches of its decision/fork",
                    ));
                }
            }
            NodeKind::ActivityFinal if !stack.is_empty() => {
                return Err(error(
                    nodes[i].line,
                    "branch reaches final without its merge/join",
                ));
            }
            _ => {}
        }
        for &to in &outgoing[i] {
            stacks[to].push(stack.clone());
            degrees[to] -= 1;
            if degrees[to] == 0 {
                pending.push_back(to);
            }
        }
    }
    if count != nodes.len() {
        return Err(error(
            1,
            "interaction-overview control flow must be connected and acyclic",
        ));
    }
    Ok(())
}
