//! Read-only dependency analysis for source deletion. Branches restore incoming
//! call/execution state, just as the compiler does; visible labels are not IDs.
use laml::sequence::{Event, MessageKind, SequenceDiagram};
use std::collections::BTreeSet;
#[derive(Clone)]
struct Call {
    from: String,
    to: String,
    origins: BTreeSet<usize>,
}
#[derive(Clone, Default)]
struct Context {
    calls: Vec<Call>,
    active: Vec<(String, usize)>,
}
struct Frame {
    operator: String,
    entry: Context,
    exits: Vec<Context>,
}
fn line(event: &Event) -> usize {
    match event {
        Event::Message(m) => m.line,
        Event::Note(n) => n.line,
        Event::Continuation(c) => c.line,
        Event::Annotation(a) => a.line,
        Event::Ordering(o) => o.line,
        Event::Decomposition { line, .. }
        | Event::Reference { line, .. }
        | Event::FrameStart { line, .. }
        | Event::FrameBranch { line, .. }
        | Event::FrameEnd { line }
        | Event::Destroy { line, .. }
        | Event::Activate { line, .. }
        | Event::Deactivate { line, .. } => *line,
    }
}
pub(super) fn expand(
    source: &str,
    doc: &SequenceDiagram,
    offset: isize,
    prefix: &str,
    selected: &mut BTreeSet<usize>,
    names: &mut BTreeSet<String>,
) {
    let absolute = |line: usize| line.saturating_add_signed(offset);
    let qualify = |id: &str| {
        if prefix.is_empty() {
            id.to_owned()
        } else {
            format!("{prefix}.{id}")
        }
    };
    for participant in &doc.participants {
        let births: Vec<_> = doc
            .births
            .iter()
            .filter(|(id, _)| id == &participant.id)
            .collect();
        if selected.contains(&absolute(participant.line))
            && (births.is_empty()
                || births
                    .iter()
                    .all(|(_, line)| selected.contains(&absolute(*line))))
        {
            names.insert(qualify(&participant.id));
        }
    }
    let mut context = Context::default();
    let mut frames: Vec<Frame> = Vec::new();
    let index = crate::source::SourceIndex::new(source);
    for event in &doc.events {
        let at = absolute(line(event));
        if selected.contains(&at) {
            let id = match event {
                Event::Message(m) => m.name.as_deref(),
                Event::Note(n) => Some(n.id.as_str()),
                Event::Continuation(c) => Some(c.id.as_str()),
                Event::FrameStart { id, .. } => id.as_deref(),
                _ => None,
            };
            if let Some(id) = id {
                names.insert(qualify(id));
            }
        }
        match event {
            Event::FrameStart { operator, .. } => frames.push(Frame {
                operator: operator.clone(),
                entry: context.clone(),
                exits: vec![],
            }),
            Event::FrameBranch { .. } => {
                if let Some(frame) = frames.last_mut() {
                    frame.exits.push(context.clone());
                    context = frame.entry.clone();
                }
            }
            Event::FrameEnd { .. } => {
                if let Some(mut frame) = frames.pop() {
                    frame.exits.push(context.clone());
                    if frame
                        .operator
                        .split(|c: char| c.is_whitespace() || c == '(')
                        .any(|s| s == "opt" || s == "loop")
                    {
                        frame.exits.push(frame.entry);
                    }
                    let first = &frame.exits[0];
                    let same = frame.exits.iter().all(|exit| {
                        exit.calls.len() == first.calls.len()
                            && exit
                                .calls
                                .iter()
                                .zip(&first.calls)
                                .all(|(a, b)| a.from == b.from && a.to == b.to)
                    });
                    context = first.clone();
                    if same {
                        for exit in frame.exits.iter().skip(1) {
                            for (a, b) in context.calls.iter_mut().zip(&exit.calls) {
                                a.origins.extend(&b.origins);
                            }
                        }
                    } else {
                        context.calls.clear();
                    }
                }
            }
            Event::Message(m) if m.kind == MessageKind::Sync => context.calls.push(Call {
                from: m.from.clone(),
                to: m.to.clone(),
                origins: BTreeSet::from([at]),
            }),
            Event::Message(m) if m.kind == MessageKind::Reply => {
                let shorthand = index.line(at).is_some_and(|range| {
                    let text = super::code(&source[range]);
                    text.find("<--").is_some_and(|op| {
                        text[..op].trim().is_empty() || text[..op].trim_end().ends_with(':')
                    })
                });
                if let Some(call) = context
                    .calls
                    .last()
                    .filter(|c| c.from == m.to && c.to == m.from)
                {
                    if shorthand && call.origins.iter().any(|origin| selected.contains(origin)) {
                        selected.insert(at);
                    }
                    context.calls.pop();
                }
            }
            Event::Activate {
                participant, id, ..
            } => {
                if selected.contains(&at)
                    && let Some(id) = id
                {
                    let selector = format!(
                        "{}node[{participant}].activation[{id}]",
                        prefix
                            .split('.')
                            .filter(|s| !s.is_empty())
                            .map(|s| format!("node[{s}] "))
                            .collect::<String>()
                    )
                    .split_whitespace()
                    .collect::<String>();
                    for entry in super::records(source) {
                        let compact = entry.heading.split_whitespace().collect::<String>();
                        if entry.section == "styling" && entry.opens && compact == selector {
                            selected.insert(entry.line);
                            continue;
                        }
                        if entry.section != "layout"
                            && !["at[", "between[", "order["]
                                .iter()
                                .any(|p| entry.heading.starts_with(p))
                        {
                            continue;
                        }
                        let scope = super::references::scope(
                            source,
                            &entry,
                            crate::notation::Notation::Sequence,
                        );
                        let tokens = super::fields::tokens(&entry.heading);
                        let raw: Vec<_> =
                            tokens.iter().map(|r| &entry.heading[r.clone()]).collect();
                        for window in raw.windows(6) {
                            if (window[0] == "execution"
                                || (!prefix.is_empty()
                                    && window[0] == format!("{prefix}.execution")))
                                && window[1] == "("
                                && window[3] == ","
                                && window[4] == id
                                && window[5] == ")"
                                && window[2] == participant
                                && (scope == prefix
                                    || (scope.is_empty()
                                        && window[0] == format!("{prefix}.execution")))
                            {
                                selected.insert(entry.line);
                            }
                        }
                    }
                }
                context.active.push((participant.clone(), at));
            }
            Event::Deactivate { participant, .. } => {
                if let Some(i) = context.active.iter().rposition(|(p, _)| p == participant) {
                    if selected.contains(&context.active[i].1) {
                        selected.insert(at);
                    }
                    context.active.remove(i);
                }
            }
            Event::Destroy { participant, .. } => {
                context
                    .calls
                    .retain(|c| c.from != *participant && c.to != *participant);
                context.active.retain(|(p, _)| p != participant);
            }
            _ => {}
        }
    }
    // Lifelines created in an operand have a local lifetime. Do not remove an
    // independent reuse of the same ID in a different alternative.
    for (id, born) in &doc.births {
        if !selected.contains(&absolute(*born)) {
            continue;
        }
        let end = doc
            .events
            .iter()
            .filter_map(|e| match e {
                Event::Destroy { participant, line } if participant == id && line > born => {
                    Some(*line)
                }
                _ => None,
            })
            .min()
            .unwrap_or(usize::MAX);
        for event in &doc.events {
            let at = line(event);
            if at < *born || at > end {
                continue;
            }
            let refers = match event {
                Event::Message(m) => m.from == *id || m.to == *id,
                Event::Activate { participant, .. }
                | Event::Deactivate { participant, .. }
                | Event::Destroy { participant, .. } => participant == id,
                _ => false,
            };
            if refers {
                selected.insert(absolute(at));
            }
        }
    }
    for child in &doc.decompositions {
        expand(
            source,
            &child.diagram,
            offset + child.line as isize - 5,
            &qualify(&child.parent),
            selected,
            names,
        );
    }
}

pub(super) fn identities(doc: &SequenceDiagram, prefix: &str, out: &mut BTreeSet<String>) {
    let qualify = |id: &str| {
        if prefix.is_empty() {
            id.to_owned()
        } else {
            format!("{prefix}.{id}")
        }
    };
    out.extend(doc.participants.iter().map(|p| qualify(&p.id)));
    for event in &doc.events {
        let id = match event {
            Event::Message(m) => m.name.as_deref(),
            Event::Note(n) => Some(n.id.as_str()),
            Event::Continuation(c) => Some(c.id.as_str()),
            Event::FrameStart { id, .. } => id.as_deref(),
            _ => None,
        };
        if let Some(id) = id {
            out.insert(qualify(id));
        }
    }
    for child in &doc.decompositions {
        identities(&child.diagram, &qualify(&child.parent), out);
    }
}
