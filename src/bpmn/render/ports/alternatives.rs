//! Conservative second-stage port choices for the orthogonal edge planner.
//! Keep gateway, event, note and message attachments stable. For tasks and
//! stores, compare free sides and distinct positions on an occupied side.
use super::{Attach, Ports, gateway, regular_point};
use crate::bpmn::geometry::Geometry;
use crate::bpmn::{Document, EdgeKind, Kind, Side};

pub(in crate::bpmn::render) fn for_end(
    doc: &Document,
    geom: &Geometry,
    ports: &Ports,
    index: usize,
    outgoing: bool,
) -> Vec<Attach> {
    let original = if outgoing {
        ports.ends[index].0
    } else {
        ports.ends[index].1
    }
    .unwrap();
    let mut choices = vec![original];
    let edge = &doc.edges[index];
    if !matches!(edge.kind, EdgeKind::Sequence | EdgeKind::Data) {
        return choices;
    }
    let id = if outgoing { &edge.from } else { &edge.to };
    let other = if outgoing { &edge.to } else { &edge.from };
    let node_index = doc.nodes.iter().position(|node| &node.id == id).unwrap();
    let other_index = doc.nodes.iter().position(|node| &node.id == other).unwrap();
    let node = &doc.nodes[node_index];
    if gateway(node.kind)
        || !matches!(
            node.kind,
            Kind::Task
                | Kind::UserTask
                | Kind::ServiceTask
                | Kind::SendTask
                | Kind::ReceiveTask
                | Kind::ScriptTask
                | Kind::ManualTask
                | Kind::Collapsed
                | Kind::Subprocess
                | Kind::Store
        )
    {
        return choices;
    }
    let b = geom.nodes[node_index];
    let q = geom.nodes[other_index];
    // A data association can exit a task at distinct points on its bottom
    // or lower-left border. Compare the actual anchors, not just their side:
    // other flows may safely occupy the same edge at separate locations.
    if edge.kind == EdgeKind::Data && outgoing && q.x < b.x && q.y > b.y {
        for t in [0.9, 1.0] {
            let candidate = regular_point(node.kind, b, Side::Left, t);
            if ports.ends.iter().enumerate().any(|(i, (from, to))| {
                i != index
                    && ((doc.edges[i].from == *id
                        && from.is_some_and(|p| {
                            (p.x - candidate.x).abs() + (p.y - candidate.y).abs() < 10.0
                        }))
                        || (doc.edges[i].to == *id
                            && to.is_some_and(|p| {
                                (p.x - candidate.x).abs() + (p.y - candidate.y).abs() < 10.0
                            })))
            }) {
                continue;
            }
            choices.push(candidate);
        }
    }
    if edge.kind == EdgeKind::Data && outgoing && original.side == Side::Bottom {
        for t in [-0.75, -0.6, -0.45, -0.3, 0.0] {
            let candidate = regular_point(node.kind, b, Side::Bottom, t);
            if (candidate.x - original.x).abs() < 12.0
                || ports.ends.iter().enumerate().any(|(i, (from, to))| {
                    i != index
                        && ((doc.edges[i].from == *id
                            && from.is_some_and(|p| {
                                (p.x - candidate.x).abs() + (p.y - candidate.y).abs() < 15.0
                            }))
                            || (doc.edges[i].to == *id
                                && to.is_some_and(|p| {
                                    (p.x - candidate.x).abs() + (p.y - candidate.y).abs() < 15.0
                                })))
                })
            {
                continue;
            }
            choices.push(candidate);
        }
    }
    let mut sides = Vec::new();
    if (q.x - b.x).abs() > 1.0 {
        sides.push(if q.x > b.x { Side::Right } else { Side::Left });
    }
    if (q.y - b.y).abs() > 1.0 {
        sides.push(if q.y > b.y { Side::Bottom } else { Side::Top });
    }
    for side in sides {
        if side == original.side
            || ports.ends.iter().enumerate().any(|(i, (from, to))| {
                i != index
                    && (if outgoing {
                        doc.edges[i].from == *id && from.is_some_and(|p| p.side == side)
                    } else {
                        doc.edges[i].to == *id && to.is_some_and(|p| p.side == side)
                    })
            })
        {
            continue;
        }
        choices.push(regular_point(node.kind, b, side, 0.0));
    }
    choices
}
