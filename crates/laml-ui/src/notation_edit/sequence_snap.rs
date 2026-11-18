//! Dock an activation edge exactly onto a message, retaining source event order.
#[cfg(test)]
mod dual_tests;
#[cfg(test)]
mod self_tests;
use super::*;
use laml::sequence::{Event, MessageKind, activation_layout::executions};

fn remove_declaration(source: &str, line: usize) -> Result<Edit, String> {
    let r = record(source, line)?;
    let raw = &source[r.range.clone()];
    let body = code(raw);
    let range = if raw[body.trim_end().len()..].trim().is_empty() {
        r.block
    } else {
        r.range.start + body.len() - body.trim_start().len()..r.range.start + body.trim_end().len()
    };
    Ok(Edit {
        range,
        replacement: String::new(),
    })
}

fn explicit_reply(source: &str, line: usize, marker: &str) -> Result<Edit, String> {
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    let m = doc
        .events
        .iter()
        .find_map(|e| match e {
            Event::Message(m) if m.line == line => Some(m),
            _ => None,
        })
        .ok_or("Missing reply")?;
    let field = fields::of(source, line)
        .into_iter()
        .find(|f| f.name == "Type")
        .ok_or("Missing reply operator")?;
    Ok(Edit {
        range: field.range,
        replacement: format!("{} <-- {}{marker}", m.to, m.from),
    })
}

pub(super) fn bound_marker(
    source: &str,
    line: usize,
    owner: &str,
    lower: bool,
) -> Result<fields::Field, String> {
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    let receive = doc
        .events
        .iter()
        .find_map(|e| match e {
            Event::Activate {
                participant,
                at_send,
                line: at,
                ..
            } if !lower && participant == owner && *at == line => Some(!at_send),
            Event::Deactivate {
                participant,
                at_send,
                line: at,
                ..
            } if lower && participant == owner && *at == line => Some(!at_send),
            _ => None,
        })
        .ok_or("The bound activation edge no longer exists")?;
    marker_field(source, line, owner, receive)
}

fn marker_field(
    source: &str,
    line: usize,
    owner: &str,
    receive: bool,
) -> Result<fields::Field, String> {
    let fields = fields::of(source, line);
    let name = if receive { "Target" } else { "Source" };
    fields
        .iter()
        .find(|f| f.name == name && f.value == owner)
        .ok_or("This message endpoint does not touch the activation")?;
    fields
        .iter()
        .find(|f| f.name == format!("{name} activation / lifetime"))
        .cloned()
        .ok_or("Missing endpoint marker".into())
}

fn mark(
    source: &str,
    line: usize,
    owner: &str,
    symbol: &str,
    receive: bool,
) -> Result<Vec<Edit>, String> {
    if !fields::of(source, line).iter().any(|f| f.name == "Source") {
        let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
        let m = doc
            .events
            .iter()
            .find_map(|e| match e {
                Event::Message(m) if m.line == line => Some(m),
                _ => None,
            })
            .ok_or("Missing message")?;
        if (if receive { &m.to } else { &m.from }) != owner {
            return Err("This reply endpoint does not touch the activation".into());
        }
        let closes = doc.events.iter().any(|e| matches!(e, Event::Deactivate { line: at, participant, .. } if *at == line && participant == &m.from));
        if closes && !receive {
            return Err("This message endpoint already ends an activation".into());
        }
        let operator = fields::of(source, line)
            .into_iter()
            .find(|f| f.name == "Type")
            .ok_or("Missing reply operator")?;
        let left = if receive { symbol } else { "" };
        let right = if !receive {
            symbol
        } else if closes {
            "-"
        } else {
            ""
        };
        return Ok(vec![Edit {
            range: operator.range,
            replacement: format!("{}{left} <-- {}{right}", m.to, m.from),
        }]);
    }
    let marker = marker_field(source, line, owner, receive)?;
    if !marker.value.is_empty() {
        return Err(
            "This message endpoint already changes an activation or participant lifetime".into(),
        );
    }
    fields::edit(source, &marker, symbol)
}

