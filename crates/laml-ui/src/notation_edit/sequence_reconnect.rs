//! Endpoint reconnection and source-order moves without changing message notation.
use super::*;
use laml::sequence::{Event, MessageKind};

pub fn reconnect(
    source: &str,
    line: usize,
    moving_source: bool,
    owner: &str,
    before: Option<usize>,
    activation_line: Option<usize>,
) -> Result<(Vec<Edit>, usize), String> {
    if !crate::graph_edit::identifier(owner) {
        return Err("Choose a participant in this interaction".into());
    }
    let old = laml::sequence::parse(source).map_err(|e| e.message)?;
    let message = old
        .events
        .iter()
        .find_map(|e| {
            if let Event::Message(m) = e {
                (m.line == line).then_some(m)
            } else {
                None
            }
        })
        .ok_or("Choose a message in the current interaction")?;
    // A self-call's send and receive occurrences share a source line, but not
    // a height. Docking the other endpoint transfers the existing start.
    if activation_line == Some(line)
        && let Some(start) = old.events.iter().find_map(|e| match e {
            Event::Activate {
                participant,
                line: at,
                ..
            } if *at == line && participant == owner => Some(*at),
            _ => None,
        })
    {
        return super::sequence_align::snap_to_occurrence(
            source,
            owner,
            start,
            start,
            false,
            line,
            !moving_source,
        );
    }
    let before = activation_line.or(before);
    let activation_edit = if let Some(at) = activation_line {
        if !matches!(message.kind, MessageKind::Sync | MessageKind::Async) {
            return Err("Only a call can take over the start of this activation".into());
        }
        Some(super::sequence_activation::consume_start(
            source, at, owner,
        )?)
    } else {
        None
    };
    let field_name = if moving_source { "Source" } else { "Target" };
    let fields = fields::of(source, line);
    let endpoint = fields.iter().find(|f| f.name == field_name);
    let token_edit = if let Some(field) = endpoint {
        let mut replacement = owner.to_owned();
        if activation_line.is_some() {
            let marker = fields
                .iter()
                .find(|f| f.name == format!("{field_name} activation / lifetime"))
                .map_or("", |f| f.value.as_str());
            if marker.is_empty() {
                replacement.push('+');
            } else if marker != "+" {
                return Err("This message ends an activation or lifetime; remove that marker before using it to start an activation".into());
            }
        }
        Edit {
            range: field.range.clone(),
            replacement,
        }
    } else if message.kind == MessageKind::Reply {
        // Make an implicit reply explicit, retaining its automatic activation close.
        let operator = fields
            .iter()
            .find(|f| f.name == "Type")
            .ok_or("Reply operator not found")?;
        let (from, to) = if moving_source {
            (owner, message.to.as_str())
        } else {
            (message.from.as_str(), owner)
        };
        let closes = old.events.iter().any(|e| matches!(e, Event::Deactivate { line: at, participant, .. } if *at == line && participant == &message.from));
        Edit {
            range: operator.range.clone(),
            replacement: format!("{to} <-- {from}{}", if closes { "-" } else { "" }),
        }
    } else {
        return Err("This endpoint cannot be reconnected safely".into());
    };
    let record = record(source, line)?;
    let mut edits = if let Some(before) = before.filter(|before| *before != line) {
        let raw = &source[record.range.clone()];
        let start = token_edit.range.start - record.range.start;
        let end = token_edit.range.end - record.range.start;
        let body = format!("{}{}{}", &raw[..start], token_edit.replacement, &raw[end..]);
        let mut edits = super::sequence_insert::at(source, before, body.trim())?;
        if edits[0].range.start > record.block.start && edits[0].range.start < record.block.end {
            return Err("Choose another event boundary".into());
        }
        edits.push(Edit {
            range: record.block.clone(),
            replacement: String::new(),
        });
        edits
    } else {
        vec![token_edit]
    };
    let inserted_at = edits
        .iter()
        .find(|e| e.range.is_empty())
        .map(|e| e.range.start);
    if let Some(edit) = activation_edit {
        super::sequence_activation::absorb_start(&mut edits, edit);
    }
    edits = crate::element_edit::merge_insertions(edits);
    let candidate = apply(source, &edits)?;
    if candidate == source {
        return Ok((vec![], line));
    }
    let new = laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    for old in old.events.iter().filter_map(|e| {
        if let Event::Message(m) = e {
            (m.line != line).then_some(m)
        } else {
            None
        }
    }) {
        let mapped = crate::relation_edit::remap_line(source, old.line, &edits);
        if !new.events.iter().any(|e| matches!(e, Event::Message(m) if m.line == mapped && m.from == old.from && m.to == old.to && m.kind == old.kind)) {
            return Err("This reconnection would retarget another message or shorthand reply. Reconnect its explicit endpoints first.".into());
        }
    }
    let new_line = if before.is_some_and(|before| before != line) {
        let inserted_at = inserted_at.ok_or("Missing message insertion")?;
        let mut mapped = source[..inserted_at]
            .bytes()
            .filter(|b| *b == b'\n')
            .count()
            + 1;
        for e in &edits {
            if !e.range.is_empty() && e.range.end <= inserted_at {
                mapped -= source[e.range.clone()]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count();
            }
        }
        mapped
    } else {
        crate::relation_edit::remap_line(source, line, &edits)
    };
    let (from, to) = if moving_source {
        (owner, message.to.as_str())
    } else {
        (message.from.as_str(), owner)
    };
    if !new.events.iter().any(|e| matches!(e, Event::Message(m) if m.line == new_line && m.from == from && m.to == to && m.kind == message.kind)) {
        return Err("The resulting message would not retain the chosen endpoints and type".into());
    }
    Ok((edits, new_line))
}
