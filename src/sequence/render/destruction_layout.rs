//! Align complete destruction geometry after message/label reflow.
use super::{SequenceDiagram, DrawnExecution};

pub(super) fn apply(diagram: &SequenceDiagram, deaths: &mut [(String, f64)],
    positions: &mut [(usize, f64)], bars: &mut [DrawnExecution]) -> Result<(), String> {
    for group in crate::sequence::destruction_layout::groups(diagram) {
        // Moving the earlier end down preserves all preceding occurrences on
        // each lifeline. No source event is reordered to achieve equal height.
        let target = deaths.iter().filter(|(id, _)| group.contains(id))
            .map(|(_, y)| *y).reduce(f64::max).ok_or("Missing destruction geometry")?;
        for id in group {
            let line = crate::sequence::destruction_layout::standalone_line(diagram, &id)?;
            let (_, y) = deaths.iter_mut().find(|(owner, _)| *owner == id)
                .ok_or("Missing destruction occurrence")?;
            let old = *y;
            *y = target;
            let (_, position) = positions.iter_mut().find(|(at, _)| *at == line)
                .ok_or("Missing destruction source position")?;
            *position = target;
            for bar in bars.iter_mut().filter(|bar| bar.opened.participant == id && (bar.to - old).abs() < 0.01) {
                bar.to = target;
            }
        }
    }
    Ok(())
}
