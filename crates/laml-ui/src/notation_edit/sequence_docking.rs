//! Align message occurrences with activation edges without consuming declarations.
use super::sequence_controls::{Boundary, Dock};
use super::*;
use laml::sequence::{AnnotationEnd, Event, occurrence_layout::Endpoint};
fn occurrence(
    rule: &laml::sequence::occurrence_layout::Alignment,
    name: &str,
    receive: bool,
) -> bool {
    [&rule.first,&rule.second].iter().any(|e|matches!(e,Endpoint::Message {name:id,end} if id==name && (*end==AnnotationEnd::Receive)==receive))
}
fn erase_rule(source: &str, line: usize) -> Result<Edit, String> {
    let r = record(source, line)?;
    let raw = &source[r.range.clone()];
    let body = code(raw);
    Ok(Edit {
        range: r.range.start + body.len() - body.trim_start().len()
            ..r.range.start + body.trim_end().len(),
        replacement: String::new(),
    })
}
pub fn detach(source: &str, line: usize, receive: bool) -> Result<Vec<Edit>, String> {
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    let message = doc
        .events
        .iter()
        .find_map(|e| match e {
            Event::Message(m) if m.line == line => Some(m),
            _ => None,
        })
        .ok_or("The message no longer exists")?;
    let Some(name) = &message.name else {
        return Ok(vec![]);
    };
    doc.occurrence_layout
        .iter()
        .filter(|r| occurrence(r, name, receive))
        .map(|r| erase_rule(source, r.line))
        .collect()
}
fn fresh(source: &str, prefix: &str) -> String {
    (1..)
        .map(|n| format!("{prefix}{n}"))
        .find(|id| !source.contains(id))
        .unwrap()
}
fn message_name(source: &mut String, line: usize) -> Result<String, String> {
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    let m = doc
        .events
        .iter()
        .find_map(|e| match e {
            Event::Message(m) if m.line == line => Some(m),
            _ => None,
        })
        .ok_or("The message no longer exists")?;
    if let Some(name) = &m.name {
        return Ok(name.clone());
    }
    let name = fresh(source, "message");
    *source = apply(source, &super::sequence::assign_name(source, line, &name)?)?;
    Ok(name)
}
fn activation_reference(source: &mut String, dock: &Dock) -> Result<String, String> {
    let (start, lower) = match dock.boundary {
        Some(Boundary::Start(s)) => (s, false),
        Some(Boundary::End { start, .. }) => (start, true),
        None => return Err("Choose an activation boundary".into()),
    };
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    let executions = laml::sequence::activation_layout::executions(&doc);
    let e = executions
        .iter()
        .find(|e| e.owner == dock.owner && e.start == start)
        .ok_or("The activation no longer exists")?;
    if lower && e.end.is_none() {
        let (edits, _) = super::sequence_drag::close_open(source, start, 0)?;
        *source = apply(source, &edits)?;
    }
    let name = if let Some(name) = &e.name {
        Some(name.clone())
    } else if executions.iter().filter(|e| e.owner == dock.owner).count() == 1 {
        None
    } else {
        let name = fresh(source, "activation");
        *source = apply(source, &super::sequence::assign_name(source, start, &name)?)?;
        Some(name)
    };
    Ok(format!(
        "{}{}{}",
        dock.owner,
        name.map_or(String::new(), |n| format!(".{n}")),
        if lower { '-' } else { '+' }
    ))
}
/// Names are added only when an occurrence/ambiguous bar needs an address.
/// Existing names, labels and message-bound +/- markers remain unchanged.
pub fn align(
    source: &str,
    line: usize,
    dock: &Dock,
    receive: bool,
) -> Result<(Vec<Edit>, usize), String> {
    if let Some(boundary) = &dock.boundary {
        let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
        if let Some(name) = doc.events.iter().find_map(|e| match e {
            Event::Message(m) if m.line == line => m.name.as_deref(),
            _ => None,
        }) {
            let rules = doc
                .occurrence_layout
                .iter()
                .filter(|r| occurrence(r, name, receive))
                .collect::<Vec<_>>();
            if rules.len() == 1
                && rules[0].reference_second
                && let Endpoint::Activation { reference, lower } = &rules[0].second
            {
                let e = laml::sequence::activation_layout::resolve(&doc, reference)?;
                let (start, wanted_lower) = match boundary {
                    Boundary::Start(s) => (*s, false),
                    Boundary::End { start, .. } => (*start, true),
                };
                if e.owner == dock.owner && e.start == start && *lower == wanted_lower {
                    return Ok((vec![], line));
                }
            }
        }
    }

    if dock.boundary.is_none() {
        return Ok((detach(source, line, receive)?, line));
    }
    let mut candidate = source.to_owned();
    let name = message_name(&mut candidate, line)?;
    let reference = activation_reference(&mut candidate, dock)?;
    let removal = detach(&candidate, line, receive)?;
    candidate = apply(&candidate, &removal)?;
    let declaration = format!(
        "{name}.{} ? {reference} : align=center-y; reference=second",
        if receive { "receive" } else { "send" }
    );
    let edits = insert(&candidate, "layout", None, None, &declaration)?;
    candidate = apply(&candidate, &edits)?;
    laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    let edits = crate::graph_edit::difference(source, &candidate);
    Ok((edits, line))
}
/// Retain source occurrence identity after an edit whose minimal diff spans
/// other statements. Structural docking never changes message order.
pub fn message_line(source: &str, ordinal: usize) -> Result<usize, String> {
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    doc.events
        .iter()
        .filter_map(|e| {
            if let Event::Message(m) = e {
                Some(m.line)
            } else {
                None
            }
        })
        .nth(ordinal)
        .ok_or_else(|| "Missing message occurrence".into())
}

/// New arrivals at a top are declared immediately after the opening event,
/// keeping that event's identity and automatic row stable.
pub fn insertion_before(source: &str, dock: &Dock) -> Result<usize, String> {
    match dock.boundary {
        Some(Boundary::Start(start)) => Ok(record(source, start)?.line + 1),
        _ => Ok(dock.before),
    }
}

/// An explicit row move releases layout-only docks, not semantic +/- markers.
pub fn move_message(
    source: &str,
    line: usize,
    before: usize,
) -> Result<(Vec<Edit>, usize), String> {
    let mut candidate = source.to_owned();
    for receive in [false, true] {
        let edits = detach(&candidate, line, receive)?;
        candidate = apply(&candidate, &edits)?;
    }
    let (edits, selected) = super::sequence_drag::move_event(&candidate, line, before)?;
    candidate = apply(&candidate, &edits)?;
    Ok((crate::graph_edit::difference(source, &candidate), selected))
}
