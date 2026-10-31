//! Named boundary ports: measured text strip and deterministic left/right anchors.
use super::BoxNode;
use crate::syntax::{Diagram, PortSide};

#[derive(Clone, Debug)]
pub struct BoxPort {
    pub id: String,
    pub label: String,
    pub side: PortSide,
    pub x: f64,
    pub y: f64,
    pub color: String,
    pub fill: String,
    pub border_color: String,
}

pub(super) fn position(diagram: &Diagram, boxes: &mut [BoxNode]) {
    for index in 0..boxes.len() {
        let (left, right) = boxes.split_at_mut(index);
        let (current, after) = right.split_first_mut().expect("valid box index");
        let neighbors = left.iter().chain(after.iter());
        for (number, port) in current.ports.iter_mut().enumerate() {
            let other = diagram.relations.iter().find_map(|relation| {
                let opposite = if relation.from == port.id {
                    &relation.to
                } else if relation.to == port.id {
                    &relation.from
                } else {
                    return None;
                };
                neighbors.clone().find(|node| node.has_endpoint(opposite))
            });
            port.side = diagram
                .port_layout
                .iter()
                .find(|rule| rule.id == port.id)
                .map(|rule| rule.side)
                .unwrap_or_else(|| {
                    if other.is_some_and(|node| {
                        node.x + node.width / 2.0 < current.x + current.width / 2.0
                    }) {
                        PortSide::Left
                    } else {
                        PortSide::Right
                    }
                });
            port.x = match port.side {
                PortSide::Left => current.x,
                PortSide::Right => current.x + current.width,
            };
            port.y =
                current.y + current.port_start + current.font_size * 1.25 * (number as f64 + 0.5);
        }
    }
}
