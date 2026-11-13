//! Deterministic bounded alternatives to committing ports in declaration order.
use crate::{geometry::BoxNode, syntax::Diagram};

pub(super) fn alternatives(diagram: &Diagram, boxes: &[BoxNode]) -> Vec<Vec<usize>> {
    let n = diagram.relations.len();
    let original: Vec<_> = (0..n).collect();
    let mut orders = vec![original.iter().copied().rev().collect::<Vec<_>>()];
    let owner = |id: &str| boxes.iter().position(|b| b.has_endpoint(id)).unwrap();
    let center = |id: &str| {
        let b = &boxes[owner(id)];
        (b.x + b.width / 2.0, b.y + b.height / 2.0)
    };
    // Evaluate both orders of sibling connections. This changes the available
    // departure slots without changing semantic or returned relation order.
    for source in 0..n {
        let siblings: Vec<_> = original
            .iter()
            .copied()
            .filter(|&i| diagram.relations[i].from == diagram.relations[source].from)
            .collect();
        if siblings.len() < 2 || siblings[0] != source {
            continue;
        }
        let mut spatial = siblings.clone();
        spatial.sort_by(|&a, &b| {
            let ca = center(&diagram.relations[a].to);
            let cb = center(&diagram.relations[b].to);
            ca.0.total_cmp(&cb.0)
                .then(ca.1.total_cmp(&cb.1))
                .then(a.cmp(&b))
        });
        for permutation in [spatial.clone(), spatial.into_iter().rev().collect()] {
            let mut order = original.clone();
            for (&slot, edge) in siblings.iter().zip(permutation) {
                order[slot] = edge;
            }
            if order != original && !orders.contains(&order) {
                orders.push(order);
            }
        }
        if orders.len() >= 7 {
            break;
        }
    }
    let mut short_first = original.clone();
    short_first.sort_by(|&a, &b| {
        let distance = |i: usize| {
            let r = &diagram.relations[i];
            let (x, y) = center(&r.from);
            let (u, v) = center(&r.to);
            (x - u).abs() + (y - v).abs()
        };
        distance(a).total_cmp(&distance(b)).then(a.cmp(&b))
    });
    for order in [short_first.clone(), short_first.into_iter().rev().collect()] {
        if order != original && !orders.contains(&order) {
            orders.push(order);
        }
    }
    // Long connections dominate the available corridors. Rotate their joint
    // allocation in both directions instead of changing only one source.
    // This remains bounded rather than growing factorially with diagram size.
    let distance = |i: usize| {
        let r = &diagram.relations[i];
        let (x, y) = center(&r.from);
        let (u, v) = center(&r.to);
        (x - u).abs() + (y - v).abs()
    };
    let longest = original
        .iter()
        .map(|&i| distance(i))
        .fold(0.0_f64, f64::max);
    let long: Vec<_> = original
        .iter()
        .copied()
        .filter(|&i| distance(i) > longest * 0.75)
        .take(4)
        .collect();
    if long.len() >= 2 {
        let mut permutations_out = Vec::new();
        for mut direction in [long.clone(), long.iter().copied().rev().collect()] {
            for _ in 0..long.len() {
                permutations_out.push(direction.clone());
                direction.rotate_left(1);
            }
        }
        for permutation in permutations_out {
            let mut order = original.clone();
            for (&slot, edge) in long.iter().zip(permutation) {
                order[slot] = edge;
            }
            if order != original && !orders.contains(&order) {
                orders.push(order);
            }
        }
    }
    orders
}