pub(super) fn dock(
    source: &str,
    owner: &str,
    start: usize,
    edge: usize,
    lower: bool,
    message: usize,
    receive: bool,
) -> Result<(Vec<Edit>, usize), String> {
    let old = laml::sequence::parse(source).map_err(|e| e.message)?;
    let execution = executions(&old)
        .into_iter()
        .find(|e| e.owner == owner && e.start == start)
        .ok_or("The activation no longer exists")?;
    if (!lower && start == message) || (lower && execution.end == Some(message)) {
        let at_send = old
            .events
            .iter()
            .find_map(|e| match e {
                Event::Activate {
                    line,
                    participant,
                    at_send,
                    ..
                } if !lower && *line == message && participant == owner => Some(*at_send),
                Event::Deactivate {
                    line,
                    participant,
                    at_send,
                    ..
                } if lower && *line == message && participant == owner => Some(*at_send),
                _ => None,
            })
            .ok_or("The bound occurrence no longer exists")?;
        if at_send != receive {
            return Ok((vec![], start));
        }
        // A self-call has two distinct occurrences on the same source line.
        let symbol = if lower { "-" } else { "+" };
        let marker = bound_marker(source, message, owner, lower)?;
        let clean = apply(
            source,
            &[Edit {
                range: marker.range,
                replacement: String::new(),
            }],
        )?;
        let edits = mark(&clean, message, owner, symbol, receive)?;
        let candidate = apply(&clean, &edits)?;
        let new = laml::sequence::parse(&candidate).map_err(|e| e.message)?;
        let mut expected = old.events.clone();
        for event in &mut expected {
            match event {
                Event::Activate {
                    line,
                    participant,
                    at_send,
                    ..
                } if !lower && *line == message && participant == owner => *at_send = !receive,
                Event::Deactivate {
                    line,
                    participant,
                    at_send,
                    ..
                } if lower && *line == message && participant == owner => *at_send = !receive,
                _ => {}
            }
        }
        if new.events != expected {
            return Err("Switching this occurrence would change another event".into());
        }
        return Ok((crate::graph_edit::difference(source, &candidate), start));
    }
    let target = old
        .events
        .iter()
        .find_map(|e| match e {
            Event::Message(m) if m.line == message => Some(m),
            _ => None,
        })
        .ok_or("Choose a message")?;
    if !lower && execution.name.is_some() && execution.name != target.name {
        return Err("Keep this named activation's declaration, or match the message name before binding its start".into());
    }
    if old
        .events
        .iter()
        .any(|e| matches!(e, Event::Activate { line, label: Some(_), .. } if *line == edge))
    {
        return Err("Keep this activation's labelled declaration when resizing its start".into());
    }
    if old
        .participants
        .iter()
        .any(|p| p.id == owner && (p.created_line == Some(edge) || p.destroyed_line == Some(edge)))
    {
        return Err("This edge also creates or destroys the participant".into());
    }
    if record(source, edge)?.parent != record(source, message)?.parent {
        return Err("Resize within the same fragment operand".into());
    }
    let symbol = if lower { "-" } else { "+" };
    let mut edits = mark(source, message, owner, symbol, receive)?;
    if lower && execution.end.is_none() {
        // Closing an open bar needs no old end to unlink.
    } else if let Some(bound) = old.events.iter().find_map(|e| match e {
        Event::Message(m) if m.line == edge => Some(m),
        _ => None,
    }) {
        if let Ok(marker) = bound_marker(source, edge, owner, lower) {
            edits.push(Edit {
                range: marker.range,
                replacement: String::new(),
            });
        } else if lower && bound.kind == MessageKind::Reply {
            edits.push(explicit_reply(source, edge, "")?);
        } else {
            return Err("This bound edge cannot be moved independently".into());
        }
    } else {
        edits.push(remove_declaration(source, edge)?);
    }
    let candidate = apply(source, &edits)?;
    let new = laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    let map = |line| crate::relation_edit::remap_line(source, line, &edits);
    let messages = |doc: &laml::sequence::SequenceDiagram| {
        doc.events
            .iter()
            .filter_map(|e| match e {
                Event::Message(m) => Some((
                    m.line,
                    m.from.clone(),
                    m.to.clone(),
                    m.kind,
                    m.name.clone(),
                    m.label.clone(),
                )),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let expected = messages(&old)
        .into_iter()
        .map(|(line, a, b, k, n, l)| (map(line), a, b, k, n, l))
        .collect::<Vec<_>>();
    if messages(&new) != expected {
        return Err("Docking this edge would change another message or shorthand reply".into());
    }
    let new_pairs = super::sequence_drag::bindings(&new);
    let old_pairs = super::sequence_drag::bindings(&old);
    if old_pairs.len() != new_pairs.len() {
        return Err("This message already belongs to another activation edge".into());
    }
    for (id, a, b) in old_pairs {
        let edited = id == owner && a == start;
        let a = if edited && !lower {
            map(message)
        } else {
            map(a)
        };
        let b = if edited && lower {
            map(message)
        } else if b == 0 {
            0
        } else {
            map(b)
        };
        if !new_pairs
            .iter()
            .any(|(p, s, e)| *p == id && *s == a && *e == b)
        {
            return Err("Docking this edge would change an activation's pairing".into());
        }
    }
    let selected = if lower { map(start) } else { map(message) };
    Ok((edits, selected))
}

#[cfg(test)]
mod tests {
    use super::*;
    const SOURCE: &str = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n A -> B\n A -> B+: request // keep\n <--: response\n}\n";
    fn check(
        source: &str,
        start: usize,
        edge: usize,
        lower: bool,
        target: usize,
    ) -> (String, usize) {
        let (edits, selected) =
            super::super::sequence_align::snap_to_message(source, "B", start, edge, lower, target)
                .unwrap();
        let candidate = apply(source, &edits).unwrap();
        let preview = crate::preview::build(&candidate, std::path::Path::new("snap.laml")).unwrap();
        let bar = preview
            .regions
            .iter()
            .find(|r| r.kind == "execution" && r.line == selected)
            .unwrap();
        let doc = laml::sequence::parse(&candidate).unwrap();
        let e = executions(&doc)
            .into_iter()
            .find(|e| e.start == selected && e.owner == "B")
            .unwrap();
        let line = if lower { e.end.unwrap() } else { e.start };
        let endpoint = preview
            .regions
            .iter()
            .find(|r| {
                r.line == line
                    && r.id == "B"
                    && matches!(r.kind.as_str(), "sequence-send" | "sequence-receive")
            })
            .unwrap();
        let edge_y = if lower { bar.y + bar.height } else { bar.y };
        assert!((edge_y - endpoint.y).abs() < 0.01);
        let mut document = crate::document::Document::new(source.into());
        document.apply(0, "Snap activation edge", edits).unwrap();
        assert!(document.undo());
        assert_eq!(document.source(), source);
        (candidate, selected)
    }
    #[test]
    fn bound_start_snaps_exactly_to_upper_arrow_without_moving_messages() {
        let (candidate, selected) = check(SOURCE, 8, 8, false, 7);
        assert_eq!(selected, 7);
        assert!(candidate.contains(" A -> B+\n A -> B: request // keep\n"));
    }
    #[test]
    fn standalone_start_snaps_exactly_to_message_with_crlf_and_comments() {
        let source = SOURCE
            .replace(" A -> B+: request", " B+ // activation\n A -> B: request")
            .replace('\n', "\r\n");
        let (candidate, _) = check(&source, 8, 8, false, 7);
        assert!(candidate.contains("// activation\r\n"));
        assert!(!candidate.replace("\r\n", "").contains('\n'));
    }
    #[test]
    fn lower_edge_snaps_to_later_message_without_moving_reply() {
        let source = SOURCE.replace(" <--: response", " <--: response\n A -> B: Later");
        let (candidate, _) = check(&source, 8, 9, true, 10);
        assert!(candidate.contains("A <-- B: response\n A -> B-: Later"));
    }
    #[test]
    fn open_activation_can_end_exactly_on_an_arrow() {
        let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n B+\n A -> B: End\n}\n";
        let (candidate, _) = check(source, 7, 7, true, 8);
        assert!(candidate.contains("A -> B-: End"));
    }
    #[test]
    fn snapping_cannot_steal_another_activation_or_cross_nested_pairs() {
        let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n A -> B+\n A -> B+\n B-\n B-\n}\n";
        assert!(
            super::super::sequence_align::snap_to_message(source, "B", 8, 8, false, 7).is_err()
        );
    }
}
