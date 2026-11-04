use super::{Equality, GeometryError, Minimum, err};

pub(super) fn solve_axis(
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
