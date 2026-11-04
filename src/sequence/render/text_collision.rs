//! Measured message-text obstacles for anchored sequence annotations.
use super::{
    DrawnFrame, Message,
    anchors::MessageAnchor,
    continuations, frames,
    geometry::{self, label_width_at},
    notes, styles,
};
use crate::sequence::{Continuation, Note, SequenceDiagram};
use crate::styles::StyleSheet;

#[derive(Clone, Copy, Debug)]
pub(super) struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Rect {
    pub fn hits_segment(self, start: (f64, f64), end: (f64, f64)) -> bool {
        let mut enter: f64 = 0.0;
        let mut exit: f64 = 1.0;
        for (origin, delta, low, high) in [
            (
                start.0,
                end.0 - start.0,
                self.x - 2.0,
                self.x + self.width + 2.0,
            ),
            (
                start.1,
                end.1 - start.1,
                self.y - 2.0,
                self.y + self.height + 2.0,
            ),
        ] {
            if delta.abs() < 1e-9 {
                if origin < low || origin > high {
                    return false;
                }
            } else {
                let a = (low - origin) / delta;
                let b = (high - origin) / delta;
                enter = enter.max(a.min(b));
                exit = exit.min(a.max(b));
            }
        }
        enter <= exit
    }

    pub fn overlaps(self, other: Self) -> bool {
        self.x < other.x + other.width + 4.0
            && self.x + self.width + 4.0 > other.x
            && self.y < other.y + other.height + 4.0
            && self.y + self.height + 4.0 > other.y
    }
}

pub(super) fn path_clear(occupied: &[Rect], points: &[(f64, f64)]) -> bool {
    points.windows(2).all(|pair| {
        !occupied
            .iter()
            .any(|region| region.hits_segment(pair[0], pair[1]))
    })
}

pub(super) fn region_clear(
    region: Rect,
    occupied: &[Rect],
    connectors: &[Vec<(f64, f64)>],
) -> bool {
    !occupied.iter().any(|other| region.overlaps(*other))
        && !connectors.iter().any(|path| {
            path.windows(2)
                .any(|pair| region.hits_segment(pair[0], pair[1]))
        })
}

pub(super) fn text(x: f64, baseline: f64, value: &str, font: f64) -> Result<Rect, String> {
    let lines = value.split('\n').count() as f64;
    Ok(Rect {
        x,
        y: baseline - font,
        width: label_width_at(value, font)?,
        height: font * (1.0 + (lines - 1.0) * 1.25),
    })
}

pub(super) fn labeled_messages(
    positioned: &[(&Message, f64)],
    anchors: &[MessageAnchor],
    sheet: &StyleSheet,
) -> Result<Vec<(usize, Rect)>, String> {
    positioned
        .iter()
        .zip(anchors)
        .enumerate()
        .filter_map(|(index, ((message, _), anchor))| {
            message.label.as_ref().map(|label| {
                let font =
                    styles::message(sheet, message.name.as_deref(), &message.classes).font_size;
                let width = label_width_at(label, font)?;
                let x = if message.from == message.to {
                    anchor.send.0 + 52.0
                } else {
                    (anchor.send.0 + anchor.receive.0 - width) / 2.0
                };
                let lines = label.split('\n').count() as f64;
                let baseline =
                    (anchor.send.1 + anchor.receive.1) / 2.0 - 9.0 - font * 1.25 * (lines - 1.0);
                text(x, baseline, label, font).map(|rect| (index, rect))
            })
        })
        .collect()
}

pub(super) fn messages(
    positioned: &[(&Message, f64)],
    anchors: &[MessageAnchor],
    sheet: &StyleSheet,
) -> Result<Vec<Rect>, String> {
    Ok(labeled_messages(positioned, anchors, sheet)?
        .into_iter()
        .map(|(_, rect)| rect)
        .collect())
}

pub(super) fn note_regions(
    diagram: &SequenceDiagram,
    anchors: &[MessageAnchor],
    centers: &[f64],
    notes_at: &[(&Note, f64)],
    continuations_at: &[(&Continuation, f64)],
    sheet: &StyleSheet,
) -> Result<Vec<Rect>, String> {
    let mut regions = Vec::new();
    for (note, top) in notes_at {
        let placement = diagram
            .note_layout
            .iter()
            .find(|p| p.note == note.id)
            .ok_or("note has no placement")?;
        let font = styles::note(sheet, &note.id).font_size;
        let width = label_width_at(&note.text, font)? + 32.0;
        let x = notes::left(diagram, centers, anchors, placement, width)?;
        regions.push(Rect {
            x,
            y: *top,
            width,
            height: notes::height(note, sheet),
        });
    }
    for (continuation, top) in continuations_at {
        let placement = diagram
            .continuation_layout
            .iter()
            .find(|p| p.id == continuation.id)
            .ok_or("continuation has no placement")?;
        let mut left = f64::INFINITY;
        let mut right = f64::NEG_INFINITY;
        for participant in &placement.participants {
            let index = diagram
                .participants
                .iter()
                .position(|p| p.id == *participant)
                .ok_or("unknown continuation participant")?;
            left = left.min(centers[index]);
            right = right.max(centers[index]);
        }
        let font = styles::continuation(sheet, &continuation.id).font_size;
        let width = (right - left + 40.0).max(label_width_at(&continuation.text, font)? + 32.0);
        regions.push(Rect {
            x: (left + right - width) / 2.0,
            y: *top,
            width,
            height: continuations::height(continuation, sheet),
        });
    }
    Ok(regions)
}

