//! Keep annotations out of a boundary event's normal outgoing corridor.
use super::BoxAt;
use crate::bpmn::{Document, EdgeKind, Kind};
pub(super) fn clear_boundary_exit(doc: &Document, boxes: &mut [BoxAt]) {
    for (i, note) in doc.nodes.iter().enumerate() {
        if note.kind != Kind::Annotation {
            continue;
        }
        let Some(owner) = doc
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Association)
            .find_map(|e| {
                if e.from == note.id {
                    Some(&e.to)
                } else if e.to == note.id {
                    Some(&e.from)
                } else {
                    None
                }
            })
        else {
            continue;
        };
        let Some(boundary) = doc.nodes.iter().enumerate().find(|(_, n)| {
            n.kind == Kind::Boundary && n.attached.as_deref() == Some(owner.as_str())
        }) else {
            continue;
        };
        let b = boxes[boundary.0];
        if boxes[i].y < b.y || boxes[i].x + boxes[i].w / 2.0 + 12.0 < b.x {
            continue;
        }
        let mut candidate = boxes[i];
        candidate.x = b.x - candidate.w / 2.0 - 17.0;
        let blocked = doc.nodes.iter().enumerate().any(|(j, n)| {
            j != i
                && j != boundary.0
                && n.pool == note.pool
                && n.lane == note.lane
                && (boxes[j].x - candidate.x).abs() < (boxes[j].w + candidate.w) / 2.0 + 10.0
                && (boxes[j].y - candidate.y).abs() < (boxes[j].h + candidate.h) / 2.0 + 10.0
        });
        if !blocked && candidate.x - candidate.w / 2.0 > 140.0 {
            boxes[i] = candidate;
        }
    }
}
