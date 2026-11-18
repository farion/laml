//! Resize a message-bound activation edge without reordering its message.
use super::*;
use laml::sequence::{Event, activation_layout::executions};

pub(super) fn move_edge(
    source: &str,
    owner: &str,
    start: usize,
    edge: usize,
    lower: bool,
    before: usize,
) -> Result<(Vec<Edit>, usize), String> {
    let old = laml::sequence::parse(source).map_err(|e| e.message)?;
    let Some(message) = old.events.iter().find_map(|e| match e {
        Event::Message(m) if m.line == edge => Some(m),
        _ => None,
    }) else {
        return super::sequence_drag::move_event(source, edge, before);
    };
    let execution = executions(&old)
        .into_iter()
        .find(|e| e.owner == owner && e.start == start)
        .ok_or("The activation no longer exists")?;
    if old
        .participants
        .iter()
        .any(|p| p.id == owner && (p.created_line == Some(edge) || p.destroyed_line == Some(edge)))
    {
        return Err("This edge also creates or destroys the participant; its lifetime cannot be moved independently".into());
    }
    if super::sequence_drag::insertion_parent(source, before)? != record(source, edge)?.parent {
        return Err("Resize within the same fragment operand".into());
    }
    let unlink = if let Ok(marker) = super::sequence_snap::bound_marker(source, edge, owner, lower)
    {
        Edit {
            range: marker.range,
            replacement: String::new(),
        }
    } else if lower && message.kind == laml::sequence::MessageKind::Reply {
        // Shorthand replies implicitly close the callee. An explicit reply
        // retains the endpoints/call pairing without that implicit close.
        let fields = fields::of(source, edge);
        let operator = fields
            .iter()
            .find(|f| f.name == "Type")
            .ok_or("Missing reply operator")?;
        Edit {
            range: operator.range.clone(),
            replacement: format!("{} <-- {}", message.to, message.from),
        }
    } else {
        return Err("This message edge cannot be resized independently".into());
    };
    let body = if lower {
        format!("{owner}-")
    } else if let Some(name) = &execution.name {
        format!("{name}: {owner}+")
    } else {
        format!("{owner}+")
    };
    let mut edits = super::sequence_insert::at(source, before, &body)?;
    let inserted = source[..edits[0].range.start]
        .bytes()
        .filter(|b| *b == b'\n')
        .count()
        + 1;
    edits.push(unlink);
    let candidate = apply(source, &edits)?;
    let new = laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    let map = |line| crate::relation_edit::remap_line(source, line, &edits);
    // Neither endpoint resizing nor marker extraction may retarget/reorder messages.
    let messages = |doc: &laml::sequence::SequenceDiagram| {
        doc.events
            .iter()
            .filter_map(|e| {
                if let Event::Message(m) = e {
                    Some((m.line, m.from.clone(), m.to.clone(), m.kind))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };
    let expected = messages(&old)
        .into_iter()
        .map(|(line, from, to, kind)| (map(line), from, to, kind))
        .collect::<Vec<_>>();
    if messages(&new) != expected {
        return Err("Resizing this edge would change a message or shorthand reply".into());
    }
    let new_pairs = super::sequence_drag::bindings(&new);
    for (id, a, b) in super::sequence_drag::bindings(&old) {
        let edited = id == owner && a == start;
        let new_start = if edited && !lower { inserted } else { map(a) };
        let new_end = if edited && lower {
            inserted
        } else if b == 0 {
            0
        } else {
            map(b)
        };
        if !new_pairs
            .iter()
            .any(|(p, s, e)| *p == id && *s == new_start && *e == new_end)
        {
            return Err("This resize would change which activation an end belongs to".into());
        }
    }
    let selected = if lower { map(start) } else { inserted };
    Ok((edits, selected))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> String {
        "settings {\n type: sequence\n}\nsemantics {\n Client\n Service\n Client -> Service\n call: Client -> Service+: request // keep\n <--: response\n}\n".into()
    }
    #[test]
    fn extending_bound_start_keeps_both_messages_in_order_and_starts_above_them() {
        let source = source().replace('\n', "\r\n");
        let (edits, selected) =
            super::super::sequence_align::resize(&source, "Service", 8, 8, false, false, 7)
                .unwrap();
        let candidate = apply(&source, &edits).unwrap();
        assert!(candidate.contains("call: Client -> Service: request // keep\r\n"));
        assert!(!candidate.replace("\r\n", "").contains('\n'));
        let doc = laml::sequence::parse(&candidate).unwrap();
        let messages = doc
            .events
            .iter()
            .filter_map(|e| {
                if let Event::Message(m) = e {
                    Some(m)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(messages[0].label, None);
        assert_eq!(messages[1].label.as_deref(), Some("request"));
        assert_eq!(messages[2].label.as_deref(), Some("response"));
        assert!(selected < messages[0].line);
        let preview =
            crate::preview::build(&candidate, std::path::Path::new("resize.laml")).unwrap();
        let bar = preview
            .regions
            .iter()
            .find(|r| r.kind == "execution" && r.line == selected)
            .unwrap();
        let first = preview
            .regions
            .iter()
            .find(|r| r.kind == "sequence-receive" && r.line == messages[0].line)
            .unwrap();
        assert!(bar.y < first.y);
        let mut document = crate::document::Document::new(source.clone());
        document.apply(0, "Resize activation", edits).unwrap();
        assert!(document.undo());
        assert_eq!(document.source(), source);
        assert!(!document.undo());
    }
    #[test]
    fn implicit_reply_stays_in_order_when_its_activation_end_is_extended() {
        let source = source().replace(
            " <--: response",
            " <--: response\n Client -> Service: Later",
        );
        let (edits, selected) =
            super::super::sequence_align::resize(&source, "Service", 8, 9, true, false, 0).unwrap();
        let candidate = apply(&source, &edits).unwrap();
        assert!(candidate.contains("Client <-- Service: response"));
        let doc = laml::sequence::parse(&candidate).unwrap();
        let activation = executions(&doc)
            .into_iter()
            .find(|e| e.owner == "Service")
            .unwrap();
        assert_eq!(activation.start, selected);
        let later = doc
            .events
            .iter()
            .find_map(|e| match e {
                Event::Message(m) if m.label.as_deref() == Some("Later") => Some(m.line),
                _ => None,
            })
            .unwrap();
        assert!(activation.end.unwrap() > later);
    }
    #[test]
    fn nested_activation_cannot_be_resized_across_its_parent_end() {
        let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n A -> B+\n A -> B+\n B-\n B-\n}\n";
        assert!(super::super::sequence_align::resize(source, "B", 8, 9, true, false, 0).is_err());
    }
}
