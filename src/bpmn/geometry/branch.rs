//! Align the single-node spine with the middle of a gateway's branches.
//! This is an automatic preference, applied before mandatory layout rules.
use super::BoxAt;
use crate::bpmn::{Document, EdgeKind, Kind};

pub(super) fn spine_center(
    doc: &Document,
    indices: &[usize],
    ranks: &[usize],
    boxes: &[BoxAt],
) -> Option<f64> {
    let mut best: Option<(f64, f64)> = None;
    for (gateway_index, node) in doc.nodes.iter().enumerate() {
        if !matches!(
            node.kind,
            Kind::Exclusive | Kind::Parallel | Kind::Inclusive | Kind::EventBased
        ) || !indices.contains(&gateway_index)
        {
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
