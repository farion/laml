//! Independent anchor equalities and narrowly scoped intentional center overlap.
use super::{BoxNode, Equality};
use crate::syntax::Diagram;

pub(super) fn apply(
    diagram: &Diagram,
    boxes: &[BoxNode],
    x: &mut Vec<Equality>,
    y: &mut Vec<Equality>,
) -> Vec<(usize, usize)> {
    let mut center_x: Vec<_> = (0..boxes.len()).collect();
    let mut center_y = center_x.clone();
    for rule in &diagram.alignments {
        let a = boxes
            .iter()
            .position(|node| node.id == rule.first)
            .expect("validated alignment node");
        let b = boxes
            .iter()
            .position(|node| node.id == rule.second)
            .expect("validated alignment node");
        let (ax, ay) = rule.align.anchors();
        if let Some(anchor) = ax {
            x.push(Equality {
                a,
                b,
                delta: anchor * (boxes[a].width - boxes[b].width),
                line: rule.line,
            });
            if anchor == 0.5 {
                union(&mut center_x, a, b);
            }
        }
        if let Some(anchor) = ay {
            y.push(Equality {
                a,
                b,
                delta: anchor * (boxes[a].height - boxes[b].height),
                line: rule.line,
            });
            if anchor == 0.5 {
                union(&mut center_y, a, b);
            }
        }
    }
    let mut allowed = Vec::new();
    for a in 0..boxes.len() {
        for b in a + 1..boxes.len() {
            // Both centers must be explicitly connected. One-axis alignment never
            // disables collision handling, and no third-party node gets a waiver.
            if root(&center_x, a) == root(&center_x, b) && root(&center_y, a) == root(&center_y, b)
            {
                allowed.push((a, b));
            }
        }
    }
    allowed
}
fn root(parents: &[usize], mut i: usize) -> usize {
    while parents[i] != i {
        i = parents[i];
    }
    i
}
fn union(parents: &mut [usize], a: usize, b: usize) {
    let a = root(parents, a);
    let b = root(parents, b);
    parents[b] = a;
}
