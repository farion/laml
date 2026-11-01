//! Stable automatic ranks; annotations track the element they describe.
use super::super::{Document, EdgeKind, Kind};
pub(super) fn ranks(doc: &Document) -> Vec<usize> {
    let mut adjacency: Vec<Vec<usize>> = vec![vec![]; doc.nodes.len()];
    let mut accepted = Vec::new();
    // A directed cycle loses only the last automatic placement preference.
    for edge in doc.edges.iter().filter(|e| e.kind == EdgeKind::Sequence) {
        let mut a = doc.nodes.iter().position(|n| n.id == edge.from).unwrap();
        let b = doc.nodes.iter().position(|n| n.id == edge.to).unwrap();
        if doc.nodes[a].kind == Kind::Boundary
            && let Some(owner) = &doc.nodes[a].attached
        {
            a = doc.nodes.iter().position(|n| &n.id == owner).unwrap();
        }
        if a == b {
            continue;
        }
        let mut visited = vec![false; doc.nodes.len()];
        let mut stack = vec![b];
        while let Some(i) = stack.pop() {
            if visited[i] {
                continue;
            }
            visited[i] = true;
            stack.extend(adjacency[i].iter().copied());
        }
        if !visited[a] {
            adjacency[a].push(b);
            accepted.push((a, b));
        }
    }
    let mut result = vec![0usize; doc.nodes.len()];
    for _ in 0..doc.nodes.len() {
        let mut changed = false;
        for &(a, b) in &accepted {
            if result[b] <= result[a] {
                result[b] = result[a] + 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for edge in doc.edges.iter().filter(|e| e.kind == EdgeKind::Association) {
        let a = doc.nodes.iter().position(|n| n.id == edge.from).unwrap();
        let b = doc.nodes.iter().position(|n| n.id == edge.to).unwrap();
        if doc.nodes[a].kind == Kind::Annotation {
            result[a] = result[b];
        } else if doc.nodes[b].kind == Kind::Annotation {
            result[b] = result[a];
        }
    }
    result
}