pub(super) fn frame_regions(
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    centers: &[f64],
    widths: &[f64],
    drawn: &[DrawnFrame],
) -> Result<Vec<Rect>, String> {
    let left = geometry::frame_left(diagram, sheet, centers, widths)?;
    let last_right = centers
        .last()
        .zip(widths.last())
        .map(|(x, width)| x + width / 2.0 + 16.0)
        .unwrap_or(112.0);
    let right = geometry::frame_right(diagram, left, last_right, sheet)?;
    let mut regions = Vec::new();
    for frame in drawn {
        let (x, _) = geometry::frame_bounds(diagram, sheet, centers, widths, frame, left, right)?;
        let font = styles::fragment(
            sheet,
            geometry::operator_name(&frame.operator),
            frame.id.as_deref(),
        )
        .font_size;
        let mut caption = label_width_at(&frame.operator, font)? + 22.0;
        if !frame.guard.is_empty() {
            caption += label_width_at(&format!("[{}]", frame.guard), font)? + 8.0;
        }
        regions.push(Rect {
            x,
            y: frame.top,
            width: caption,
            height: frames::descriptor_height(font),
        });
        for (guard, at) in &frame.separators {
            if !guard.is_empty() {
                regions.push(Rect {
                    x: x + 10.0,
                    y: at + 5.0,
                    width: label_width_at(&format!("[{guard}]"), font)?,
                    height: font,
                });
            }
        }
    }
    Ok(regions)
}

// A candidate needs both the occupied regions and its full canvas limits.
#[allow(clippy::too_many_arguments)]
pub(super) fn time_label(
    occupied: &[Rect],
    connectors: &[Vec<(f64, f64)>],
    x: f64,
    y: f64,
    label: &str,
    font: f64,
    floor: f64,
    ceiling: f64,
    preferred_offset: f64,
    allow_diagonal: bool,
) -> Result<(f64, f64), String> {
    let width = label_width_at(label, font)?;
    for at_y in [
        y - 18.0 - preferred_offset,
        y - 18.0,
        y - 40.0,
        y - 62.0,
        y - 84.0,
        y + 32.0,
        y + 58.0,
        y + 84.0,
    ] {
        for left in [false, true] {
            let at_x = if left { x - width - 22.0 } else { x + 22.0 };
            let region = text(at_x, at_y, label, font)?;
            let tip_x = if left { x - 18.0 } else { x + 18.0 };
            let path = if allow_diagonal {
                vec![(x, y), (tip_x, at_y + 1.0)]
            } else {
                vec![(x, y), (x, at_y + 1.0), (tip_x, at_y + 1.0)]
            };
            if region.x >= 8.0
                && region.y >= floor
                && region.y + region.height < ceiling - 8.0
                && region_clear(region, occupied, connectors)
                && path_clear(occupied, &path)
            {
                return Ok((at_x, at_y));
            }
        }
    }
    Err(format!(
        "time annotation '{label}' at y={y:.0} has no space beside its message; move the message or reduce nearby labels"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn time_text_tries_the_other_side_before_overlapping_message_text() {
        let blocking = Rect {
            x: 62.0,
            y: 64.0,
            width: 50.0,
            height: 25.0,
        };
        let (x, y) = time_label(
            &[blocking],
            &[],
            40.0,
            100.0,
            "t0",
            14.0,
            20.0,
            300.0,
            0.0,
            false,
        )
        .unwrap();
        assert!(x < 40.0 || y < 82.0);
        assert!(!text(x, y, "t0", 14.0).unwrap().overlaps(blocking));
    }

    #[test]
    fn bent_duration_connector_does_not_run_through_a_label() {
        let label = Rect {
            x: 45.0,
            y: 35.0,
            width: 20.0,
            height: 15.0,
        };
        assert!(!path_clear(
            &[label],
            &[(10.0, 40.0), (80.0, 40.0), (80.0, 90.0)]
        ));
        assert!(path_clear(
            &[label],
            &[(10.0, 20.0), (80.0, 20.0), (80.0, 90.0)]
        ));
    }

    #[test]
    fn observation_leader_avoids_text_even_when_its_own_label_is_clear() {
        let blocker = Rect {
            x: 106.0,
            y: 132.0,
            width: 10.0,
            height: 13.0,
        };
        let (x, _) = time_label(
            &[blocker],
            &[],
            100.0,
            150.0,
            "t",
            14.0,
            84.0,
            300.0,
            0.0,
            false,
        )
        .unwrap();
        assert!(x < 100.0);
    }

    #[test]
    fn repeated_observations_at_one_anchor_use_distinct_visible_positions() {
        let mut occupied = Vec::new();
        for index in 0..5 {
            let (x, y) = time_label(
                &occupied,
                &[],
                100.0,
                150.0,
                "t",
                14.0,
                84.0,
                400.0,
                index as f64 * 30.0,
                false,
            )
            .unwrap();
            let next = text(x, y, "t", 14.0).unwrap();
            assert!(!occupied.iter().any(|older: &Rect| next.overlaps(*older)));
            occupied.push(next);
        }
    }
}
