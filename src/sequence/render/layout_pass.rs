//! Shared timeline pass for source-backed sequence layout constraints.
use super::{Message, SequenceDiagram, retime::Shiftables};
use crate::styles::StyleSheet;
pub(super) fn apply(
    diagram: &SequenceDiagram,
    positioned: &mut [(&Message, f64)],
    centers: &mut [f64],
    widths: &[f64],
    timeline: &mut Shiftables<'_, '_>,
    sheet: &StyleSheet,
) -> Result<(), String> {
    for region in &diagram.coregions {
        let (threshold, delta) =
            super::coregions::reorder_region(diagram, region, timeline.anchors)?;
        timeline.expand(region, threshold, delta);
    }
    super::activation_layout::apply(
        diagram,
        positioned,
        timeline.anchors,
        timeline.bars,
        timeline.open,
        timeline.positions,
        timeline.deaths,
    )?;
    if !diagram.occurrence_layout.is_empty() {
        super::destruction_layout::apply(
            diagram,
            timeline.deaths,
            timeline.positions,
            timeline.bars,
        )?;
    }
    super::occurrence_layout::apply(diagram, positioned, timeline)?;
    // Reserve horizontal self-call corridors before testing caption/note
    // collisions, rather than rejecting space that automatic lane spacing can give.
    super::routing::reserve(
        diagram,
        positioned,
        timeline.anchors,
        sheet,
        centers,
        widths,
    )?;
    let active = timeline
        .bars
        .iter()
        .map(|bar| super::DrawnExecution {
            opened: bar.opened.clone(),
            to: bar.to,
            end_line: bar.end_line,
        })
        .chain(
            timeline
                .open
                .iter()
                .cloned()
                .map(|bar| bar.finish(*timeline.final_y + 64.0, None)),
        )
        .collect::<Vec<_>>();
    super::bar_attachments::attach(
        diagram,
        sheet,
        centers,
        widths,
        &active,
        positioned,
        timeline.anchors,
    )?;
    super::reflow::message_text(diagram, positioned, centers, widths, timeline, sheet)?;
    if diagram.occurrence_layout.is_empty() {
        super::destruction_layout::apply(
            diagram,
            timeline.deaths,
            timeline.positions,
            timeline.bars,
        )?;
    }
    super::activation_layout::verify(diagram, timeline.bars, timeline.open)?;
    super::occurrence_layout::verify(diagram, positioned, timeline)?;
    super::coregions::validate_downward(timeline.anchors)
}
