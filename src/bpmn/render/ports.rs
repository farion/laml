//! Gateway ports: distinct cardinal anchors for degree <= 4, distributed
//! perimeter points for larger gateways. Explicit layout overrides reserve sides.
use super::Geometry;
use crate::bpmn::{Document, EdgeKind, Kind, Side};
use crate::syntax::FlowDirection;

#[derive(Clone, Copy, Debug)]
pub(super) struct Attach {
    pub side: Side,
    pub x: f64,
    pub y: f64,
}
pub(super) struct Ports {
    pub ends: Vec<(Option<Attach>, Option<Attach>)>,
}
fn gateway(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Exclusive | Kind::Parallel | Kind::Inclusive | Kind::EventBased
    )
}
fn rotate(side: Side, direction: FlowDirection) -> Side {
    if direction != FlowDirection::TopToBottom {
        return side;
    }
    match side {
        Side::Left => Side::Top,
        Side::Top => Side::Left,
        Side::Right => Side::Bottom,
        Side::Bottom => Side::Right,
    }
}
fn preferred(in_count: usize, out_count: usize, outgoing: bool, order: usize) -> Side {
    match (in_count, out_count, outgoing) {
        (1, 2, false) | (1, 3, false) | (1, 1, false) => Side::Left,
        (1, 2, true) | (0, 2, true) => [Side::Top, Side::Bottom][order.min(1)],
        (2, 1, false) | (2, 0, false) => [Side::Top, Side::Bottom][order.min(1)],
        (2, 1, true) | (3, 1, true) | (1, 1, true) => Side::Right,
        (1, 3, true) | (0, 3, true) => [Side::Top, Side::Right, Side::Bottom][order.min(2)],
        (3, 1, false) | (3, 0, false) => [Side::Top, Side::Left, Side::Bottom][order.min(2)],
        (2, 2, false) => [Side::Left, Side::Top][order.min(1)],
        (2, 2, true) => [Side::Right, Side::Bottom][order.min(1)],
        (_, _, false) => [Side::Left, Side::Top, Side::Bottom, Side::Right][order.min(3)],
        (_, _, true) => [Side::Right, Side::Top, Side::Bottom, Side::Left][order.min(3)],
    }
}
fn point(side: Side, x: f64, y: f64, r: f64, t: f64) -> Attach {
    let (x, y) = match side {
        Side::Top => (x + r * t, y - r * (1.0 - t.abs())),
        Side::Right => (x + r * (1.0 - t.abs()), y + r * t),
        Side::Bottom => (x + r * t, y + r * (1.0 - t.abs())),
        Side::Left => (x - r * (1.0 - t.abs()), y + r * t),
    };
    Attach { side, x, y }
}
pub(super) fn assign(doc: &Document, geom: &Geometry) -> Result<Ports, String> {
    let mut ends = vec![(None, None); doc.edges.len()];
    for (node_index, node) in doc.nodes.iter().enumerate() {
        if !gateway(node.kind) {
            continue;
        }
        let mut incoming: Vec<_> = doc
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.kind == EdgeKind::Sequence && e.to == node.id)
            .map(|(i, _)| i)
            .collect();
        let mut outgoing: Vec<_> = doc
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.kind == EdgeKind::Sequence && e.from == node.id)
            .map(|(i, _)| i)
            .collect();
        let cross = |id: &str| {
            let other = doc.nodes.iter().position(|n| n.id == id).unwrap();
            if doc.direction == FlowDirection::TopToBottom {
                geom.nodes[other].x
            } else {
                geom.nodes[other].y
            }
        };
        incoming.sort_by(|&a, &b| {
            cross(&doc.edges[a].from)
                .total_cmp(&cross(&doc.edges[b].from))
                .then(a.cmp(&b))
        });
        outgoing.sort_by(|&a, &b| {
            cross(&doc.edges[a].to)
                .total_cmp(&cross(&doc.edges[b].to))
                .then(a.cmp(&b))
        });
        let degree = incoming.len() + outgoing.len();
        let mut chosen: Vec<(usize, bool, Side)> = Vec::new();
        let mut used = Vec::new();
        for (&index, out) in incoming
            .iter()
            .map(|i| (i, false))
            .chain(outgoing.iter().map(|i| (i, true)))
        {
            let rule = doc
                .flow_layout
                .iter()
                .find(|r| r.from == doc.edges[index].from && r.to == doc.edges[index].to);
            let side = rule.and_then(|r| if out { r.from_side } else { r.to_side });
            if let Some(side) = side {
                if degree <= 4 && used.contains(&side) {
                    return Err(format!(
                        "gateway {} has two flows on {:?}; use distinct sides",
                        node.id, side
                    ));
                }
                used.push(side);
                chosen.push((index, out, side));
            }
        }
        for (out, indices) in [(false, &incoming), (true, &outgoing)] {
            for (order, &index) in indices.iter().enumerate() {
                if chosen.iter().any(|(i, _, _)| *i == index) {
                    continue;
                }
                let default = if degree > 4 {
                    if out { Side::Right } else { Side::Left }
                } else {
                    preferred(incoming.len(), outgoing.len(), out, order)
                };
                let default = rotate(default, doc.direction);
                let mut candidates = vec![default];
                for side in [Side::Top, Side::Bottom, Side::Left, Side::Right] {
                    let side = rotate(side, doc.direction);
                    if !candidates.contains(&side) {
                        candidates.push(side);
                    }
                }
                let side = if degree > 4 {
                    default
                } else {
                    *candidates
                        .iter()
                        .find(|side| !used.contains(side))
                        .ok_or_else(|| format!("gateway {} has no free attachment", node.id))?
                };
                used.push(side);
                chosen.push((index, out, side));
            }
        }
        let b = geom.nodes[node_index];
        for side in [Side::Left, Side::Top, Side::Right, Side::Bottom] {
            let group: Vec<_> = chosen.iter().filter(|(_, _, s)| *s == side).collect();
            for (slot, (index, out, _)) in group.iter().enumerate() {
                let t = if degree <= 4 {
                    0.0
                } else if group.len() == 1 {
                    0.18
                } else {
                    let raw = -0.72 + slot as f64 * 1.44 / (group.len() - 1) as f64;
                    if raw.abs() < 0.02 { 0.08 } else { raw }
                };
                let attach = point(side, b.x, b.y, b.w / 2.0, t);
                if *out {
                    ends[*index].0 = Some(attach);
                } else {
                    ends[*index].1 = Some(attach);
                }
            }
        }
    }
    Ok(Ports { ends })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bpmn::{geometry, parse};
    fn basic() -> Document {
        parse(include_str!("../../../examples/bpmn-basic.laml")).unwrap()
    }
    fn edge_index(doc: &Document, from: &str, to: &str) -> usize {
        doc.edges
            .iter()
            .position(|e| e.from == from && e.to == to)
            .unwrap()
    }
    #[test]
    fn split_and_merge_use_distinct_cardinal_anchors_and_first_row_is_aligned() {
        let doc = basic();
        let geom = geometry::place(&doc).unwrap();
        let ports = assign(&doc, &geom).unwrap();
        let approve = ports.ends[edge_index(&doc, "Complete", "Approve")]
            .0
            .unwrap();
        let request = ports.ends[edge_index(&doc, "Complete", "RequestDocuments")]
            .0
            .unwrap();
        let top = ports.ends[edge_index(&doc, "Approve", "Merge")].1.unwrap();
        let bottom = ports.ends[edge_index(&doc, "RequestDocuments", "Merge")]
            .1
            .unwrap();
        assert_eq!((approve.side, request.side), (Side::Top, Side::Bottom));
        assert_eq!((top.side, bottom.side), (Side::Top, Side::Bottom));
        assert_ne!((approve.x, approve.y), (request.x, request.y));
        assert_ne!((top.x, top.y), (bottom.x, bottom.y));
        let y = |id: &str| geom.nodes[doc.nodes.iter().position(|n| n.id == id).unwrap()].y;
        assert_eq!(y("Start"), y("Review"));
        assert_eq!(y("Merge"), y("Done"));
    }
    #[test]
    fn explicit_side_overrides_are_mandatory_and_collisions_fail() {
        let mut doc = basic();
        doc.flow_layout.push(crate::bpmn::FlowPortRule {
            from: "Complete".into(),
            to: "Approve".into(),
            from_side: Some(Side::Right),
            to_side: None,
        });
        let geom = geometry::place(&doc).unwrap();
        let ports = assign(&doc, &geom).unwrap();
        assert_eq!(
            ports.ends[edge_index(&doc, "Complete", "Approve")]
                .0
                .unwrap()
                .side,
            Side::Right
        );
        doc.flow_layout.push(crate::bpmn::FlowPortRule {
            from: "Complete".into(),
            to: "RequestDocuments".into(),
            from_side: Some(Side::Right),
            to_side: None,
        });
        assert!(assign(&doc, &geom).is_err());
    }
    #[test]
    fn degree_above_four_spreads_perimeter_attachments() {
        let source = "settings {\ntype: bpmn\n}\nsemantics {\nStart --> <X>@Choice\nChoice <X>@Extra A ; B\n}";
        // A single split with five exits and one incoming.
        let source = source.replace(
            "Start --> <X>@Choice\nChoice <X>@Extra A ; B",
            "Start <X>@Choice A ; B ; C ; D ; E",
        );
        let doc = parse(&source).unwrap();
        let geom = geometry::place(&doc).unwrap();
        let ports = assign(&doc, &geom).unwrap();
        let points: Vec<_> = doc
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.from == "Choice")
            .map(|(i, _)| {
                let a = ports.ends[i].0.unwrap();
                (a.x, a.y)
            })
            .collect();
        assert_eq!(points.len(), 5);
        for (i, p) in points.iter().enumerate() {
            for q in points.iter().skip(i + 1) {
                assert_ne!(p, q);
            }
        }
    }
}
