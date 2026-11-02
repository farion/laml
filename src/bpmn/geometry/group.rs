//! Rank placement and measured lane breadth for both diagram directions.
use super::{BoxAt, Document, branch, size};
use crate::bpmn::EdgeKind;
pub(super) fn place_group(
    doc: &Document,
    indices: &[usize],
    ranks: &[usize],
    boxes: &mut [BoxAt],
    left: f64,
    top: f64,
) -> (f64, f64) {
    let vertical = doc.direction == crate::syntax::FlowDirection::TopToBottom;
    let max_rank = indices.iter().map(|&i| ranks[i]).max().unwrap_or(0);
    let mut x = vec![left; max_rank + 1];
    // Before the final axis swap, ranks use x and lanes use y. Their spans
    // must still be measured from the node's *final* (unrotated) shape.
    let widths: Vec<_> = (0..=max_rank)
        .map(|rank| {
            indices
                .iter()
                .filter(|&&i| ranks[i] == rank)
                .map(|&i| {
                    let (w, h) = size(doc, &doc.nodes[i]);
                    if vertical { h } else { w }
                })
                .fold(70.0_f64, f64::max)
        })
        .collect();
    for rank in 1..=max_rank {
        x[rank] = x[rank - 1] + widths[rank - 1] / 2.0 + widths[rank] / 2.0 + 80.0;
    }
    let first_height = indices
        .iter()
        .map(|&i| {
            let (w, h) = size(doc, &doc.nodes[i]);
            if vertical { w } else { h }
        })
        .fold(70.0_f64, f64::max);
    let first_center = top + 30.0 + if vertical { 24.0 } else { 0.0 } + first_height / 2.0;
    let mut row_bottom = vec![top + 30.0; max_rank + 1];
    let mut occupied = vec![false; max_rank + 1];
    let mut height = 100.0_f64;
    for &i in indices {
        let rank = ranks[i];
        let (w, h) = size(doc, &doc.nodes[i]);
        let breadth = if vertical { w } else { h };
        let y = if occupied[rank] {
            row_bottom[rank] + breadth / 2.0
        } else {
            first_center
        };
        occupied[rank] = true;
        boxes[i] = BoxAt {
            x: x[rank],
            y,
            w,
            h,
        };
        row_bottom[rank] = y + breadth / 2.0 + if vertical { 24.0 } else { 45.0 };
        height = height.max(row_bottom[rank] - top + if vertical { 0.0 } else { 25.0 });
    }
    if let Some(center) = branch::spine_center(doc, indices, ranks, boxes) {
        for &i in indices {
            if indices.iter().filter(|&&j| ranks[j] == ranks[i]).count() == 1
                && doc.edges.iter().any(|e| {
                    e.kind == EdgeKind::Sequence
                        && (e.from == doc.nodes[i].id || e.to == doc.nodes[i].id)
                })
            {
                boxes[i].y = center;
            }
        }
    }
    branch::uncross_merges(doc, indices, boxes);
    branch::center_merges(doc, indices, ranks, boxes);
    if vertical && !indices.is_empty() {
        let min = indices
            .iter()
            .map(|&i| boxes[i].y - boxes[i].w / 2.0)
            .fold(f64::INFINITY, f64::min);
        let max = indices
            .iter()
            .map(|&i| boxes[i].y + boxes[i].w / 2.0)
            .fold(f64::NEG_INFINITY, f64::max);
        let shift = top + 30.0 + 24.0 - min;
        for &i in indices {
            boxes[i].y += shift;
        }
        height = (max - min) + 30.0 + 48.0;
    } else {
        height = indices
            .iter()
            .map(|&i| boxes[i].y + boxes[i].h / 2.0 + 70.0 - top)
            .fold(height, f64::max);
    }
    let width = x[max_rank] - left + widths[max_rank] / 2.0 + 70.0;
    (width, height)
}
