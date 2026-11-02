//! Conservative second-stage port choices for the orthogonal edge planner.
//! Keep gateway, event, note and message attachments stable; only use a free
//! cardinal side of a rectangular task or store facing the other endpoint.
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
    if !matches!(edge.kind, EdgeKind::Sequence | EdgeKind::Data)
        || (edge.kind == EdgeKind::Data && outgoing)
    {
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
