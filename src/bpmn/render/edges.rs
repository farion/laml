//! Edge attachment and orthogonal routes.
use super::ports::{Attach, Ports};
use super::{Document, EdgeKind, Geometry, text};
use crate::bpmn::{Edge, Side};
use crate::syntax::FlowDirection;
use std::fmt::Write;

fn regular(b: super::super::geometry::BoxAt, side: Side) -> Attach {
    let (x, y) = match side {
        Side::Left => (b.x - b.w / 2.0, b.y),
        Side::Right => (b.x + b.w / 2.0, b.y),
        Side::Top => (b.x, b.y - b.h / 2.0),
        Side::Bottom => (b.x, b.y + b.h / 2.0),
    };
    Attach { side, x, y }
}
fn stub(a: Attach) -> (f64, f64) {
    match a.side {
        Side::Left => (a.x - 23.0, a.y),
        Side::Right => (a.x + 23.0, a.y),
        Side::Top => (a.x, a.y - 23.0),
        Side::Bottom => (a.x, a.y + 23.0),
    }
}
fn gateway_route(
    doc: &Document,
    geom: &Geometry,
    e: &Edge,
    from: Option<Attach>,
    to: Option<Attach>,
) -> (String, f64, f64) {
    let a = geom.nodes[doc.nodes.iter().position(|n| n.id == e.from).unwrap()];
    let b = geom.nodes[doc.nodes.iter().position(|n| n.id == e.to).unwrap()];
    let vertical = doc.direction == FlowDirection::TopToBottom;
    let source = from.unwrap_or_else(|| {
        regular(
            a,
            if vertical {
                if b.y >= a.y { Side::Bottom } else { Side::Top }
            } else if b.x >= a.x {
                Side::Right
            } else {
                Side::Left
            },
        )
    });
    let target = to.unwrap_or_else(|| {
        regular(
            b,
            if vertical {
                if b.y >= a.y { Side::Top } else { Side::Bottom }
            } else if b.x >= a.x {
                Side::Left
            } else {
                Side::Right
            },
        )
    });
    let (sx, sy) = stub(source);
    let (tx, ty) = stub(target);
    let start_vertical = matches!(source.side, Side::Top | Side::Bottom);
    let end_vertical = matches!(target.side, Side::Top | Side::Bottom);
    let mut points = vec![(source.x, source.y), (sx, sy)];
    match (start_vertical, end_vertical) {
        (true, false) => points.push((tx, sy)),
        (false, true) => points.push((sx, ty)),
        (true, true) => {
            let mid = (sy + ty) / 2.0;
            points.extend([(sx, mid), (tx, mid)]);
        }
        (false, false) => {
            let mid = (sx + tx) / 2.0;
            points.extend([(mid, sy), (mid, ty)]);
        }
    }
    points.extend([(tx, ty), (target.x, target.y)]);
    points.dedup_by(|a, b| (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01);
    let mut path = String::new();
    for (i, (x, y)) in points.iter().enumerate() {
        write!(path, "{} {x:.1} {y:.1}", if i == 0 { "M" } else { " L" }).unwrap();
    }
    let label_segment = points
        .windows(2)
        .filter(|pair| (pair[0].1 - pair[1].1).abs() < 0.01 && (pair[0].0 - pair[1].0).abs() > 28.0)
        .max_by(|a, b| (a[0].0 - a[1].0).abs().total_cmp(&(b[0].0 - b[1].0).abs()));
    let (label_x, label_y) = label_segment
        .map_or(((sx + tx) / 2.0, (sy + ty) / 2.0 - 9.0), |pair| {
            ((pair[0].0 + pair[1].0) / 2.0, pair[0].1 - 9.0)
        });
    (path, label_x, label_y)
}
pub(super) fn edge(
    svg: &mut String,
    doc: &Document,
    geom: &Geometry,
    e: &Edge,
    index: usize,
    ports: &Ports,
) {
    let ai = doc.nodes.iter().position(|n| n.id == e.from).unwrap();
    let bi = doc.nodes.iter().position(|n| n.id == e.to).unwrap();
    let (a, b) = (geom.nodes[ai], geom.nodes[bi]);
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let horizontal = dx.abs() > dy.abs() * 0.75 && e.kind != EdgeKind::Message;
    let (x1, y1, x2, y2) = if horizontal {
        let sign = dx.signum();
        (
            a.x + sign * a.w / 2.0,
            a.y,
            b.x - sign * b.w / 2.0 - sign * 4.0,
            b.y,
        )
    } else {
        let sign = dy.signum();
        (
            a.x,
            a.y + sign * a.h / 2.0,
            b.x,
            b.y - sign * b.h / 2.0 - sign * 4.0,
        )
    };
    let (dash, marker) = match e.kind {
        EdgeKind::Sequence => ("", "solid-arrow"),
        EdgeKind::Message => ("stroke-dasharray=\"8 5\"", "open-arrow"),
        EdgeKind::Data => ("stroke-dasharray=\"2 5\"", "open-arrow"),
        EdgeKind::Association => ("stroke-dasharray=\"4 4\"", ""),
    };
    let (mid_x, mid_y) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
    let blocked = horizontal
        && (y1 - y2).abs() < 2.0
        && doc.nodes.iter().enumerate().any(|(i, n)| {
            if i == ai
                || i == bi
                || n.parent != doc.nodes[ai].parent
                || n.pool != doc.nodes[ai].pool
            {
                return false;
            }
            let q = geom.nodes[i];
            q.x + q.w / 2.0 > x1.min(x2)
                && q.x - q.w / 2.0 < x1.max(x2)
                && q.y - q.h / 2.0 < y1
                && q.y + q.h / 2.0 > y1
        });
    let channel_clear = |y: f64| {
        !doc.nodes.iter().enumerate().any(|(i, n)| {
            if i == ai
                || i == bi
                || n.parent != doc.nodes[ai].parent
                || n.pool != doc.nodes[ai].pool
            {
                return false;
            }
            let q = geom.nodes[i];
            q.x + q.w / 2.0 > x1.min(x2)
                && q.x - q.w / 2.0 < x1.max(x2)
                && q.y - q.h / 2.0 - 8.0 < y
                && q.y + q.h / 2.0 + 8.0 > y
        })
    };
    let mut corridor = y1.max(y2) + a.h.max(b.h) / 2.0 + 55.0;
    if !channel_clear(corridor) {
        corridor = y1.min(y2) - a.h.max(b.h) / 2.0 - 55.0;
        while !channel_clear(corridor) && corridor > 35.0 {
            corridor -= 45.0;
        }
    }
    let blocked_vertical = !horizontal
        && e.kind != EdgeKind::Message
        && doc.nodes.iter().enumerate().any(|(i, n)| {
            if i == ai
                || i == bi
                || n.parent != doc.nodes[ai].parent
                || n.pool != doc.nodes[ai].pool
            {
                return false;
            }
            let q = geom.nodes[i];
            q.x - q.w / 2.0 < x1
                && q.x + q.w / 2.0 > x1
                && q.y + q.h / 2.0 > y1.min(y2)
                && q.y - q.h / 2.0 < y1.max(y2)
        });
    let corridor_x = if blocked_vertical {
        doc.nodes
            .iter()
            .enumerate()
            .filter(|(i, n)| {
                *i != ai
                    && *i != bi
                    && n.parent == doc.nodes[ai].parent
                    && n.pool == doc.nodes[ai].pool
            })
            .map(|(i, _)| geom.nodes[i])
            .filter(|q| q.y + q.h / 2.0 > y1.min(y2) && q.y - q.h / 2.0 < y1.max(y2))
            .map(|q| q.x + q.w / 2.0 + 30.0)
            .fold(x1.max(x2) + 30.0, f64::max)
    } else {
        mid_x
    };
    let gateway_ports = ports.ends[index];
    let gateway_path = if e.kind == EdgeKind::Sequence
        && (gateway_ports.0.is_some() || gateway_ports.1.is_some())
    {
        Some(gateway_route(
            doc,
            geom,
            e,
            gateway_ports.0,
            gateway_ports.1,
        ))
    } else {
        None
    };
    let path = if let Some((path, _, _)) = &gateway_path {
        path.clone()
    } else if blocked {
        format!(
            "M {x1:.1} {y1:.1} L {:.1} {y1:.1} L {:.1} {corridor:.1} L {:.1} {corridor:.1} L {:.1} {y2:.1} L {x2:.1} {y2:.1}",
            x1 + 15.0,
            x1 + 15.0,
            x2 - 15.0,
            x2 - 15.0
        )
    } else if horizontal {
        format!("M {x1:.1} {y1:.1} L {mid_x:.1} {y1:.1} L {mid_x:.1} {y2:.1} L {x2:.1} {y2:.1}")
    } else if blocked_vertical {
        format!(
            "M {x1:.1} {y1:.1} L {corridor_x:.1} {y1:.1} L {corridor_x:.1} {y2:.1} L {x2:.1} {y2:.1}"
        )
    } else {
        format!("M {x1:.1} {y1:.1} L {x1:.1} {mid_y:.1} L {x2:.1} {mid_y:.1} L {x2:.1} {y2:.1}")
    };
    let arrow = if marker.is_empty() {
        String::new()
    } else {
        format!("marker-end=\"url(#{marker})\"")
    };
    writeln!(
        svg,
        "<path d=\"{path}\" fill=\"none\" stroke=\"#263548\" stroke-width=\"1.7\" {dash} {arrow}/>"
    )
    .unwrap();
    if e.kind == EdgeKind::Message {
        writeln!(
            svg,
            "<circle cx=\"{x1:.1}\" cy=\"{y1:.1}\" r=\"4\" fill=\"white\" stroke=\"#263548\"/>"
        )
        .unwrap();
    }
    if let Some(label) = &e.label {
        let (label_x, label_y) = gateway_path.as_ref().map_or(
            (mid_x, if blocked { corridor - 9.0 } else { mid_y - 9.0 }),
            |(_, x, y)| (*x, *y),
        );
        text(svg, label_x, label_y, label, 12.0);
    }
}
