//! Center straight-through BPMN spines between their branches and joins.
use super::BoxAt;
use crate::bpmn::{Document, EdgeKind, Kind};
fn gateway(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Exclusive | Kind::Parallel | Kind::Inclusive | Kind::EventBased
    )
}
pub(super) fn spine_center(
    doc: &Document,
    indices: &[usize],
    ranks: &[usize],
    boxes: &[BoxAt],
) -> Option<f64> {
    let mut best: Option<(f64, f64)> = None;
    for (gateway_index, node) in doc.nodes.iter().enumerate() {
        if !gateway(node.kind) || !indices.contains(&gateway_index) {
            continue;
        }
        for outgoing in [true, false] {
            let mut connected: Vec<_> = doc
                .edges
                .iter()
                .filter(|e| {
                    e.kind == EdgeKind::Sequence
                        && if outgoing {
                            e.from == node.id
                        } else {
                            e.to == node.id
                        }
                })
                .filter_map(|e| {
                    doc.nodes.iter().position(|n| {
                        n.id.as_str()
                            == if outgoing {
                                e.to.as_str()
                            } else {
                                e.from.as_str()
                            }
                    })
                })
                .filter(|i| indices.contains(i))
                .collect();
            connected.sort_unstable();
            for &index in &connected {
                let row: Vec<_> = connected
                    .iter()
                    .filter(|&&other| ranks[other] == ranks[index])
                    .map(|&other| boxes[other].y)
                    .collect();
                if row.len() < 2 {
                    continue;
                }
                let low = row.iter().copied().fold(f64::INFINITY, f64::min);
                let high = row.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let span = high - low;
                if span > 1.0 && best.is_none_or(|(previous, _)| span > previous) {
                    best = Some((span, (low + high) / 2.0));
                }
            }
        }
    }
    best.map(|(_, center)| center)
}
/// Avoid a split-to-branch route cutting across another branch's path into
/// the same join. Reorder only free automatic rows within one semantic lane;
/// authored layout and docking rules keep precedence.
pub(super) fn uncross_merges(doc: &Document, indices: &[usize], boxes: &mut [BoxAt]) {
    for &join in indices {
        if !gateway(doc.nodes[join].kind) {
            continue;
        }
        let incoming: Vec<_> = doc
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Sequence && e.to == doc.nodes[join].id)
            .filter_map(|e| doc.nodes.iter().position(|n| n.id == e.from))
            .filter(|i| indices.contains(i))
            .collect();
        if incoming.len() < 3 {
            continue;
        }
        for &branch in &incoming {
            let Some(split) = doc
                .edges
                .iter()
                .filter(|e| e.kind == EdgeKind::Sequence && e.to == doc.nodes[branch].id)
                .filter_map(|e| doc.nodes.iter().position(|n| n.id == e.from))
                .find(|&i| {
                    gateway(doc.nodes[i].kind)
                        && !indices.contains(&i)
                        && doc
                            .edges
                            .iter()
                            .filter(|edge| {
                                edge.kind == EdgeKind::Sequence && edge.from == doc.nodes[i].id
                            })
                            .count()
                            >= 2
                })
            else {
                continue;
            };
            let axis = boxes[split].x;
            if axis < 1.0 || boxes[split].y >= boxes[branch].y || axis >= boxes[join].x {
                continue;
            }
            for &other in &incoming {
                if other == branch
                    || boxes[other].x >= axis
                    || boxes[other].y >= boxes[branch].y - 1.0
                    || doc.edges.iter().any(|e| {
                        e.kind == EdgeKind::Sequence
                            && e.from == doc.nodes[split].id
                            && e.to == doc.nodes[other].id
                    })
                {
                    continue;
                }
                let touches = |id: &str| {
                    doc.layout.iter().any(|(a, _, b)| {
                        id == a
                            || id.ends_with(&format!(".{a}"))
                            || id == b
                            || id.ends_with(&format!(".{b}"))
                    })
                };
                if touches(&doc.nodes[other].id)
                    || touches(&doc.nodes[branch].id)
                    || doc.flow_layout.iter().any(|r| {
                        r.to == doc.nodes[join].id
                            && (r.from == doc.nodes[other].id || r.from == doc.nodes[branch].id)
                            && r.to_side.is_some()
                    })
                {
                    continue;
                }
                let new_y = boxes[branch].y + (boxes[branch].h + boxes[other].h) / 2.0 + 45.0;
                let moved = BoxAt {
                    y: new_y,
                    ..boxes[other]
                };
                if indices.iter().any(|&j| {
                    j != other
                        && (boxes[j].x - moved.x).abs() < (boxes[j].w + moved.w) / 2.0 + 8.0
                        && (boxes[j].y - moved.y).abs() < (boxes[j].h + moved.h) / 2.0 + 8.0
                }) {
                    continue;
                }
                boxes[other] = moved;
                break;
            }
        }
    }
}
/// Align a join and its downstream spine without moving its incoming branches.
/// In particular, a cross-rank join need not inherit the first branch's row.
pub(super) fn center_merges(
    doc: &Document,
    indices: &[usize],
    ranks: &[usize],
    boxes: &mut [BoxAt],
) {
    for &gateway_index in indices {
        let node = &doc.nodes[gateway_index];
        if !gateway(node.kind) {
            continue;
        }
        let sources: Vec<_> = doc
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Sequence && e.to == node.id)
            .filter_map(|e| doc.nodes.iter().position(|n| n.id == e.from))
            .filter(|i| indices.contains(i))
            .collect();
        if sources.len() < 2 {
            continue;
        }
        let low = sources
            .iter()
            .map(|&i| boxes[i].y)
            .fold(f64::INFINITY, f64::min);
        let high = sources
            .iter()
            .map(|&i| boxes[i].y)
            .fold(f64::NEG_INFINITY, f64::max);
        if high - low < 35.0 {
            continue;
        }
        let shift = (low + high) / 2.0 - boxes[gateway_index].y;
        if shift.abs() < 1.0 {
            continue;
        }
        boxes[gateway_index].y += shift;
        let mut visited = vec![false; doc.nodes.len()];
        let mut pending = vec![gateway_index];
        visited[gateway_index] = true;
        while let Some(from) = pending.pop() {
            for e in doc
                .edges
                .iter()
                .filter(|e| e.kind == EdgeKind::Sequence && e.from == doc.nodes[from].id)
            {
                let Some(to) = doc.nodes.iter().position(|n| n.id == e.to) else {
                    continue;
                };
                if !indices.contains(&to) || visited[to] || ranks[to] <= ranks[gateway_index] {
                    continue;
                }
                visited[to] = true;
                if indices.iter().filter(|&&i| ranks[i] == ranks[to]).count() == 1 {
                    boxes[to].y += shift;
                }
                pending.push(to);
            }
        }
    }
}
