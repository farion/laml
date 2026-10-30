//! State-shaped continuation symbol spanning explicitly placed lifelines.

use super::{
    SequenceDiagram,
    geometry::{label_width_at, x_of},
    styles::{self, escape},
};
use crate::sequence::Continuation;
use crate::styles::StyleSheet;
use std::fmt::Write;

pub(super) fn height(cont: &Continuation, sheet: &StyleSheet) -> f64 {
    let size = styles::continuation(sheet, &cont.id).font_size;
    (size * 1.25 * cont.text.split('\n').count() as f64 + 20.0).max(38.0)
}

pub(super) fn draw(
    svg: &mut String,
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    centers: &[f64],
    positions: &[(&Continuation, f64)],
) -> Result<(), String> {
    for (cont, top) in positions {
        let placement = diagram
            .continuation_layout
            .iter()
            .find(|p| p.id == cont.id)
            .ok_or_else(|| format!("continuation '{}' needs layout", cont.id))?;
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        for id in &placement.participants {
            let x = x_of(diagram, centers, id)?;
            low = low.min(x);
            high = high.max(x);
        }
        let paint = styles::continuation(sheet, &cont.id);
        let width = (high - low + 40.0).max(label_width_at(&cont.text, paint.font_size)? + 32.0);
        let x = (low + high - width) / 2.0;
        let height = height(cont, sheet);
        writeln!(svg, "<rect x=\"{x:.2}\" y=\"{top:.2}\" width=\"{width:.2}\" height=\"{height:.2}\" rx=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" data-continuation=\"{}\" />",
            height / 2.0, escape(&paint.fill), escape(&paint.border), paint.border_width, escape(&cont.id)).unwrap();
        for (index, line) in cont.text.split('\n').enumerate() {
            writeln!(svg, "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\">{}</text>",
                (low + high) / 2.0, top + 11.0 + paint.font_size
                    + index as f64 * paint.font_size * 1.25,
                paint.font_size, escape(&paint.color), escape(line)).unwrap();
        }
    }
    Ok(())
}
