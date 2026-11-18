//! Inline subdiagrams are rendered from the same source without opening files.

use super::{SequenceDiagram, styles::escape};
use crate::{styles::StyleSheet, svg::TextMode};
use std::fmt::Write;

pub(super) struct Prepared {
    pub svg: String,
    pub width: f64,
    pub height: f64,
    pub regions: Vec<crate::authoring::Region>,
}

fn dimension(svg: &str, attribute: &str) -> Result<f64, String> {
    let first = svg.lines().next().ok_or("empty subdiagram SVG")?;
    first
        .split(&format!(" {attribute}=\""))
        .nth(1)
        .and_then(|value| value.split('"').next())
        .ok_or_else(|| format!("missing SVG {attribute}"))?
        .parse::<f64>()
        .map_err(|_| format!("invalid SVG {attribute}"))
}

pub(super) fn prepare(
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
) -> Result<Vec<Prepared>, String> {
    diagram
        .decompositions
        .iter()
        .map(|part| {
            let classes = &diagram
                .participants
                .iter()
                .find(|p| p.id == part.parent)
                .ok_or("unknown decomposed parent")?
                .classes;
            let child_sheet = sheet.with_ancestor(&part.parent, classes);
            let (svg, regions) =
                super::render_for_editor(&part.diagram, &child_sheet, TextMode::System)?;
            Ok(Prepared {
                width: dimension(&svg, "width")?,
                height: dimension(&svg, "height")?,
                svg,
                regions,
            })
        })
        .collect()
}

pub(super) fn draw(
    svg: &mut String,
    diagram: &SequenceDiagram,
    centers: &[f64],
    rendered: &[Prepared],
    positions: &[(usize, f64)],
) -> Result<(), String> {
    for &(index, top) in positions {
        let part = diagram
            .decompositions
            .get(index)
            .ok_or("missing decomposition")?;
        let panel = rendered
            .get(index)
            .ok_or("missing rendered decomposition")?;
        let parent = diagram
            .participants
            .iter()
            .position(|p| p.id == part.parent)
            .ok_or("unknown decomposed parent")?;
        let left = centers[parent] - panel.width / 2.0;
        writeln!(svg, "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"white\" stroke=\"none\" data-decomposition=\"{}\" />",
            left - 8.0, top - 8.0, panel.width + 16.0, panel.height + 16.0,
            escape(&part.parent)).unwrap();
        let nested =
            panel
                .svg
                .replacen("<svg ", &format!("<svg x=\"{left:.2}\" y=\"{top:.2}\" "), 1);
        svg.push_str(&nested);
    }
    Ok(())
}

/// Account for routed child extents, including nested self-message corridors.
pub(super) fn fit_widths(
    diagram: &SequenceDiagram,
    prepared: &[Prepared],
    centers: &mut [f64],
    widths: &mut [f64],
) {
    let mut extra = vec![0.0_f64; widths.len()];
    for (part, child) in diagram.decompositions.iter().zip(prepared) {
        if let Some(i) = diagram
            .participants
            .iter()
            .position(|p| p.id == part.parent)
        {
            extra[i] = extra[i].max((child.width + 32.0 - widths[i]).max(0.0));
        }
    }
    let mut shift = 0.0;
    for i in 0..widths.len() {
        centers[i] += shift + extra[i] / 2.0;
        widths[i] += extra[i];
        shift += extra[i];
    }
}
