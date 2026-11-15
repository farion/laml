//! Additional spacing is solved with, never substituted for, authored constraints.
use super::{BoxNode, Equality, GeometryError, Minimum, err, solve_axis};

#[allow(clippy::too_many_arguments)]
pub(super) fn separate(
    boxes: &[BoxNode],
    a: usize,
    b: usize,
    gap: f64,
    xs: &[f64],
    ys: &[f64],
    eq_x: &[Equality],
    eq_y: &[Equality],
    x: &mut Vec<Minimum>,
    y: &mut Vec<Minimum>,
    pref_x: &[f64],
    pref_y: &[f64],
) -> Result<(), GeometryError> {
    let mut choices = Vec::new();
    for (a, b) in [(a, b), (b, a)] {
        choices.push((xs[a] + boxes[a].width + gap - xs[b], true, a, b));
        choices.push((ys[a] + boxes[a].height + gap - ys[b], false, a, b));
    }
    choices.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (_, horizontal, a, b) in choices {
        let (bounds, equalities, pref, extent) = if horizontal {
            (&mut *x, eq_x, pref_x, boxes[a].width)
        } else {
            (&mut *y, eq_y, pref_y, boxes[a].height)
        };
        bounds.push(Minimum {
            a,
            b,
            distance: extent + gap,
            line: 0,
        });
        if solve_axis(boxes.len(), equalities, bounds, pref).is_ok() {
            return Ok(());
        }
        bounds.pop();
    }
    Err(err(
        boxes[b].line,
        "additional routing space conflicts with mandatory layout",
    ))
}
