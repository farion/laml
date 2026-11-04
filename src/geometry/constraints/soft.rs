use super::{Equality, Minimum, solve_axis};

pub(super) fn try_soft(
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
