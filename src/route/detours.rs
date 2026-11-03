//! Bounded obstacle-edge corridors; tried after the stable direct routes.
use super::{
    Point,
    geometry::{center, port},
};
use crate::geometry::BoxNode;

fn tracks(boxes: &[BoxNode], horizontal: bool, middle: f64, gap_midpoints: bool) -> Vec<f64> {
    let mut values = Vec::new();
    for node in boxes {
        let (low, high) = if horizontal {
            (node.x, node.x + node.width)
        } else {
            (node.y, node.y + node.height)
        };
        values.extend([low - 24.0, high + 24.0]);
    }
    // A route can pass between two disjoint boxes even if another box,
    // elsewhere on the diagram, overlaps their gap on this axis. Keep the
    // midpoint as a candidate and let the full obstacle check decide.
    if gap_midpoints {
        for (i, first) in boxes
            .iter()
            .enumerate()
            .filter(|(_, node)| !node.is_container)
        {
            for second in boxes.iter().skip(i + 1).filter(|node| !node.is_container) {
                let (first_low, first_high, second_low, second_high) = if horizontal {
                    (
                        first.x,
                        first.x + first.width,
                        second.x,
                        second.x + second.width,
                    )
                } else {
                    (
                        first.y,
                        first.y + first.height,
                        second.y,
                        second.y + second.height,
                    )
                };
                for (end, start) in [(first_high, second_low), (second_high, first_low)] {
                    if start - end > 8.0 {
                        values.push((end + start) / 2.0);
                    }
                }
            }
        }
    }
    values.sort_by(|a: &f64, b: &f64| {
        (*a - middle)
            .abs()
            .total_cmp(&(*b - middle).abs())
            .then(a.total_cmp(b))
    });
    values.dedup_by(|a, b| (*a - *b).abs() < 1.0);
    values.truncate(12);
    values
}

pub(super) fn corridors(
    a: &BoxNode,
    b: &BoxNode,
    boxes: &[BoxNode],
    gap_midpoints: bool,
) -> Vec<Vec<Point>> {
    let (ca, cb) = (center(a), center(b));
    let xs = tracks(boxes, true, (ca.x + cb.x) / 2.0, gap_midpoints);
    let ys = tracks(boxes, false, (ca.y + cb.y) / 2.0, gap_midpoints);
    let mut result = Vec::new();
    for &x in &xs {
        let (wa, wb) = (Point { x, y: ca.y }, Point { x, y: cb.y });
        result.push(vec![port(a, wa), wa, wb, port(b, wb)]);
    }
    for &y in &ys {
        let (wa, wb) = (Point { x: ca.x, y }, Point { x: cb.x, y });
        result.push(vec![port(a, wa), wa, wb, port(b, wb)]);
    }
    // The two-corridor candidates step around an obstacle that occupies the
    // direct line without forcing a long global detour around the whole graph.
    for &x in &xs {
        for &y in &ys {
            let start = Point { x, y: ca.y };
            let end = Point { x: cb.x, y };
            result.push(vec![
                port(a, start),
                start,
                Point { x, y },
                end,
                port(b, end),
            ]);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn obstacle_edge_tracks_offer_a_safe_two_turn_corridor() {
        let diagram = crate::parser::parse(
            "semantics {\nA\nB\nC\n@r A -- C\n}\nlayout {\nA < B : align=center; gap=90\nB < C : align=center; gap=90\n}\n",
        )
        .unwrap();
        let boxes =
            crate::geometry::place(&diagram, &crate::styles::StyleSheet::default()).unwrap();
        assert!(
            corridors(&boxes[0], &boxes[2], &boxes, false)
                .iter()
                .any(|path| {
                    path.len() == 5 && super::super::geometry::route_is_clear(path, &boxes, &[])
                })
        );
    }
}
