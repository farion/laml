//! Preserve a clear straight lane for a downward gateway branch where a
//! small, safe adjustment of an unrelated node removes several route bends.
use super::{BoxAt, move_node};
use crate::bpmn::{Document, EdgeKind, Kind};
fn gateway(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Exclusive | Kind::Parallel | Kind::Inclusive | Kind::EventBased
    )
}
pub(super) fn clear_lower_branches(doc: &Document, boxes: &mut [BoxAt]) {
    for e in doc.edges.iter().filter(|e| e.kind == EdgeKind::Sequence) {
        let Some(from) = doc.nodes.iter().position(|n| n.id == e.from) else {
            continue;
        };
        let Some(to) = doc.nodes.iter().position(|n| n.id == e.to) else {
            continue;
        };
        if !gateway(doc.nodes[from].kind)
            || doc.nodes[from].pool != doc.nodes[to].pool
            || boxes[to].y < boxes[from].y + boxes[from].h / 2.0 + 50.0
        {
            continue;
        }
        let axis = boxes[from].x;
        for i in 0..doc.nodes.len() {
            if i == from
                || i == to
                || doc.nodes[i].pool != doc.nodes[to].pool
                || doc.nodes[i].lane != doc.nodes[to].lane
                || gateway(doc.nodes[i].kind)
                || boxes[i].y < boxes[from].y + boxes[from].h / 2.0
                || boxes[i].y > boxes[to].y
                || boxes[i].x - boxes[i].w / 2.0 >= axis + 8.0
                || boxes[i].x + boxes[i].w / 2.0 <= axis - 8.0
            {
                continue;
            }
            let desired = axis - boxes[i].w / 2.0 - 12.0;
            if desired >= boxes[i].x || desired - boxes[i].w / 2.0 < 80.0 {
                continue;
            }
            let mut candidate = boxes[i];
            candidate.x = desired;
            let collides = boxes.iter().enumerate().any(|(j, b)| {
                j != i
                    && doc.nodes[j].pool == doc.nodes[i].pool
                    && doc.nodes[j].lane == doc.nodes[i].lane
                    && (b.x - candidate.x).abs() < (b.w + candidate.w) / 2.0 + 8.0
                    && (b.y - candidate.y).abs() < (b.h + candidate.h) / 2.0 + 8.0
            });
            if !collides {
                let y = boxes[i].y;
                move_node(doc, boxes, i, desired, y);
            }
        }
    }
}
