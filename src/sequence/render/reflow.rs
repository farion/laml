//! Make room between independently ordered message text, notes and continuations.
use super::{Message, retime::Shiftables, text_collision};
use crate::sequence::{Coregion, SequenceDiagram};
use crate::styles::StyleSheet;

pub(super) fn message_text(
    diagram: &SequenceDiagram,
    positioned: &[(&Message, f64)],
    centers: &[f64],
    widths: &[f64],
    timeline: &mut Shiftables<'_, '_>,
    sheet: &StyleSheet,
) -> Result<(), String> {
    let empty = Coregion {
        participant: String::new(),
        ends: Vec::new(),
        line: 0,
    };
    let limit = (positioned.len() + timeline.notes.len() + timeline.continuations.len())
        .saturating_mul(4)
        .max(1);
    for _ in 0..limit {
        let bounds = text_collision::labeled_messages(positioned, timeline.anchors, sheet)?;
        let messages = (0..bounds.len()).find_map(|later| {
            (0..later)
                .find(|earlier| bounds[*earlier].1.overlaps(bounds[later].1))
                .map(|earlier| (earlier, later))
        });
        if let Some((earlier, later)) = messages {
            let before = &timeline.anchors[bounds[earlier].0];
            let after = &timeline.anchors[bounds[later].0];
            let threshold = before.send.1.max(before.receive.1);
            let next = after.send.1.min(after.receive.1);
            let label = positioned[bounds[later].0]
                .0
                .name
                .as_deref()
                .unwrap_or("unnamed");
            if next <= threshold + 0.01 {
                return Err(format!(
                    "message '{label}' overlaps an earlier message label at the same occurrence row; put the messages on separate rows"
                ));
            }
            let distance =
                bounds[earlier].1.y + bounds[earlier].1.height - bounds[later].1.y + 12.0;
            timeline.expand(&empty, threshold, distance.max(12.0));
            continue;
        }
        let mut shapes =
            text_collision::frame_regions(diagram, sheet, centers, widths, timeline.frames)?;
        shapes.extend(text_collision::note_regions(
            diagram,
            timeline.anchors,
            centers,
            timeline.notes,
            timeline.continuations,
            sheet,
        )?);
        let overlap = bounds.iter().find_map(|(index, rect)| {
            shapes
                .iter()
                .find(|shape| rect.overlaps(**shape))
                .map(|shape| (*index, *rect, *shape))
        });
        let Some((index, label, shape)) = overlap else {
            return Ok(());
        };
        let anchor = &timeline.anchors[index];
        let (early, late) = (
            anchor.send.1.min(anchor.receive.1),
            anchor.send.1.max(anchor.receive.1),
        );
        let (threshold, distance) = if shape.y > late + 0.01 {
            (late, label.y + label.height - shape.y + 12.0)
        } else if early > shape.y + 0.01 {
            (shape.y, shape.y + shape.height - label.y + 12.0)
        } else {
            let id = positioned[index].0.name.as_deref().unwrap_or("unnamed");
            return Err(format!(
                "message '{id}' overlaps a frame title, note or continuation at the same occurrence; move that element to another row"
            ));
        };
        timeline.expand(&empty, threshold, distance.max(12.0));
    }
    Err("cannot separate message text from nearby labels or notes".into())
}
