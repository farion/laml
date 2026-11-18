//! Canonical contour docks connect endpoints to the corridor graph, not each other.
use super::super::{Point, Route, geometry, obstacles::Obstacles, orthogonal, side_rules};
use crate::{
    geometry::BoxNode,
    syntax::{Diagram, EndPosition, PortSide},
};

#[derive(Clone, Copy)]
pub(super) struct Dock {
    pub port: Point,
    pub stub: Point,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn docks(
    d: &Diagram,
    index: usize,
    boxes: &[BoxNode],
    node: usize,
    other: usize,
    source: bool,
    previous: &[Route],
    check: &Obstacles,
    gap: f64,
) -> Vec<Dock> {
    let rel = &d.relations[index];
    let id = if source { &rel.from } else { &rel.to };
    let b = &boxes[node];
    let member = (id != &b.id).then_some(id.as_str());
    let required = side_rules::requested(
        d,
        index,
        if source {
            EndPosition::Source
        } else {
            EndPosition::Target
        },
    );
    let named_side = member
        .and_then(|id| b.ports.iter().find(|p| p.id == id))
        .map(|p| p.side);
    let mut result = Vec::new();
    for side in [
        PortSide::Right,
        PortSide::Left,
        PortSide::Bottom,
        PortSide::Top,
    ] {
        if required.is_some_and(|s| s != side) || named_side.is_some_and(|s| s != side) {
            continue;
        }
        let ports = if member.is_some() && named_side.is_none() {
            if !matches!(side, PortSide::Left | PortSide::Right) {
                continue;
            }
            geometry::member_port(
                b,
                id,
                side_rules::member_towards(b, Some(side), geometry::center(&boxes[other])),
            )
            .into_iter()
            .collect()
        } else {
            orthogonal::endpoints(
                b,
                member,
                Some(side),
                side,
                true,
                geometry::center(&boxes[other]),
            )
        };
        for port in ports {
            let mut stub = port;
            let symbol = if source {
                rel.appearance.start
            } else {
                rel.appearance.end
            };
            let gap = if symbol == crate::syntax::EndSymbol::None {
                20.0
            } else {
                gap
            };
            let gap = gap * super::super::container_ends::polarity(b, &boxes[other]);
            match side {
                PortSide::Left => stub.x -= gap,
                PortSide::Right => stub.x += gap,
                PortSide::Top => stub.y -= gap,
                PortSide::Bottom => stub.y += gap,
            }
            if check.clear(&[port, stub], previous)
                && !result
                    .iter()
                    .any(|d: &Dock| d.port == port && d.stub == stub)
            {
                result.push(Dock { port, stub });
            }
        }
    }
    result
}
