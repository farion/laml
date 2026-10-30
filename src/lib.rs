//! LAML compiler stages: parsing, styles, geometry, routing, and export.
//!
//! A relation can contribute a *preferred* direction to automatic placement.
//! Preferences are processed in source order. A later directed relation that
//! would close a cycle loses only its placement preference; it remains a
//! semantic relation for routing. Hard layout rules are separate and
//! must never be discarded this way.

pub mod export;
pub mod geometry;
pub mod layout;
mod lex;
pub mod parser;
pub mod route;
pub mod sequence;
pub mod styles;
pub mod svg;
mod svg_paths;
pub mod syntax;
pub mod text;
pub mod update;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationKind {
    Directed,
    Bidirectional,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Relation {
    /// Source node for a directed relation, or the first declared endpoint.
    pub from: String,
    /// Target node for a directed relation, or the second declared endpoint.
    pub to: String,
    pub kind: RelationKind,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirectionPreferences {
    /// Indices into the input relations, in declaration order.
    pub accepted: Vec<usize>,
    /// Directed relations still exist, but do not influence placement direction.
    pub lost: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelError {
    DuplicateNode(String),
    UnknownNode { relation: usize, id: String },
}

/// Choose acyclic automatic direction preferences in declaration order.
///
/// Bidirectional relations impose no direction; their intended proximity
/// effect is handled in a later phase and is not implemented here. This
/// function does not compute coordinates, resolve hard layout constraints,
/// or route lines.
pub fn resolve_direction_preferences(
    nodes: &[Node],
    relations: &[Relation],
) -> Result<DirectionPreferences, ModelError> {
    for (i, node) in nodes.iter().enumerate() {
        if nodes[..i].iter().any(|previous| previous.id == node.id) {
            return Err(ModelError::DuplicateNode(node.id.clone()));
        }
    }

    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    let mut result = DirectionPreferences::default();

    for (index, relation) in relations.iter().enumerate() {
        let from = nodes
            .iter()
            .position(|node| node.id == relation.from)
            .ok_or_else(|| ModelError::UnknownNode {
                relation: index,
                id: relation.from.clone(),
            })?;
        let to = nodes
            .iter()
            .position(|node| node.id == relation.to)
            .ok_or_else(|| ModelError::UnknownNode {
                relation: index,
                id: relation.to.clone(),
            })?;

        if relation.kind == RelationKind::Bidirectional {
            continue;
        }
        if from == to || has_path(&adjacency, to, from) {
            result.lost.push(index);
        } else {
            adjacency[from].push(to);
            result.accepted.push(index);
        }
    }

    Ok(result)
}

fn has_path(adjacency: &[Vec<usize>], start: usize, target: usize) -> bool {
    let mut visited = vec![false; adjacency.len()];
    let mut stack = vec![start];
    while let Some(node) = stack.pop() {
        if node == target {
            return true;
        }
        if !visited[node] {
            visited[node] = true;
            stack.extend(adjacency[node].iter().copied());
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodes(names: &[&str]) -> Vec<Node> {
        names.iter().map(|id| Node { id: (*id).into() }).collect()
    }

    fn directed(from: &str, to: &str) -> Relation {
        Relation {
            from: from.into(),
            to: to.into(),
            kind: RelationKind::Directed,
        }
    }

    #[test]
    fn last_relation_loses_automatic_direction_in_cycle() {
        let relations = [directed("A", "B"), directed("B", "C"), directed("C", "A")];
        let result = resolve_direction_preferences(&nodes(&["A", "B", "C"]), &relations).unwrap();
        assert_eq!(result.accepted, [0, 1]);
        assert_eq!(result.lost, [2]);
    }

    #[test]
    fn changing_declaration_order_changes_which_relation_loses() {
        let relations = [directed("C", "A"), directed("A", "B"), directed("B", "C")];
        let result = resolve_direction_preferences(&nodes(&["A", "B", "C"]), &relations).unwrap();
        assert_eq!(result.accepted, [0, 1]);
        assert_eq!(result.lost, [2]);
    }

    #[test]
    fn bidirectional_relation_does_not_impose_direction() {
        let relations = [
            Relation {
                from: "B".into(),
                to: "A".into(),
                kind: RelationKind::Bidirectional,
            },
            directed("A", "B"),
        ];
        let result = resolve_direction_preferences(&nodes(&["A", "B"]), &relations).unwrap();
        assert_eq!(result.accepted, [1]);
        assert!(result.lost.is_empty());
    }

    #[test]
    fn unknown_node_is_an_error() {
        let error =
            resolve_direction_preferences(&nodes(&["A"]), &[directed("A", "Missing")]).unwrap_err();
        assert_eq!(
            error,
            ModelError::UnknownNode {
                relation: 0,
                id: "Missing".into()
            }
        );
    }

    #[test]
    fn self_relation_is_retained_semantically_but_not_as_direction_preference() {
        let result = resolve_direction_preferences(&nodes(&["A"]), &[directed("A", "A")]).unwrap();
        assert_eq!(result.lost, [0]);
    }
}
