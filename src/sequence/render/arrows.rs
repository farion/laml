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
    let [(x, y), (one_x, one_y), (two_x, two_y)] =
        super::message_geometry::arrowhead(tip, direction, modern);
    if filled {
        writeln!(svg, "<path d=\"M{x:.2},{y:.2} L{one_x:.2},{one_y:.2} L{two_x:.2},{two_y:.2} Z\" fill=\"{}\" data-arrowhead=\"filled\" />", escape(color)).unwrap();
    } else {
        writeln!(svg, "<path d=\"M{one_x:.2},{one_y:.2} L{x:.2},{y:.2} L{two_x:.2},{two_y:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" data-arrowhead=\"open\" />", escape(color)).unwrap();
    }
}
