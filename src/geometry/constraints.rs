use super::nodes::BoxNode;
use crate::styles::{Element, StyleSheet, Value};
use crate::syntax::{Alignment, Diagram, FlowDirection, Placement};
use crate::text::measure;

#[derive(Clone, Debug)]
pub struct GeometryError {
    pub line: usize,
    pub message: String,
}

#[derive(Clone)]
struct Equality {
    a: usize,
    b: usize,
    delta: f64,
    line: usize,
}
#[derive(Clone)]
struct Minimum {
    a: usize,
    b: usize,
    distance: f64,
    line: usize,
}

fn err(line: usize, message: impl Into<String>) -> GeometryError {
    GeometryError {
        line,
        message: message.into(),
    }
}

fn solve_axis(
    count: usize,
    equal: &[Equality],
    minimum: &[Minimum],
    preferred: &[f64],
) -> Result<Vec<f64>, GeometryError> {
    let mut adjacency = vec![Vec::<(usize, f64, usize)>::new(); count];
    for eq in equal {
        adjacency[eq.a].push((eq.b, eq.delta, eq.line));
        adjacency[eq.b].push((eq.a, -eq.delta, eq.line));
    }
    let mut root = vec![usize::MAX; count];
    let mut offset = vec![0.0; count];
    let mut groups = 0;
    for start in 0..count {
        if root[start] != usize::MAX {
            continue;
        }
        root[start] = groups;
        let mut stack = vec![start];
        while let Some(a) = stack.pop() {
            for &(b, delta, line) in &adjacency[a] {
                let proposed = offset[a] + delta;
                if root[b] == usize::MAX {
                    root[b] = groups;
                    offset[b] = proposed;
                    stack.push(b);
                } else if (offset[b] - proposed).abs() > 1e-6 {
                    return Err(err(line, "incompatible mandatory alignments"));
                }
            }
        }
        groups += 1;
    }
    let mut values = vec![0.0_f64; groups];
    let mut initialized = vec![false; groups];
    for i in 0..count {
        if !initialized[root[i]] {
            values[root[i]] = preferred[i] - offset[i];
            initialized[root[i]] = true;
        }
    }
    for iteration in 0..groups {
        let mut changed = false;
        for bound in minimum {
            let needed = bound.distance + offset[bound.a] - offset[bound.b];
            if root[bound.a] == root[bound.b] {
                if needed > 1e-6 {
                    return Err(err(
                        bound.line,
                        "mandatory alignment conflicts with separation",
                    ));
                }
                continue;
            }
            let candidate = values[root[bound.a]] + needed;
            if candidate > values[root[bound.b]] + 1e-6 {
                values[root[bound.b]] = candidate;
                changed = true;
                if iteration + 1 == groups {
                    return Err(err(
                        bound.line,
                        "impossible cycle of mandatory placement rules",
                    ));
                }
            }
        }
        if !changed {
            break;
        }
    }
    Ok((0..count).map(|i| values[root[i]] + offset[i]).collect())
}

fn try_soft(
    count: usize,
    equal: &[Equality],
    bounds: &mut Vec<Minimum>,
    candidate: Minimum,
    preferred: &[f64],
) {
    bounds.push(candidate);
    if solve_axis(count, equal, bounds, preferred).is_err() {
        bounds.pop();
    }
}

