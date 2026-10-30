//! Stable source-order lanes for multiple relations sharing an unordered pair.

use super::{Point, Route, center, port, self_loops, style};
use crate::geometry::BoxNode;
use crate::styles::StyleSheet;
use crate::syntax::Diagram;

pub(super) fn offset(
    diagram: &Diagram,
    relation_index: usize,
    boxes: &[BoxNode],
    a: usize,
    b: usize,
    styles: &StyleSheet,
) -> Result<f64, String> {
    let relation = &diagram.relations[relation_index];
    if !(relation.from == boxes[a].id || relation.from == boxes[b].id)
        || !(relation.to == boxes[a].id || relation.to == boxes[b].id)
    {
        // A member's address is not a node ID; it keeps its row anchor.
        return Ok(0.0);
    }
    let mut siblings = Vec::new();
    let mut widest = 0.0_f64;
    for (index, rel) in diagram.relations.iter().enumerate() {
        let same = (rel.from == boxes[a].id && rel.to == boxes[b].id)
            || (rel.from == boxes[b].id && rel.to == boxes[a].id);
        if same {
            siblings.push(index);
            widest = widest.max(style(rel, styles).1);
        }
    }
    if siblings.len() == 1 {
        return Ok(0.0);
    }
    let rank = siblings
        .iter()
        .position(|&i| i == relation_index)
        .expect("relation in its pair");
    let side_span = |node: &BoxNode, target: &BoxNode| {
        let boundary = port(node, center(target));
        if (boundary.x - node.x).abs() < 1e-6 || (boundary.x - node.x - node.width).abs() < 1e-6 {
            node.height
        } else {
            node.width
        }
    };
    let span = side_span(&boxes[a], &boxes[b]).min(side_span(&boxes[b], &boxes[a]));
    let available = span - 12.0;
    let step = 12.0_f64.min(available / (siblings.len() - 1) as f64);
    if step < widest + 6.0 || step < 9.0 {
        return Err(format!(
            "{} parallel relations cannot fit distinct ports on both node faces",
            siblings.len()
        ));
    }
    Ok((rank as f64 - (siblings.len() - 1) as f64 / 2.0) * step)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Face {
    Left,
    Right,
    Top,
    Bottom,
}

fn face(node: &BoxNode, point: Point) -> Option<Face> {
    if (point.x - node.x).abs() < 1e-5 {
        Some(Face::Left)
    } else if (point.x - node.x - node.width).abs() < 1e-5 {
        Some(Face::Right)
    } else if (point.y - node.y).abs() < 1e-5 {
        Some(Face::Top)
    } else if (point.y - node.y - node.height).abs() < 1e-5 {
        Some(Face::Bottom)
    } else {
        None
    }
}

/// A pair's lane rank only reserves a port on faces used by other arrows.
/// An otherwise unused top or bottom face gets its centred port back.
pub(super) fn has_competing_port(
    diagram: &Diagram,
    index: usize,
    boxes: &[BoxNode],
    node: usize,
    candidate: Point,
    previous: &[Route],
) -> bool {
    let Some(side) = face(&boxes[node], candidate) else {
        return false;
    };
    if previous.iter().any(|route| {
        (route.from == node && face(&boxes[node], route.points[0]) == Some(side))
            || (route.to == node && face(&boxes[node], *route.points.last().unwrap()) == Some(side))
    }) {
        return true;
    }
    diagram
        .relations
        .iter()
        .enumerate()
        .any(|(other_index, rel)| {
            if other_index == index {
                return false;
            }
            if rel.from == boxes[node].id && rel.to == boxes[node].id {
                let rank = diagram.relations[..other_index]
                    .iter()
                    .filter(|r| r.from == rel.from && r.to == rel.to)
                    .count();
                if rank >= 4 {
                    return false;
                }
                let preferred = &self_loops::candidates(&boxes[node], rank)[0];
                return [preferred[0], *preferred.last().unwrap()]
                    .iter()
                    .any(|&p| face(&boxes[node], p) == Some(side));
            }
            let other = if rel.from == boxes[node].id {
                &rel.to
            } else if rel.to == boxes[node].id {
                &rel.from
            } else {
                return false;
            };
            boxes
                .iter()
                .find(|b| b.id == *other)
                .is_some_and(|b| face(&boxes[node], port(&boxes[node], center(b))) == Some(side))
        })
}

pub(super) fn shift_port(node: &BoxNode, point: Point, offset: f64) -> Option<Point> {
    if offset.abs() < 1e-9 {
        return Some(point);
    }
    let vertical_side =
        (point.x - node.x).abs() < 1e-5 || (point.x - (node.x + node.width)).abs() < 1e-5;
    if vertical_side {
        let y = point.y + offset;
        if y < node.y + 6.0 || y > node.y + node.height - 6.0 {
            return None;
        }
        Some(Point { x: point.x, y })
    } else {
        let horizontal_side =
            (point.y - node.y).abs() < 1e-5 || (point.y - (node.y + node.height)).abs() < 1e-5;
        if !horizontal_side {
            return None;
        }
        let x = point.x + offset;
        if x < node.x + 6.0 || x > node.x + node.width - 6.0 {
            return None;
        }
        Some(Point { x, y: point.y })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::place;
    use crate::parser::parse;
    #[test]
    fn two_parallel_relations_get_different_source_order_lanes() {
        let diagram = parse("semantics {\nA\nB\nA -> B\nB -> A\n}\n").unwrap();
        let styles = StyleSheet::default();
        let boxes = place(&diagram, &styles).unwrap();
        assert_eq!(offset(&diagram, 1, &boxes, 0, 1, &styles).unwrap(), 6.0);
        assert_eq!(offset(&diagram, 0, &boxes, 0, 1, &styles).unwrap(), -6.0);
    }
}
