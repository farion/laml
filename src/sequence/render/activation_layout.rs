//! Equal-height execution boundaries carry their attached source events with them.
use super::{Event, Message, SequenceDiagram, DrawnExecution, OpenExecution, anchors::MessageAnchor};
use crate::sequence::activation_layout::{groups, resolve};
use std::collections::BTreeMap;

fn touches(event: &Event) -> Vec<&str> {
    match event {
        Event::Message(m) => vec![&m.from, &m.to],
        Event::Activate { participant, .. } | Event::Deactivate { participant, .. }
        | Event::Destroy { participant, .. } => vec![participant],
        _ => vec![],
    }
}
#[allow(clippy::too_many_arguments)]
pub(super) fn apply(diagram: &SequenceDiagram, positioned: &mut [(&Message, f64)],
    anchors: &mut [MessageAnchor], bars: &mut [DrawnExecution], open: &mut [OpenExecution],
    positions: &mut [(usize, f64)], deaths: &mut [(String, f64)]) -> Result<(), String> {
    let mut shifts: BTreeMap<usize, f64> = BTreeMap::new();
    for lower in [false, true] {
        for group in groups(diagram, lower) {
            let mut ends = Vec::new();
            for name in &group {
                let execution = resolve(diagram, name)?;
                let bar = bars.iter().find(|b| b.opened.line == execution.start && b.opened.participant == execution.owner);
                let active = open.iter().find(|b| b.line == execution.start && b.participant == execution.owner);
                let y = if lower { bar.map(|b| b.to) } else { bar.map(|b| b.opened.from).or_else(|| active.map(|b| b.from)) }
                    .ok_or("Missing activation edge geometry")?;
                let line = if lower { execution.end.ok_or("Close the activation before aligning its end")? } else { execution.start };
                ends.push((name, line, y));
            }
            let anchored: Vec<_> = diagram.activation_layout.iter().filter(|r|
                r.lower == lower && r.reference_second && group.contains(&r.first)).collect();
            let target = if anchored.is_empty() {
                ends.iter().map(|(_, _, y)| *y).reduce(if lower { f64::max } else { f64::min }).unwrap()
            } else {
                // Follow reference chains to their stationary target, rather than
                // forcing an upward drag to extend both bars downward.
                let roots: Vec<_> = anchored.iter().filter(|r| !anchored.iter().any(|other| other.first == r.second))
                    .filter_map(|r| ends.iter().find(|(name, _, _)| **name == r.second).map(|(_, _, y)| *y)).collect();
                let Some(&target) = roots.first() else { return Err("Cyclic activation alignment references".into()); };
                if roots.iter().any(|y| (*y-target).abs() > 0.01) { return Err("Conflicting fixed activation alignment references".into()); }
                target
            };
            for (_, line, y) in ends {
                let delta = target - y;
                if let Some(old) = shifts.insert(line, delta) && (old - delta).abs() > 0.01 {
                    return Err("Conflicting activation alignments on the same source event".into());
                }
            }
        }
    }
    if shifts.is_empty() { return Ok(()); }
    let position = |line: usize| positions.iter().find(|(l, _)| *l == line)
        .map(|(_, y)| *y + shifts.get(&line).copied().unwrap_or(0.0));
    for event in &diagram.events {
        let line = super::authoring::event_line(event);
        if shifts.get(&line).is_none_or(|delta| delta.abs() < 0.01) { continue; }
        let Some(y) = position(line) else { continue; };
        let owners = touches(event);
        for other in &diagram.events {
            let at = super::authoring::event_line(other);
            if at == line || !touches(other).iter().any(|id| owners.contains(id)) { continue; }
            if let Some(other_y) = position(at)
                && ((at < line && other_y > y + 0.01) || (at > line && other_y < y - 0.01)) {
                return Err("Activation alignment would reverse events on a lifeline; move the intervening event first".into());
            }
        }
    }
    for (line, y) in positions.iter_mut() { *y += shifts.get(line).copied().unwrap_or(0.0); }
    for ((message, y), anchor) in positioned.iter_mut().zip(anchors) {
        let delta = shifts.get(&message.line).copied().unwrap_or(0.0);
        *y += delta; anchor.send.1 += delta; anchor.receive.1 += delta;
    }
    for bar in bars {
        bar.opened.from += shifts.get(&bar.opened.line).copied().unwrap_or(0.0);
        bar.to += bar.end_line.and_then(|line| shifts.get(&line).copied()).unwrap_or(0.0);
        if bar.to < bar.opened.from + 2.0 { return Err("Activation alignment leaves no room for the activation".into()); }
    }
    for bar in open { bar.from += shifts.get(&bar.line).copied().unwrap_or(0.0); }
    for event in &diagram.events {
        if let Event::Destroy { participant, line } = event && let Some(delta) = shifts.get(line) {
            if deaths.iter().filter(|(id, _)| id == participant).count() != 1 {
                return Err("Cannot align an ambiguous destruction-bound activation".into());
            }
            if let Some((_, y)) = deaths.iter_mut().find(|(id, _)| id == participant) { *y += delta; }
        }
    }
    Ok(())
}

pub(super) fn verify(diagram: &SequenceDiagram, bars: &[DrawnExecution], open: &[OpenExecution]) -> Result<(), String> {
    for rule in &diagram.activation_layout {
        let y = |name: &str| -> Result<f64, String> {
            let e = resolve(diagram, name)?;
            let closed = bars.iter().find(|b| b.opened.line == e.start && b.opened.participant == e.owner);
            if rule.lower { closed.map(|b| b.to) } else {
                closed.map(|b| b.opened.from).or_else(|| open.iter().find(|b| b.line == e.start && b.participant == e.owner).map(|b| b.from))
            }.ok_or_else(|| "Missing aligned activation".into())
        };
        if (y(&rule.first)? - y(&rule.second)?).abs() > 0.01 {
            return Err("Activation alignment conflicts with another mandatory layout rule".into());
        }
    }
    Ok(())
}