pub fn place(
    diagram: &Diagram,
    boxes: &mut [BoxNode],
    styles: &StyleSheet,
) -> Result<(), GeometryError> {
    let n = boxes.len();
    // The preferred centre is independent of the measured size. Expansion
    // grows around that centre; later nodes are displaced only as needed.
    let pref_x: Vec<f64> = boxes.iter().map(|b| 96.0 - b.width / 2.0).collect();
    let pref_y: Vec<f64> = boxes.iter().map(|b| 96.0 - b.height / 2.0).collect();
    let mut eq_x = Vec::new();
    let mut eq_y = Vec::new();
    let mut x = Vec::new();
    let mut y = Vec::new();
    let find = |id: &str| {
        boxes
            .iter()
            .position(|b| b.id == id)
            .expect("parsed diagram has checked node IDs")
    };
    let owner = |id: &str| {
        boxes
            .iter()
            .position(|b| b.has_endpoint(id))
            .expect("parsed diagram has checked member IDs")
    };
    for rule in &diagram.layout {
        let a = find(&rule.first);
        let b = find(&rule.second);
        let mut gap = rule.gap.unwrap_or(24.0);
        // A minimum gap may grow to accommodate a nearby relation label
        // and its arrowhead. Never shrink an explicit minimum.
        for relation in &diagram.relations {
            if !((relation.from == rule.first && relation.to == rule.second)
                || (relation.from == rule.second && relation.to == rule.first))
            {
                continue;
            }
            let shown_label = relation.display_label();
            let Some(label) = shown_label.as_deref() else {
                continue;
            };
            let element = Element::Relation {
                name: relation.name.as_deref(),
                classes: &relation.classes,
            };
            let size = match styles.get(&element, "font-size").map(|d| &d.value) {
                Some(Value::Number(n)) => *n,
                _ => 14.0,
            };
            let clearance = if matches!(rule.placement, Placement::Left | Placement::Right) {
                measure(label, size).map_err(|message| err(relation.line, message))? + 40.0
            } else {
                size * 1.25 + 48.0
            };
            gap = gap.max(clearance);
        }
        match rule.placement {
            Placement::Left => x.push(Minimum {
                a,
                b,
                distance: boxes[a].width + gap,
                line: rule.line,
            }),
            Placement::Right => x.push(Minimum {
                a: b,
                b: a,
                distance: boxes[b].width + gap,
                line: rule.line,
            }),
            Placement::Above => y.push(Minimum {
                a,
                b,
                distance: boxes[a].height + gap,
                line: rule.line,
            }),
            Placement::Below => y.push(Minimum {
                a: b,
                b: a,
                distance: boxes[b].height + gap,
                line: rule.line,
            }),
        }
        if let Some(align) = rule.align {
            let horizontal = matches!(rule.placement, Placement::Left | Placement::Right);
            let delta = match align {
                Alignment::Top | Alignment::Left => 0.0,
                Alignment::Center if horizontal => (boxes[a].height - boxes[b].height) / 2.0,
                Alignment::Center => (boxes[a].width - boxes[b].width) / 2.0,
                Alignment::Bottom => boxes[a].height - boxes[b].height,
                Alignment::Right => boxes[a].width - boxes[b].width,
            };
            let eq = Equality {
                a,
                b,
                delta,
                line: rule.line,
            };
            if horizontal {
                eq_y.push(eq);
            } else {
                eq_x.push(eq);
            }
        }
    }
    // Only explicit constraints may cause a hard failure at this point.
    solve_axis(n, &eq_x, &x, &pref_x)?;
    solve_axis(n, &eq_y, &y, &pref_y)?;

    if diagram.relations.is_empty() && diagram.layout.is_empty() {
        for i in 1..n {
            y.push(Minimum {
                a: i - 1,
                b: i,
                distance: boxes[i - 1].height + 24.0,
                line: 0,
            });
        }
    } else {
        // Preferences are accepted only when consistent with all hard rules.
        let relations: Vec<_> = diagram
            .relations
            .iter()
            .map(|r| crate::Relation {
                from: boxes[owner(&r.from)].id.clone(),
                to: boxes[owner(&r.to)].id.clone(),
                kind: r.kind,
            })
            .collect();
        let nodes: Vec<_> = boxes
            .iter()
            .map(|b| crate::Node { id: b.id.clone() })
            .collect();
        let preferences = crate::resolve_direction_preferences(&nodes, &relations)
            .expect("parsed diagram has checked relation endpoints");
        for index in preferences.accepted {
            let relation = &diagram.relations[index];
            let a = owner(&relation.from);
            let b = owner(&relation.to);
            let distance = if matches!(
                diagram.settings.direction,
                FlowDirection::LeftToRight | FlowDirection::RightToLeft
            ) {
                boxes[a].width + 96.0
            } else {
                boxes[a].height + 64.0
            };
            let reverse = matches!(
                diagram.settings.direction,
                FlowDirection::RightToLeft | FlowDirection::BottomToTop
            );
            let (from, to, distance) = if reverse {
                let extent = if matches!(diagram.settings.direction, FlowDirection::RightToLeft) {
                    boxes[b].width
                } else {
                    boxes[b].height
                };
                (b, a, extent + 64.0)
            } else {
                (a, b, distance)
            };
            let bound = Minimum {
                a: from,
                b: to,
                distance,
                line: 0,
            };
            if matches!(
                diagram.settings.direction,
                FlowDirection::LeftToRight | FlowDirection::RightToLeft
            ) {
                try_soft(n, &eq_x, &mut x, bound, &pref_x);
            } else {
                try_soft(n, &eq_y, &mut y, bound, &pref_y);
            }
        }
        // An unheaded edge has no semantic direction. Source node order is
        // only a deterministic proximity tie-break, never a hard constraint.
        for relation in &diagram.relations {
            if !matches!(
                relation.kind,
                crate::RelationKind::Undirected | crate::RelationKind::Bidirectional
            ) {
                continue;
            }
            let (a, b) = (owner(&relation.from), owner(&relation.to));
            if a == b {
                continue;
            }
            let (first, second) = (a.min(b), a.max(b));
            let horizontal = matches!(
                diagram.settings.direction,
                FlowDirection::LeftToRight | FlowDirection::RightToLeft
            );
            let extent = if horizontal {
                boxes[first].width
            } else {
                boxes[first].height
            };
            let bound = Minimum {
                a: first,
                b: second,
                distance: extent + 64.0,
                line: 0,
            };
            if horizontal {
                try_soft(n, &eq_x, &mut x, bound, &pref_x);
            } else {
                try_soft(n, &eq_y, &mut y, bound, &pref_y);
            }
        }
    }
    // Preserve earlier node positions where possible: move the later node's
    // alignment group to clear collisions. If Y is impossible, try X.
    for _ in 0..n.saturating_mul(n).saturating_mul(4).max(1) {
        let xs = solve_axis(n, &eq_x, &x, &pref_x)?;
        let ys = solve_axis(n, &eq_y, &y, &pref_y)?;
        let collision = (0..n)
            .flat_map(|a| (a + 1..n).map(move |b| (a, b)))
            .find(|&(a, b)| {
                xs[a] < xs[b] + boxes[b].width - 1e-6
                    && xs[b] < xs[a] + boxes[a].width - 1e-6
                    && ys[a] < ys[b] + boxes[b].height - 1e-6
                    && ys[b] < ys[a] + boxes[a].height - 1e-6
            });
        if let Some((a, b)) = collision {
            let candidate = Minimum {
                a,
                b,
                distance: boxes[a].height + 24.0,
                line: boxes[b].line,
            };
            y.push(candidate);
            if solve_axis(n, &eq_y, &y, &pref_y).is_err() {
                y.pop();
                x.push(Minimum {
                    a,
                    b,
                    distance: boxes[a].width + 24.0,
                    line: boxes[b].line,
                });
                if solve_axis(n, &eq_x, &x, &pref_x).is_err() {
                    return Err(err(
                        boxes[b].line,
                        "cannot separate colliding nodes without violating mandatory layout",
                    ));
                }
            }
        } else {
            for (i, node) in boxes.iter_mut().enumerate() {
                node.x = xs[i];
                node.y = ys[i];
            }
            return Ok(());
        }
    }
    Err(err(0, "bounded collision resolver did not converge"))
}
