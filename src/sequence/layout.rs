//! Mandatory symbolic order and separation of sequence lifeline headers.

use super::{LaneRule, SequenceDiagram, error};
use crate::lex::{split_once_top_level, valid_identifier};
use crate::parser::ParseError;

pub(super) fn parse_rule(text: &str, line: usize) -> Result<LaneRule, ParseError> {
    let (placement, attrs) = match split_once_top_level(text, ':') {
        Some((placement, attrs)) => (placement.trim(), Some(attrs.trim())),
        None => (text, None),
    };
    let words: Vec<_> = placement.split_whitespace().collect();
    let (left, right) = match words.as_slice() {
        [a, "<", b] | [a, "left", "of", b] => (*a, *b),
        [a, ">", b] | [a, "right", "of", b] => (*b, *a),
        _ => return Err(error(line, "expected horizontal placement such as A < B")),
    };
    let valid = |id: &str| {
        valid_identifier(id)
            || id
                .split_once('.')
                .is_some_and(|(owner, child)| valid_identifier(owner) && valid_identifier(child))
    };
    if !valid(left) || !valid(right) || left == right {
        return Err(error(line, "invalid or identical lifelines in placement"));
    }
    let gap = if let Some(attrs) = attrs {
        let (key, value) =
            split_once_top_level(attrs, '=').ok_or_else(|| error(line, "expected gap=number"))?;
        if key.trim() != "gap" {
            return Err(error(line, "only gap is supported in sequence layout"));
        }
        let gap: f64 = value
            .trim()
            .parse()
            .map_err(|_| error(line, "gap must be a number"))?;
        if !gap.is_finite() || gap < 0.0 {
            return Err(error(line, "gap must be finite and nonnegative"));
        }
        gap
    } else {
        24.0
    };
    Ok(LaneRule {
        left: left.into(),
        right: right.into(),
        gap,
        line,
    })
}

/// Route qualified child constraints to their inline subdiagram before solving.
pub(super) fn route_decompositions(diagram: &mut SequenceDiagram) -> Result<(), ParseError> {
    let mut root = Vec::new();
    for rule in std::mem::take(&mut diagram.layout) {
        match (rule.left.split_once('.'), rule.right.split_once('.')) {
            (None, None) => root.push(rule),
            (Some((owner, left)), Some((other, right))) if owner == other => {
                let sub = diagram
                    .decompositions
                    .iter_mut()
                    .find(|d| d.parent == owner)
                    .ok_or_else(|| error(rule.line, format!("unknown decomposition '{owner}'")))?;
                sub.diagram.layout.push(LaneRule {
                    left: left.into(),
                    right: right.into(),
                    ..rule
                });
            }
            _ => {
                return Err(error(
                    rule.line,
                    "layout cannot compare lifelines from different decomposition scopes",
                ));
            }
        }
    }
    diagram.layout = root;
    for sub in &mut diagram.decompositions {
        validate_and_order(&mut sub.diagram)?;
    }
    Ok(())
}

/// Stable topological order: unspecified ties retain source declaration order.
pub(super) fn validate_and_order(diagram: &mut SequenceDiagram) -> Result<(), ParseError> {
    let count = diagram.participants.len();
    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); count];
    let mut indegree = vec![0usize; count];
    for rule in &diagram.layout {
        let from = diagram
            .participants
            .iter()
            .position(|p| p.id == rule.left)
            .ok_or_else(|| error(rule.line, format!("unknown participant '{}'", rule.left)))?;
        let to = diagram
            .participants
            .iter()
            .position(|p| p.id == rule.right)
            .ok_or_else(|| error(rule.line, format!("unknown participant '{}'", rule.right)))?;
        if !edges[from].contains(&to) {
            edges[from].push(to);
            indegree[to] += 1;
        }
    }
    let mut used = vec![false; count];
    let mut ordered = Vec::with_capacity(count);
    for _ in 0..count {
        let next = (0..count)
            .find(|&i| !used[i] && indegree[i] == 0)
            .ok_or_else(|| {
                error(
                    diagram.layout.last().map_or(1, |r| r.line),
                    "contradictory lifeline order",
                )
            })?;
        used[next] = true;
        ordered.push(diagram.participants[next].clone());
        for &to in &edges[next] {
            indegree[to] -= 1;
        }
    }
    diagram.participants = ordered;
    Ok(())
}
