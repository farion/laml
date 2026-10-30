//! Coregion marker spans unordered occurrences on exactly one lifeline.

use super::{
    SequenceDiagram,
    anchors::{MessageAnchor, position},
    geometry::x_of,
    styles::{self, escape},
};
use crate::sequence::{AnnotationEnd, Coregion, coregions::order};
use crate::styles::StyleSheet;
use std::fmt::Write;

pub(super) fn reorder_region(
    diagram: &SequenceDiagram,
    region: &Coregion,
    anchors: &mut [MessageAnchor],
) -> Result<(f64, f64), String> {
    let mut baseline = (0..region.ends.len())
        .map(|i| {
            position(anchors, &region.ends[i].message, region.ends[i].end).map(|(_, y)| (i, y))
        })
        .collect::<Result<Vec<_>, _>>()?;
    baseline.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let baseline_indices: Vec<_> = baseline.iter().map(|(i, _)| *i).collect();
    let sorted = order(diagram, region, &baseline_indices).map_err(|e| e.message)?;
    let last_original = baseline.last().ok_or("empty coregion")?.1;
    let mut previous_y = f64::NEG_INFINITY;
    let mut placed = vec![None; region.ends.len()];
    for (rank, index) in sorted.into_iter().enumerate() {
        let end = &region.ends[index];
        let anchor = anchors
            .iter_mut()
            .find(|a| a.id == end.message)
            .ok_or_else(|| format!("unknown named message '{}'", end.message))?;
        let min_send_y = if end.end == AnnotationEnd::Receive {
            anchor.send.1
        } else {
            f64::NEG_INFINITY
        };
        let required = diagram
            .vertical_layout
            .iter()
            .filter(|rule| rule.below == *end)
            .filter_map(|rule| {
                region
                    .ends
                    .iter()
                    .position(|e| e == &rule.above)
                    .and_then(|i| placed[i].map(|y: f64| y + rule.gap))
            })
            .fold(f64::NEG_INFINITY, f64::max);
        let y = baseline[rank]
            .1
            .max(previous_y + 24.0)
            .max(min_send_y)
            .max(required);
        previous_y = y;
        placed[index] = Some(y);
        if end.end == AnnotationEnd::Send {
            anchor.send.1 = y;
        } else {
            anchor.receive.1 = y;
        }
    }
    Ok((last_original, (previous_y - last_original).max(0.0)))
}

pub(super) fn validate_downward(anchors: &[MessageAnchor]) -> Result<(), String> {
    if let Some(anchor) = anchors.iter().find(|a| a.receive.1 + 0.001 < a.send.1) {
        return Err(format!(
            "message '{}' would point upward after mandatory layout",
            anchor.id
        ));
    }
    Ok(())
}

pub(super) fn draw(
    svg: &mut String,
    diagram: &SequenceDiagram,
    centers: &[f64],
    anchors: &[MessageAnchor],
    sheet: &StyleSheet,
) -> Result<(), String> {
    for (index, region) in diagram.coregions.iter().enumerate() {
        let x = x_of(diagram, centers, &region.participant)?;
        let classes = &diagram
            .participants
            .iter()
            .find(|p| p.id == region.participant)
            .ok_or("unknown coregion participant")?
            .classes;
        let paint = styles::coregion(sheet, &region.participant, classes);
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        for end in &region.ends {
            let (_, at) = position(anchors, &end.message, end.end)?;
            low = low.min(at);
            high = high.max(at);
        }
        let top = low - 10.0;
        let bottom = high + 10.0;
        writeln!(svg, "<path d=\"M{:.2},{:.2} V{top:.2} H{:.2} v8 M{:.2},{:.2} V{bottom:.2} H{:.2} v-8\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" data-coregion=\"{}\" data-coregion-index=\"{index}\" />",
            x - 9.0, top + 8.0, x + 9.0, x - 9.0, bottom - 8.0, x + 9.0,
            escape(&paint.color), paint.width, escape(&region.participant)).unwrap();
    }
    Ok(())
}
