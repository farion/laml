//! Named boundary ports with measured labels and deterministic four-side anchors.
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
        let sides: Vec<_> = current
            .ports
            .iter()
            .map(|port| {
                diagram
                    .port_layout
                    .iter()
                    .find(|rule| rule.id == port.id)
                    .map(|rule| rule.side)
                    .unwrap_or_else(|| {
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
                        if other.is_some_and(|node| {
                            node.x + node.width / 2.0 < current.x + current.width / 2.0
                        }) {
                            PortSide::Left
                        } else {
                            PortSide::Right
                        }
                    })
            })
            .collect();
        for (number, port) in current.ports.iter_mut().enumerate() {
            port.side = sides[number];
            let (x, y) = match port.side {
                PortSide::Left | PortSide::Right => {
                    let row = sides[..number]
                        .iter()
                        .filter(|side| matches!(side, PortSide::Left | PortSide::Right))
                        .count();
                    (
                        if port.side == PortSide::Left {
                            current.x
                        } else {
                            current.x + current.width
                        },
                        current.y
                            + current.port_start
                            + current.font_size * 1.25 * (row as f64 + 0.5),
                    )
                }
                PortSide::Top | PortSide::Bottom => {
                    let rank = sides[..number]
                        .iter()
                        .filter(|side| **side == port.side)
                        .count();
                    let count = sides.iter().filter(|side| **side == port.side).count();
                    (
                        current.x + current.width * (rank + 1) as f64 / (count + 1) as f64,
                        if port.side == PortSide::Top {
                            current.y
                        } else {
                            current.y + current.height
                        },
                    )
                }
            };
            port.x = x;
            port.y = y;
        }
    }
}
