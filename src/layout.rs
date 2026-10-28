//! Sound but intentionally incomplete hard-layout validation.
//! A directional cycle is impossible for positive-area node boxes.
//! Alignment, measured dimensions, and routing need further validation.

use crate::syntax::{Diagram, LayoutRule, Placement};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutError {
    pub line: usize,
    pub message: String,
}

/// Reject directional cycles on either axis, naming the rule that closes one.
/// A passing result does not yet prove that all layout constraints are feasible.
pub fn validate_ordering(diagram: &Diagram) -> Result<(), LayoutError> {
    let count = diagram.nodes.len();
    let mut x = vec![Vec::new(); count];
    let mut y = vec![Vec::new(); count];
    for rule in &diagram.layout {
        let first = diagram
            .nodes
            .iter()
            .position(|n| n.id == rule.first)
            .expect("parsed diagram has validated node references");
        let second = diagram
            .nodes
            .iter()
            .position(|n| n.id == rule.second)
            .expect("parsed diagram has validated node references");
        let (before, after, graph, axis) = match rule.placement {
            Placement::Left => (first, second, &mut x, "horizontal"),
            Placement::Right => (second, first, &mut x, "horizontal"),
            Placement::Above => (first, second, &mut y, "vertical"),
            Placement::Below => (second, first, &mut y, "vertical"),
        };
        if before == after || reachable(graph, after, before) {
            return Err(LayoutError {
                line: rule.line,
                message: format!(
                    "impossible {axis} placement: rule '{}' conflicts with earlier mandatory placement rules",
                    describe(rule)
                ),
            });
        }
        graph[before].push(after);
    }
    Ok(())
}

fn describe(rule: &LayoutRule) -> String {
    let direction = match rule.placement {
        Placement::Left => "left of",
        Placement::Right => "right of",
        Placement::Above => "above",
        Placement::Below => "below",
    };
    format!("{} {direction} {}", rule.first, rule.second)
}

fn reachable(graph: &[Vec<usize>], start: usize, target: usize) -> bool {
    let mut visited = vec![false; graph.len()];
    let mut stack = vec![start];
    while let Some(next) = stack.pop() {
        if next == target {
            return true;
        }
        if !visited[next] {
            visited[next] = true;
            stack.extend(graph[next].iter().copied());
        }
    }
    false
}
