//! Reserve ordered departure slots for siblings on a shared face.
use super::{Point, geometry::center, preferred::forced_port};
use crate::{
    geometry::{BoxNode, NodeShape},
    syntax::{Diagram, PortSide},
};

pub(super) fn preferred(
    diagram: &Diagram,
    index: usize,
    boxes: &[BoxNode],
    from: usize,
) -> Option<Point> {
    let node = &boxes[from];
    if node.shape != NodeShape::Rectangle || node.is_container {
        return None;
    }
    let relation = &diagram.relations[index];
    if relation.from != node.id {
        return None;
    }
    let origin = center(node);
    let face = |id: &str| {
        let b = boxes.iter().find(|n| n.has_endpoint(id))?;
        let p = center(b);
        let dx = p.x - origin.x;
        let dy = p.y - origin.y;
        Some((
            if dx.abs() >= dy.abs() {
                if dx > 0.0 {
                    PortSide::Right
                } else {
                    PortSide::Left
                }
            } else if dy > 0.0 {
                PortSide::Bottom
            } else {
                PortSide::Top
            },
            p,
        ))
    };
    let (side, _) = face(&relation.to)?;
    let mut siblings: Vec<_> = diagram
        .relations
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            if r.from != relation.from || r.from == r.to {
                return None;
            }
            let (s, p) = face(&r.to)?;
            (s == side).then_some((
                i,
                match side {
                    PortSide::Left | PortSide::Right => p.y,
                    _ => p.x,
                },
            ))
        })
        .collect();
    if siblings.len() < 2 {
        return None;
    }
    siblings.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let mut rank = siblings.iter().position(|&(i, _)| i == index)?;
    if siblings.len() == 2
        && siblings.iter().all(|&(_, coordinate)| match side {
            PortSide::Left | PortSide::Right => coordinate > origin.y + node.height,
            _ => coordinate > origin.x + node.width,
        })
    {
        rank = siblings.len() - 1 - rank;
    }
    let span = match side {
        PortSide::Left | PortSide::Right => node.height,
        _ => node.width,
    };
    let step =
        (24.0 / (siblings.len() - 1) as f64).min((span - 12.0) / (siblings.len() - 1) as f64);
    if step < 8.0 {
        return None;
    }
    let offset = (rank as f64 - (siblings.len() - 1) as f64 / 2.0) * step;
    let mut p = forced_port(node, side);
    match side {
        PortSide::Left | PortSide::Right => p.y += offset,
        _ => p.x += offset,
    }
    Some(p)
}
