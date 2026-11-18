//! Check mixed layout against nested bars, lifetimes and explicit occurrence gaps.
use super::{anchors, retime::Shiftables};
use crate::sequence::{Event, SequenceDiagram};
pub(super) fn verify(doc: &SequenceDiagram, t: &Shiftables<'_, '_>) -> Result<(), String> {
    let mut bars = t
        .bars
        .iter()
        .map(|b| {
            (
                &b.opened.participant,
                b.opened.line,
                b.opened.depth,
                b.opened.from,
                Some(b.to),
                b.end_line,
            )
        })
        .collect::<Vec<_>>();
    bars.extend(
        t.open
            .iter()
            .map(|b| (&b.participant, b.line, b.depth, b.from, None, None)),
    );
    for (owner, line, depth, from, to, end) in &bars {
        if !from.is_finite() || to.is_some_and(|y| !y.is_finite() || y < from + 2.0) {
            return Err("Occurrence alignment leaves no room for an activation".into());
        }
        if *depth > 0 {
            let parent = bars
                .iter()
                .filter(|(p, s, d, _, _, e)| {
                    p == owner
                        && *d + 1 == *depth
                        && *s <= *line
                        && e.is_none_or(|e| end.is_some_and(|at| at <= e))
                })
                .min_by_key(|(_, _, _, _, _, e)| e.unwrap_or(usize::MAX));
            if let Some((_, _, _, top, bottom, _)) = parent
                && (*from < *top - 0.01
                    || bottom.is_some_and(|y| to.is_none_or(|at| at > y + 0.01)))
            {
                return Err(
                    "Occurrence alignment moves a nested activation outside its parent".into(),
                );
            }
        }
    }
    for rule in &doc.vertical_layout {
        let (_, above) = anchors::position(t.anchors, &rule.above.message, rule.above.end)?;
        let (_, below) = anchors::position(t.anchors, &rule.below.message, rule.below.end)?;
        if below + 0.01 < above + rule.gap {
            return Err("Occurrence alignment conflicts with a mandatory occurrence gap".into());
        }
    }
    let mut deaths = Vec::new();
    let mut counts = std::collections::HashMap::<&str, usize>::new();
    for event in &doc.events {
        if let Event::Destroy { participant, line } = event {
            let n = counts.entry(participant).or_default();
            if let Some((_, y)) = t.deaths.iter().filter(|(id, _)| id == participant).nth(*n) {
                deaths.push((participant, *line, *y));
            }
            *n += 1;
        }
    }
    let mut message_index = 0;
    for event in &doc.events {
        let Event::Message(m) = event else {
            continue;
        };
        let a = &t.anchors[message_index];
        message_index += 1;
        if m.from == m.to && a.receive.1 < a.send.1 + 2.0 {
            return Err("A self-message needs separate send and receive heights".into());
        }
        for (owner, y) in [(&m.from, a.send.1), (&m.to, a.receive.1)] {
            if let Some((_, line)) = doc
                .births
                .iter()
                .filter(|(id, line)| id == owner && *line <= m.line)
                .max_by_key(|(_, line)| *line)
            {
                let birth = t
                    .positions
                    .iter()
                    .find(|(at, _)| at == line)
                    .ok_or("Missing creation occurrence")?
                    .1;
                if y < birth - 0.01 {
                    return Err(
                        "Occurrence alignment places a message before participant creation".into(),
                    );
                }
            }
            if let Some((_, _, death)) = deaths
                .iter()
                .filter(|(id, line, _)| *id == owner && *line >= m.line)
                .min_by_key(|(_, line, _)| *line)
                && y > *death + 0.01
            {
                return Err(
                    "Occurrence alignment places a message after participant destruction".into(),
                );
            }
        }
    }
    // Existing standalone-destruction alignments remain mandatory too.
    for rule in &doc.destruction_layout {
        let y = |id: &str| {
            t.deaths
                .iter()
                .find(|(owner, _)| owner == id)
                .map(|(_, y)| *y)
        };
        if let (Some(a), Some(b)) = (y(&rule.first), y(&rule.second))
            && (a - b).abs() > 0.01
        {
            return Err("Occurrence alignment conflicts with destruction alignment".into());
        }
    }
    Ok(())
}
