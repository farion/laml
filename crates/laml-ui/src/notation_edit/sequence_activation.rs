//! Bind activation boundaries to explicitly selected message occurrences.
use super::*;
use laml::sequence::Event;

/// Join a before-start insertion and the consumed declaration into one patch.
/// Document transactions deliberately reject edits sharing the same byte start.
pub fn absorb_start(edits: &mut Vec<Edit>, removal: Edit) {
    if let Some(insertion) = edits
        .iter_mut()
        .find(|e| e.range.is_empty() && e.range.start == removal.range.start)
    {
        insertion.range.end = removal.range.end;
    } else {
        edits.push(removal);
    }
}

pub fn consume_start(source: &str, line: usize, owner: &str) -> Result<Edit, String> {
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    if doc
        .events
        .iter()
        .any(|e| matches!(e, Event::Message(m) if m.line == line))
    {
        return Err(
            "This activation already starts with a message. Drop inside the bar instead.".into(),
        );
    }
    let (id, label) = doc
        .events
        .iter()
        .find_map(|e| match e {
            Event::Activate {
                participant,
                id,
                label,
                line: at,
                ..
            } if *at == line && participant == owner => Some((id, label)),
            _ => None,
        })
        .ok_or("The activation start no longer exists")?;
    if id.is_some() || label.is_some() {
        return Err("This activation has its own name or label. Keep that declaration and drop inside the bar, or start a new activation with the call.".into());
    }
    let record = record(source, line)?;
    let raw = &source[record.range.clone()];
    let body = code(raw);
    // An activation's inline comment remains a standalone comment at the same
    // source location; comments on the incoming message are preserved separately.
    if raw[body.trim_end().len()..].trim().is_empty() {
        Ok(Edit {
            range: record.block,
            replacement: String::new(),
        })
    } else {
        let start = body.len() - body.trim_start().len();
        Ok(Edit {
            range: record.range.start + start..record.range.start + body.trim_end().len(),
            replacement: String::new(),
        })
    }
}

/// Fuse a standalone activation end with the chosen message endpoint.
/// Existing message-bound ends cannot be stolen from another message.
/// `receive` identifies the dragged occurrence even when both ends share an owner;
/// an end line of zero explicitly closes the innermost open activation.
pub fn bind_end(
    source: &str,
    edits: Vec<Edit>,
    selected: Option<usize>,
    owner: &str,
    end_line: usize,
    receive: bool,
) -> Result<(Vec<Edit>, usize), String> {
    let old = laml::sequence::parse(source).map_err(|e| e.message)?;
    let intermediate = apply(source, &edits)?;
    let doc = laml::sequence::parse(&intermediate).map_err(|e| e.message)?;
    let end = if end_line == 0 {
        None
    } else {
        Some(crate::relation_edit::remap_line(source, end_line, &edits))
    };
    let line = selected.or_else(|| doc.events.iter().find_map(|e| {
        let Event::Message(m) = e else { return None; };
        (!old.events.iter().any(|old| matches!(old, Event::Message(old) if crate::relation_edit::remap_line(source, old.line, &edits) == m.line))).then_some(m.line)
    })).ok_or("The new message could not be identified")?;
    if end.is_some_and(|end| {
        end != line
            && doc
                .events
                .iter()
                .any(|e| matches!(e, Event::Message(m) if m.line == end))
    }) {
        return Err("This activation already ends with another message. Drop before or after that message instead.".into());
    }
    let execution = laml::sequence::activation_layout::executions(&doc)
        .into_iter()
        .filter(|e| e.owner == owner && e.end == end)
        .last()
        .ok_or("The activation end no longer exists")?;
    let (finish, _) = super::sequence_align::snap_to_occurrence(
        &intermediate,
        owner,
        execution.start,
        end.unwrap_or(execution.start),
        true,
        line,
        receive,
    )?;
    let selected = crate::relation_edit::remap_line(&intermediate, line, &finish);
    let candidate = apply(&intermediate, &finish)?;
    Ok((crate::graph_edit::difference(source, &candidate), selected))
}

#[cfg(test)]
mod end_tests {
    use super::*;
    const SOURCE: &str = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n B+\n A -> B: Hello // keep\n B- // end comment\n}\n";
    #[test]
    fn incoming_message_takes_over_standalone_end_and_preserves_comments() {
        let (edits, line) =
            super::super::sequence_reconnect::reconnect(SOURCE, 8, false, "B", Some(9), None)
                .unwrap();
        let (edits, selected) = bind_end(SOURCE, edits, Some(line), "B", 9, true).unwrap();
        let candidate = apply(SOURCE, &edits).unwrap();
        assert!(candidate.contains("A -> B-: Hello // keep"));
        assert!(candidate.contains("// end comment"));
        let doc = laml::sequence::parse(&candidate).unwrap();
        assert!(doc.events.iter().any(|e| matches!(e, Event::Deactivate { participant, line, .. } if participant == "B" && *line == selected)));
        let preview = crate::preview::build(&candidate, std::path::Path::new("end.laml")).unwrap();
        let bar = preview
            .regions
            .iter()
            .find(|r| r.kind == "sequence-execution")
            .unwrap();
        let receive = preview
            .regions
            .iter()
            .find(|r| r.kind == "sequence-receive" && r.line == selected)
            .unwrap();
        assert!((bar.y + bar.height - receive.y).abs() < 0.01);
    }
    #[test]
    fn newly_inserted_call_can_end_the_existing_activation() {
        let edits = super::super::sequence_insert::at(SOURCE, 9, "A -> B: New").unwrap();
        let (edits, _) = bind_end(SOURCE, edits, None, "B", 9, true).unwrap();
        let candidate = apply(SOURCE, &edits).unwrap();
        assert!(candidate.contains("A -> B-: New"));
        assert!(candidate.contains("A -> B: Hello // keep"));
    }
    #[test]
    fn a_message_bound_end_cannot_be_stolen() {
        let source = SOURCE.replace(
            "A -> B: Hello // keep\n B-",
            "A -> B-: Hello // keep\n // closed",
        );
        let edits = super::super::sequence_insert::at(&source, 8, "A -> B: New").unwrap();
        assert!(
            bind_end(&source, edits, None, "B", 8, true)
                .unwrap_err()
                .contains("another message")
        );
    }
    #[test]
    fn outgoing_message_can_end_its_source_activation() {
        let source = SOURCE.replace("A -> B: Hello", "B -> A: Hello");
        let (edits, line) =
            super::super::sequence_reconnect::reconnect(&source, 8, true, "B", Some(9), None)
                .unwrap();
        let (edits, _) = bind_end(&source, edits, Some(line), "B", 9, false).unwrap();
        assert!(apply(&source, &edits).unwrap().contains("B- -> A: Hello"));
    }
}
