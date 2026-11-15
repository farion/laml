//! Project independent anchor constraints through container boundaries.
use super::*;
use crate::syntax::Axis;

pub(super) fn project(
    diagram: &Diagram,
    boxes: &[BoxNode],
    parent: Option<&str>,
    graph: &mut Diagram,
) {
    for rule in &diagram.alignments {
        let (Some(first), Some(second)) = (
            direct_owner(diagram, &rule.first, parent),
            direct_owner(diagram, &rule.second, parent),
        ) else {
            continue;
        };
        if first == second {
            continue;
        }
        if first == rule.first && second == rule.second {
            graph.alignments.push(rule.clone());
            continue;
        }
        let node = |id: &str| {
            boxes
                .iter()
                .find(|node| node.id == id)
                .expect("validated alignment node")
        };
        let (a, b, oa, ob) = (
            node(&rule.first),
            node(&rule.second),
            node(&first),
            node(&second),
        );
        let (x, y) = rule.align.anchors();
        for (axis, anchor) in [(Axis::Horizontal, x), (Axis::Vertical, y)] {
            let Some(anchor) = anchor else {
                continue;
            };
            let coordinate = |node: &BoxNode| {
                if axis == Axis::Horizontal {
                    node.x
                } else {
                    node.y
                }
            };
            let extent = |node: &BoxNode| {
                if axis == Axis::Horizontal {
                    node.width
                } else {
                    node.height
                }
            };
            graph.scoped_alignments.push(ScopedAlignment {
                first: first.clone(),
                second: second.clone(),
                axis,
                delta: coordinate(a) - coordinate(oa) + anchor * extent(a) - coordinate(b)
                    + coordinate(ob)
                    - anchor * extent(b),
                line: rule.line,
            });
        }
    }
}

pub(super) fn validate(diagram: &Diagram, boxes: &[BoxNode]) -> Result<(), GeometryError> {
    for rule in &diagram.alignments {
        let node = |id: &str| {
            boxes
                .iter()
                .find(|node| node.id == id)
                .expect("visible alignment node")
        };
        let (a, b) = (node(&rule.first), node(&rule.second));
        let (x, y) = rule.align.anchors();
        let mismatch = x
            .is_some_and(|anchor| (a.x + a.width * anchor - b.x - b.width * anchor).abs() > 1e-4)
            || y.is_some_and(|anchor| {
                (a.y + a.height * anchor - b.y - b.height * anchor).abs() > 1e-4
            });
        if mismatch {
            return Err(err(
                rule.line,
                "cannot satisfy independent alignment across these container boundaries",
            ));
        }
    }
    Ok(())
}
