//! Contextual sequence edits use source event boundaries, never stored canvas coordinates.
use super::*;
use laml::sequence::{Event, MessageKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Activate,
    EndActivation,
    Destroy,
    Note,
    Participant,
    Sync,
    SyncActivate,
    Async,
    Reply,
}

/// Insert immediately before a compiler event (including a closing operand brace).
/// Zero denotes the end of the root interaction.
pub fn at(source: &str, before: usize, body: &str) -> Result<Vec<Edit>, String> {
    if before == 0 {
        return insert(source, "semantics", None, None, body);
    }
    let index = SourceIndex::new(source);
    let range = index
        .line(before)
        .ok_or("The insertion point no longer exists")?;
    let raw = &source[range.clone()];
    let indent = &raw[..raw.len() - raw.trim_start().len()];
    let indent = if code(raw).trim() == "}" {
        format!("{indent}    ")
    } else {
        indent.into()
    };
    let nl = crate::element_edit::newline(source);
    Ok(vec![Edit {
        range: range.start..range.start,
        replacement: body
            .lines()
            .map(|line| format!("{indent}{line}{nl}"))
            .collect(),
    }])
}

#[allow(clippy::too_many_arguments)]
pub fn create(
    source: &str,
    before: usize,
    owner: &str,
    action: Action,
    target: &str,
    id: &str,
    label: &str,
    right_neighbor: Option<&str>,
    end_line: Option<usize>,
    activation_line: Option<usize>,
) -> Result<Vec<Edit>, String> {
    if !crate::graph_edit::identifier(owner) {
        return Err("Choose a participant in this interaction".into());
    }
    let quoted = crate::relation_edit::quote(label);
    let mut edits = match action {
        Action::Activate => at(source, before, &format!("{owner}+\n{owner}-"))?,
        Action::Destroy => at(source, before, &format!("{owner}!"))?,
        Action::EndActivation => {
            let Some(line) = end_line else {
                let edits = at(source, before, &format!("{owner}-"))?;
                let candidate = apply(source, &edits)?;
                laml::sequence::parse(&candidate)
                    .map_err(|e| format!("Cannot end this activation here: {}", e.message))?;
                return Ok(edits);
            };
            if line == before {
                return Err("This activation already ends at this event".into());
            }
            let r = record(source, line)?;
            let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
            if !doc.events.iter().any(|e| matches!(e, Event::Deactivate { participant, line: l, .. } if participant == owner && *l == line)) {
                return Err("This activation ends on a message; edit that message to change its end".into());
            }
            let closing = source[r.range.clone()].trim();
            let mut edits = at(source, before, closing)?;
            edits.push(Edit {
                range: r.block,
                replacement: String::new(),
            });
            edits
        }
        Action::Sync | Action::SyncActivate | Action::Async | Action::Reply => {
            if !crate::graph_edit::identifier(target) {
                return Err("Choose a target participant".into());
            }
            let activates = action == Action::SyncActivate || activation_line.is_some();
            if action == Action::Reply && activates {
                return Err("A reply cannot start the receiver's activation; choose a call".into());
            }
            let marker = if activates { "+" } else { "" };
            let mut body = match action {
                Action::Reply => format!("<-- : {quoted}"),
                Action::Async => format!("{owner} ->> {target}{marker} : {quoted}"),
                _ => format!("{owner} -> {target}{marker} : {quoted}"),
            };
            // A new bar is one atomic opening/closing pair. Absorbing an
            // existing start keeps its existing end instead of adding a second.
            if activates && activation_line.is_none() {
                body.push_str(&format!("\n{target}-"));
            }
            let mut edits = at(source, activation_line.unwrap_or(before), &body)?;
            if let Some(line) = activation_line {
                let removal = super::sequence_activation::consume_start(source, line, target)?;
                super::sequence_activation::absorb_start(&mut edits, removal);
            }
            edits
        }
        Action::Note | Action::Participant => {
            if !crate::graph_edit::identifier(id) {
                return Err("Use a simple identifier".into());
            }
            if records(source).iter().any(|r| {
                fields::tokens(&r.heading)
                    .iter()
                    .any(|t| &r.heading[t.clone()] == id)
            }) || laml::sequence::parse(source)
                .map_err(|e| e.message)?
                .participants
                .iter()
                .any(|p| p.id == id)
            {
                return Err("Choose an unused identifier".into());
            }
            if action == Action::Note {
                let mut edits = at(source, before, &format!("{id}: note {quoted}"))?;
                edits.extend(insert(
                    source,
                    "layout",
                    None,
                    None,
                    &format!("{id} over {owner}"),
                )?);
                edits
            } else {
                // Declare before the first root event, so creation never changes lifetimes.
                let records = records(source);
                let first = records
                    .iter()
                    .find(|r| r.section == "semantics" && r.parent.is_none())
                    .map(|r| r.line);
                let mut edits = insert(
                    source,
                    "semantics",
                    None,
                    first,
                    &format!("{id}(label={quoted})"),
                )?;
                let mut layout = format!("{owner} < {id}");
                if let Some(right) = right_neighbor {
                    layout.push_str(&format!("\n{id} < {right}"));
                }
                edits.extend(insert(source, "layout", None, None, &layout)?);
                edits
            }
        }
    };
    edits = crate::element_edit::merge_insertions(edits);
    let candidate = apply(source, &edits)?;
    let old = laml::sequence::parse(source).map_err(|e| e.message)?;
    let new = laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    // Inserting a call must not silently retarget an existing shorthand reply.
    for old in old.events.iter().filter_map(|e| {
        if let Event::Message(m) = e {
            Some(m)
        } else {
            None
        }
    }) {
        let line = crate::relation_edit::remap_line(source, old.line, &edits);
        if !new.events.iter().any(|e| matches!(e, Event::Message(m) if m.line == line && m.from == old.from && m.to == old.to && m.kind == old.kind)) {
            return Err("This insertion would change an existing message or reply. Close the new call before the existing reply.".into());
        }
    }
    if action == Action::Reply {
        let inserted = edits
            .iter()
            .find(|e| e.replacement.contains("<--"))
            .ok_or("Missing reply")?;
        let line = candidate[..inserted.range.start]
            .bytes()
            .filter(|b| *b == b'\n')
            .count()
            + 1;
        if !new.events.iter().any(|e| matches!(e, Event::Message(m) if m.line == line && m.kind == MessageKind::Reply && m.from == owner && m.to == target)) {
            return Err("A reply here must return from the receiver to the caller of the last unambiguous open call".into());
        }
    }
    Ok(edits)
}
