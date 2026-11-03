//! Per-message arrowheads use the same color as the styled message path.

use super::styles::escape;
use std::fmt::Write;

pub(super) fn draw(
    svg: &mut String,
    tip: (f64, f64),
    direction: (f64, f64),
    filled: bool,
    color: &str,
    modern: bool,
) {
    let length = direction.0.hypot(direction.1).max(0.001);
    let (ux, uy) = (direction.0 / length, direction.1 / length);
    let (x, y) = tip;
    let arrow_length = if modern { 12.0 } else { 10.0 };
    let half_width = if modern { 6.0 } else { 5.0 };
    let (back_x, back_y) = (x - ux * arrow_length, y - uy * arrow_length);
    let (one_x, one_y) = (back_x - uy * half_width, back_y + ux * half_width);
    let (two_x, two_y) = (back_x + uy * half_width, back_y - ux * half_width);
    if filled {
        writeln!(svg, "<path d=\"M{x:.2},{y:.2} L{one_x:.2},{one_y:.2} L{two_x:.2},{two_y:.2} Z\" fill=\"{}\" data-arrowhead=\"filled\" />", escape(color)).unwrap();
    } else {
        writeln!(svg, "<path d=\"M{one_x:.2},{one_y:.2} L{x:.2},{y:.2} L{two_x:.2},{two_y:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" data-arrowhead=\"open\" />", escape(color)).unwrap();
    }
}
