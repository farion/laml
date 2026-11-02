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
                let t = if degree <= 4 || group.len() == 1 {
                    0.0
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
    // Non-gateway shapes also need independent attachment positions: sequence,
    // message and data/annotation lines must not silently share a node port.
    for (node_index, node) in doc.nodes.iter().enumerate() {
        if gateway(node.kind) {
            continue;
        }
        let b = geom.nodes[node_index];
        let mut sides: Vec<(usize, bool, Side)> = Vec::new();
        for (index, e) in doc.edges.iter().enumerate() {
            for (out, id, other) in [(true, &e.from, &e.to), (false, &e.to, &e.from)] {
                if id != &node.id {
                    continue;
                }
                let other_index = doc.nodes.iter().position(|n| &n.id == other).unwrap();
                let q = geom.nodes[other_index];
                let side = if node.kind == Kind::Annotation {
                    Side::Left
                } else if node.kind == Kind::Boundary {
                    if q.y > b.y + b.h {
                        Side::Bottom
                    } else if q.x > b.x {
                        Side::Right
                    } else {
                        Side::Left
                    }
                } else if e.kind == EdgeKind::Message {
                    if node.pool != doc.nodes[other_index].pool
                        && doc.direction == FlowDirection::TopToBottom
                    {
                        if q.x > b.x { Side::Right } else { Side::Left }
                    } else if node.pool != doc.nodes[other_index].pool
                        || (q.y - b.y).abs() > (q.x - b.x).abs()
                    {
                        if q.y > b.y { Side::Bottom } else { Side::Top }
                    } else if q.x > b.x {
                        Side::Right
                    } else {
                        Side::Left
                    }
                } else if e.kind == EdgeKind::Sequence {
                    if !out
                        && doc.direction != FlowDirection::TopToBottom
                        && gateway(doc.nodes[other_index].kind)
                        && b.x > q.x
                        && ends[index].0.is_some_and(|port| port.side == Side::Bottom)
                    {
                        Side::Left
                    } else if doc.direction == FlowDirection::TopToBottom {
                        if out { Side::Bottom } else { Side::Top }
                    } else if (q.y - b.y).abs() > (q.x - b.x).abs() * 1.5 {
                        if q.y > b.y { Side::Bottom } else { Side::Top }
                    } else if out {
                        Side::Right
                    } else {
                        Side::Left
                    }
                } else if (q.y - b.y).abs() > (q.x - b.x).abs() * 1.1 {
                    if q.y > b.y { Side::Bottom } else { Side::Top }
                } else if q.x > b.x {
                    Side::Right
                } else {
                    Side::Left
                };
                sides.push((index, out, side));
            }
        }
        for side in [Side::Left, Side::Top, Side::Right, Side::Bottom] {
            let group: Vec<_> = sides.iter().filter(|(_, _, s)| *s == side).collect();
            let primary = if group.len() == 2
                && group
                    .iter()
                    .filter(|(i, _, _)| doc.edges[*i].kind == EdgeKind::Sequence)
                    .count()
                    == 1
            {
                group
                    .iter()
                    .position(|(i, _, _)| doc.edges[*i].kind == EdgeKind::Sequence)
            } else {
                None
            };
            for (slot, (index, out, _)) in group.iter().enumerate() {
                let t = if group.len() == 1 || primary == Some(slot) {
                    0.0
                } else if primary.is_some() {
                    let other = if *out {
                        &doc.edges[*index].to
                    } else {
                        &doc.edges[*index].from
                    };
                    let q = geom.nodes[doc.nodes.iter().position(|n| &n.id == other).unwrap()];
                    let delta = if matches!(side, Side::Left | Side::Right) {
                        q.y - b.y
                    } else {
                        q.x - b.x
                    };
                    if delta >= 0.0 { 0.52 } else { -0.52 }
                } else {
                    let spread = if node.attached.is_none()
                        && doc
                            .nodes
                            .iter()
                            .any(|n| n.attached.as_deref() == Some(&node.id))
                        && side == Side::Bottom
                    {
                        0.25
                    } else {
                        0.48
                    };
                    -spread + slot as f64 * 2.0 * spread / (group.len() - 1) as f64
                };
                let attach = regular_point(node.kind, b, side, t);
                if *out {
                    ends[*index].0 = Some(attach);
                } else {
                    ends[*index].1 = Some(attach);
                }
            }
        }
    }
    // A boundary flow descending into a task may dock off-center on the
    // task's top edge. Aligning both anchors removes a cosmetic dogleg without
    // moving either node or stealing an attachment from a second flow.
    for (index, edge) in doc.edges.iter().enumerate() {
        if edge.kind != EdgeKind::Sequence {
            continue;
        }
        let (Some(source), Some(target)) = ends[index] else {
            continue;
        };
        if source.side != Side::Bottom || target.side != Side::Top || source.y >= target.y - 8.0 {
            continue;
        }
        let Some(to) = doc.nodes.iter().position(|n| n.id == edge.to) else {
            continue;
        };
        if !matches!(
            doc.nodes[to].kind,
            Kind::Task
                | Kind::UserTask
                | Kind::ServiceTask
                | Kind::SendTask
                | Kind::ReceiveTask
                | Kind::ScriptTask
                | Kind::ManualTask
                | Kind::Collapsed
                | Kind::Subprocess
        ) {
            continue;
        }
        let b = geom.nodes[to];
        if (source.x - b.x).abs() > b.w / 2.0 - 14.0 {
            continue;
        }
        if ends.iter().enumerate().any(|(other, (_, attachment))| {
            other != index
                && doc.edges[other].to == edge.to
                && attachment.is_some_and(|a| a.side == Side::Top && (a.x - source.x).abs() < 12.0)
        }) {
            continue;
        }
        ends[index].1 = Some(Attach {
            side: Side::Top,
            x: source.x,
            y: target.y,
        });
    }
    Ok(Ports { ends })
}
fn regular_point(kind: Kind, b: super::super::geometry::BoxAt, side: Side, t: f64) -> Attach {
    let (half_w, half_h) = match kind {
        Kind::Data => (31.0, 39.0),
        Kind::Store => (31.0, 31.0),
        _ => (b.w / 2.0, b.h / 2.0),
    };
    let (x, y) = match side {
        Side::Left => (b.x - half_w, b.y + t * (half_h - 8.0)),
        Side::Right => (b.x + half_w, b.y + t * (half_h - 8.0)),
        Side::Top => (b.x + t * (half_w - 8.0), b.y - half_h),
        Side::Bottom => (
            b.x + t * (half_w - 8.0),
            b.y + if kind == Kind::Store { 39.0 } else { half_h },
        ),
    };
    if matches!(
        kind,
        Kind::Start | Kind::End | Kind::Intermediate | Kind::Boundary
    ) {
        let r = b.w / 2.0;
        let (x, y) = match side {
            Side::Left => (b.x - r * (1.0 - t * t).sqrt(), b.y + r * t),
            Side::Right => (b.x + r * (1.0 - t * t).sqrt(), b.y + r * t),
            Side::Top => (b.x + r * t, b.y - r * (1.0 - t * t).sqrt()),
            Side::Bottom => (b.x + r * t, b.y + r * (1.0 - t * t).sqrt()),
        };
        Attach { side, x, y }
    } else {
        Attach { side, x, y }
    }
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
        assert_eq!(y("Review"), y("Complete"));
        assert_eq!(y("Complete"), y("Merge"));
        assert_eq!(y("Merge"), y("Done"));
        assert_eq!(y("Complete"), (y("Approve") + y("RequestDocuments")) / 2.0);
        let svg = crate::bpmn::render(&doc, crate::svg::TextMode::System).unwrap();
        let arrows: Vec<_> = svg
            .lines()
            .filter(|line| line.contains("marker-end=\"url(#solid-arrow)\""))
            .collect();
        assert_eq!(arrows.len(), doc.edges.len());
        for (from, to) in [
            ("Complete", "Approve"),
            ("Complete", "RequestDocuments"),
            ("Approve", "Merge"),
            ("RequestDocuments", "Merge"),
        ] {
            assert_eq!(
                arrows[edge_index(&doc, from, to)].matches(" L ").count(),
                2,
                "{from}->{to} should have exactly one bend"
            );
        }
    }
    #[test]
    fn three_way_gateway_centers_the_trunk_on_the_middle_branch() {
        let source = "settings {\ntype: bpmn\n}\nsemantics {\nStart <X>@Split A ; B ; C\nA --> <X>@Merge --> End\nB --> Merge\nC --> Merge\n}";
        let doc = parse(source).unwrap();
        let geom = geometry::place(&doc).unwrap();
        let y = |id: &str| geom.nodes[doc.nodes.iter().position(|n| n.id == id).unwrap()].y;
        assert_eq!(y("Split"), y("B"));
        assert_eq!(y("Merge"), y("B"));
        assert_eq!(y("Start"), y("End"));
        let ports = assign(&doc, &geom).unwrap();
        for (name, side) in [("A", Side::Top), ("B", Side::Right), ("C", Side::Bottom)] {
            assert_eq!(
                ports.ends[edge_index(&doc, "Split", name)].0.unwrap().side,
                side
            );
        }
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
        // A single split with five exits and one incoming.
        let source = "settings {\ntype: bpmn\n}\nsemantics {\nstartEvent Begin\nBegin --> Start\nStart <X>@Choice A ; B ; C ; D ; E\n}";
        let doc = parse(source).unwrap();
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
        crate::bpmn::render(&doc, crate::svg::TextMode::System).unwrap();
        for (i, p) in points.iter().enumerate() {
            for q in points.iter().skip(i + 1) {
                assert_ne!(p, q);
            }
        }
    }
}
