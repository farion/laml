//! Straighten one message end without releasing the opposite end's source anchor.
use super::sequence_controls::{Boundary, Dock};
use super::*;
use laml::sequence::{
    AnnotationEnd, Event, SequenceDiagram, activation_layout, occurrence_layout::Endpoint,
};

fn anchor(doc: &SequenceDiagram, line: usize, receive: bool) -> Result<Option<Dock>, String> {
    let message = sequence_lookup::message(doc, line).ok_or("The message no longer exists")?;
    if let Some(name) = &message.name {
        for rule in &doc.occurrence_layout {
            let matches = |endpoint: &Endpoint| matches!(endpoint,Endpoint::Message{name:id,end} if id==name && (*end==AnnotationEnd::Receive)==receive);
            let other = if matches(&rule.first) {
                Some(&rule.second)
            } else if matches(&rule.second) {
                Some(&rule.first)
            } else {
                None
            };
            if let Some(Endpoint::Activation { reference, lower }) = other {
                let execution = activation_layout::resolve(doc, reference)?;
                return Ok(Some(dock(&execution, *lower)));
            }
        }
    }
    // A +/- at the stationary occurrence is an anchor too, even when it is
    // positioned through activation layout rather than a message-height rule.
    let owner = if receive { &message.to } else { &message.from };
    for event in &doc.events {
        let lower = match event {
            Event::Activate {
                participant,
                line: at,
                at_send,
                ..
            } if *at == line && participant == owner && *at_send != receive => false,
            Event::Deactivate {
                participant,
                line: at,
                at_send,
                ..
            } if *at == line && participant == owner && *at_send != receive => true,
            _ => continue,
        };
        if let Some(e) = activation_layout::executions(doc).iter().find(|e| {
            &e.owner == owner
                && if lower {
                    e.end == Some(line)
                } else {
                    e.start == line
                }
        }) {
            return Ok(Some(dock(e, lower)));
        }
    }
    Ok(None)
}
fn dock(execution: &activation_layout::Execution, lower: bool) -> Dock {
    Dock {
        owner: execution.owner.clone(),
        before: if lower {
            execution.end.unwrap_or(0)
        } else {
            execution.start
        },
        boundary: Some(if lower {
            Boundary::End {
                start: execution.start,
                end: execution.end,
            }
        } else {
            Boundary::Start(execution.start)
        }),
    }
}
pub(super) fn reconnect(
    source: &str,
    line: usize,
    moving_source: bool,
    target: &Dock,
) -> Result<(Vec<Edit>, usize), String> {
    if target.boundary.is_some() {
        return Err("Choose the projected lifeline target to straighten a message".into());
    }
    if let Some(scope) = sequence_scope::Scope::of(source, &target.owner)? {
        let (edits, selected) = reconnect(
            &scope.source,
            scope.line(line),
            moving_source,
            &scope.dock(target)?,
        )?;
        return Ok((scope.commit(source, &edits)?, scope.absolute(selected)));
    }
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    let old = sequence_lookup::message(&doc, line).ok_or("The message no longer exists")?;
    if old.from == old.to {
        return Err("A self-message retains distinct send and receive heights".into());
    }
    // The ordinary adapter keeps event order, markers, the stationary endpoint
    // and its rules, while releasing only the moved occurrence's old binding.
    let (edits, selected) =
        sequence_controls::reconnect(source, line, moving_source, target, true)?;
    let mut candidate = apply(source, &edits)?;
    let doc = laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    let message = sequence_lookup::message(&doc, selected).ok_or("The message no longer exists")?;
    if message.from == message.to {
        return Err("Straightening must not collapse a self-message".into());
    }
    if let Some(reference) = anchor(&doc, selected, moving_source)? {
        // Reference ownership is deliberately independent of endpoint ownership:
        // a Client receive can share Service's activation end without retargeting.
        let (binding, _) =
            sequence_docking::align(&candidate, selected, &reference, !moving_source)?;
        candidate = apply(&candidate, &binding)?;
    }
    // With no stationary anchor, releasing the moved binding restores the
    // message's natural horizontal event row. Geometry preflight checks equality.
    Ok((crate::graph_edit::difference(source, &candidate), selected))
}
