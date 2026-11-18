//! Remove activation/lifetime boundaries without deleting their messages.
use super::*;
use laml::sequence::{Event, MessageKind};

fn erase_statement(source: &str, line: usize) -> Result<Edit, String> {
    let r = record(source, line)?;
    let raw = &source[r.range.clone()];
    let body = code(raw);
    Ok(Edit {
        range: r.range.start + body.len() - body.trim_start().len()
            ..r.range.start + body.trim_end().len(),
        replacement: String::new(),
    })
}
fn unbind(source: &str, owner: &str, line: usize, lower: bool) -> Result<Edit, String> {
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    let message = doc.events.iter().find_map(|e| {
        if let Event::Message(m) = e {
            (m.line == line).then_some(m)
        } else {
            None
        }
    });
    let Some(m) = message else {
        return erase_statement(source, line);
    };
    if let Ok(marker) = super::sequence_snap::bound_marker(source, line, owner, lower) {
        return Ok(Edit {
            range: marker.range,
            replacement: String::new(),
        });
    }
    if lower && m.kind == MessageKind::Reply {
        let fields = fields::of(source, line);
        let op = fields
            .iter()
            .find(|f| f.name == "Type")
            .ok_or("Missing reply operator")?;
        return Ok(Edit {
            range: op.range.clone(),
            replacement: format!("{} <-- {}", m.to, m.from),
        });
    }
    Err("The activation boundary cannot be detached safely".into())
}
fn finish(source: &str, edits: Vec<Edit>, affected: Vec<String>) -> Result<Deletion, String> {
    let edits = crate::element_edit::merge_insertions(edits);
    let candidate = apply(source, &edits)?;
    laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    Ok(Deletion { edits, affected })
}
pub fn activation(source: &str, owner: &str, start: usize) -> Result<Deletion, String> {
    if let Some(scope) = super::sequence_scope::Scope::of(source, owner)? {
        let plan = activation(&scope.source, &scope.owner(owner)?, scope.line(start))?;
        return Ok(Deletion {
            edits: scope.commit(source, &plan.edits)?,
            affected: plan.affected,
        });
    }
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    let e = laml::sequence::activation_layout::executions(&doc)
        .into_iter()
        .find(|e| e.owner == owner && e.start == start)
        .ok_or("The activation no longer exists")?;
    if doc
        .participants
        .iter()
        .any(|p| p.id == owner && p.created_line == Some(start))
    {
        return Err("This activation also creates the participant".into());
    }
    let mut edits = vec![unbind(source, owner, start, false)?];
    if let Some(end) = e.end {
        let destroyed = doc.events.iter().any(
            |e| matches!(e,Event::Destroy {participant,line} if participant==owner && *line==end),
        );
        if !destroyed {
            edits.push(unbind(source, owner, end, true)?);
        }
    }
    // The existing dependency analysis knows named activation references. Keep
    // its cleanup of layout/styles/annotations, never its removal of messages.
    let plan = delete(source, start, Some(&format!("line:{start}")), false)?;
    for edit in plan.edits {
        let record = records(source)
            .into_iter()
            .find(|r| r.range.start == edit.range.start);
        if record.is_some_and(|r| {
            r.section != "semantics"
                || r.heading.starts_with("at[")
                || r.heading.starts_with("between[")
                || r.heading.starts_with("order[")
        }) {
            edits.push(edit);
        }
    }
    let matches = |name: &str| {
        laml::sequence::activation_layout::resolve(&doc, name)
            .is_ok_and(|e| e.owner == owner && e.start == start)
    };
    // Layout references name the activation, not its declaration's line.
    let mut layout_edits = Vec::new();
    let mut removed = std::collections::BTreeSet::new();
    for lower in [false, true] {
        for group in laml::sequence::activation_layout::groups(&doc, lower)
            .into_iter()
            .filter(|g| g.iter().any(|name| matches(name)))
        {
            for rule in doc
                .activation_layout
                .iter()
                .filter(|r| r.lower == lower && group.contains(&r.first))
            {
                removed.insert(rule.line);
                layout_edits.push(erase_statement(source, rule.line)?);
            }
            let peers = group
                .iter()
                .filter(|name| !matches(name))
                .collect::<Vec<_>>();
            let suffix = if lower { '-' } else { '+' };
            let rules = peers
                .windows(2)
                .map(|p| format!("{}{suffix} ? {}{suffix} : align=center-y", p[0], p[1]))
                .collect::<Vec<_>>();
            if !rules.is_empty() {
                layout_edits.extend(insert(source, "layout", None, None, &rules.join("\n"))?);
            }
        }
    }
    for rule in &doc.occurrence_layout {
        if [&rule.first,&rule.second].iter().any(|e|matches!(e,laml::sequence::occurrence_layout::Endpoint::Activation {reference,..} if matches(reference))) {
            removed.insert(rule.line);
            layout_edits.push(erase_statement(source,rule.line)?);
        }
    }
    edits.retain(|e| {
        !records(source)
            .iter()
            .any(|r| removed.contains(&r.line) && r.block == e.range)
    });
    edits.extend(layout_edits);
    let result = finish(
        source,
        edits,
        vec![format!("Remove activation of {owner}; keep its messages")],
    )?;
    let new = laml::sequence::parse(&apply(source, &result.edits)?).map_err(|e| e.message)?;
    let old_messages = doc.events.iter().filter_map(|e| {
        if let Event::Message(m) = e {
            Some(m)
        } else {
            None
        }
    });
    let new_messages = new.events.iter().filter_map(|e| {
        if let Event::Message(m) = e {
            Some(m)
        } else {
            None
        }
    });
    if old_messages
        .map(|m| (&m.from, &m.to, m.kind))
        .ne(new_messages.map(|m| (&m.from, &m.to, m.kind)))
    {
        return Err("Removing this activation would change a message".into());
    }
    Ok(result)
}
pub fn destruction(source: &str, owner: &str, line: usize) -> Result<Deletion, String> {
    if let Some(scope) = super::sequence_scope::Scope::of(source, owner)? {
        let plan = destruction(&scope.source, &scope.owner(owner)?, scope.line(line))?;
        return Ok(Deletion {
            edits: scope.commit(source, &plan.edits)?,
            affected: plan.affected,
        });
    }
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    if !doc.events.iter().any(|e| matches!(e, Event::Destroy {participant,line:at} if participant == owner && *at == line)) {
        return Err("The destruction occurrence no longer exists".into());
    }
    let message = doc
        .events
        .iter()
        .any(|e| matches!(e,Event::Message(m) if m.line==line));
    let edit = if message {
        let fields = fields::of(source, line);
        let marker = fields
            .iter()
            .find(|f| f.name == "Target activation / lifetime" && f.value == "!")
            .ok_or("Missing destruction marker")?;
        Edit {
            range: marker.range.clone(),
            replacement: String::new(),
        }
    } else {
        erase_statement(source, line)?
    };
    let mut edits = vec![edit];
    for rule in doc
        .destruction_layout
        .iter()
        .filter(|r| r.first == owner || r.second == owner)
    {
        edits.push(erase_statement(source, rule.line)?);
    }
    finish(
        source,
        edits,
        vec![format!(
            "Remove destruction of {owner}; keep the participant and message"
        )],
    )
}

/// Change a regular message into an explicit creation/deletion message.
pub fn message_kind(source: &str, line: usize, create: bool) -> Result<Vec<Edit>, String> {
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    if let Some(message) = super::sequence_lookup::message(&doc, line)
        && let Some(scope) = super::sequence_scope::Scope::of(source, &message.from)?
    {
        let edits = message_kind(&scope.source, scope.line(line), create)?;
        return scope.commit(source, &edits);
    }

    let m = doc
        .events
        .iter()
        .find_map(|e| {
            if let Event::Message(m) = e {
                (m.line == line).then_some(m)
            } else {
                None
            }
        })
        .ok_or("Choose a message")?;
    if !matches!(m.kind, MessageKind::Sync | MessageKind::Async) || m.from == m.to {
        return Err("This message cannot create or destroy its target".into());
    }
    let fields = fields::of(source, line);
    let target = fields
        .iter()
        .find(|f| f.name == "Target")
        .ok_or("Choose a known target")?;
    let marker = fields
        .iter()
        .find(|f| f.name == "Target activation / lifetime");
    let mut edits = vec![Edit {
        range: marker.map_or(target.range.end..target.range.end, |f| f.range.clone()),
        replacement: if create { "+" } else { "!" }.into(),
    }];
    if create {
        let p = doc
            .participants
            .iter()
            .find(|p| p.id == m.to)
            .ok_or("Unknown target")?;
        if p.created_line.is_some() || p.line >= line {
            return Err("Choose a declared participant not already created by a message".into());
        }
        // Keeping labels/classes requires copying the authored participant token.
        let declaration = record(source, p.line)?;
        let text = code(&source[declaration.range.clone()]).trim();
        edits[0] = Edit {
            range: target.range.start..marker.map_or(target.range.end, |f| f.range.end),
            replacement: format!("{text}+"),
        };
        edits.push(erase_statement(source, p.line)?);
        let op = fields
            .iter()
            .find(|f| f.name == "Type")
            .ok_or("Missing message type")?;
        edits.push(Edit {
            range: op.range.clone(),
            replacement: if op.value.starts_with('<') {
                "<-"
            } else {
                "->"
            }
            .into(),
        });
        let r = record(source, line)?;
        let raw = code(&source[r.range.clone()]);
        let first = raw.len() - raw.trim_start().len();
        let prefix = &source[r.range.start + first
            ..fields
                .iter()
                .filter(|f| f.name == "Source" || f.name == "Target")
                .map(|f| f.range.start)
                .min()
                .unwrap()];
        let name = prefix.trim().strip_suffix(':').map(str::trim);
        if let Some(name) = name.filter(|n| !n.is_empty()) {
            let prefix = if let Some(name) = name.strip_suffix(')') {
                format!("{name}; kind=create): ")
            } else {
                format!("{name}(kind=create): ")
            };
            edits.push(Edit {
                range: r.range.start + first
                    ..r.range.start
                        + first
                        + fields
                            .iter()
                            .filter(|f| f.name == "Source" || f.name == "Target")
                            .map(|f| f.range.start)
                            .min()
                            .unwrap()
                        - r.range.start
                        - first,
                replacement: prefix,
            });
        } else {
            edits.push(Edit {
                range: r.range.start + first..r.range.start + first,
                replacement: "(kind=create): ".into(),
            });
        }
    }
    let candidate = apply(
        source,
        &crate::element_edit::merge_insertions(edits.clone()),
    )?;
    laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    Ok(crate::element_edit::merge_insertions(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deleting_activation_keeps_call_reply_comments_and_nested_bar() {
        let s = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n A -> B+ // call\n B+\n B-\n <--: response // reply\n}\n";
        let plan = activation(s, "B", 7).unwrap();
        let result = apply(s, &plan.edits).unwrap();
        assert!(result.contains("A -> B // call"));
        assert!(result.contains("A <-- B: response // reply"));
        assert_eq!(
            laml::sequence::activation_layout::executions(&laml::sequence::parse(&result).unwrap())
                .len(),
            1
        );
    }
    #[test]
    fn deleting_message_destruction_keeps_message() {
        let s = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n A -> B!: stop\n}\n";
        let result = apply(s, &destruction(s, "B", 7).unwrap().edits).unwrap();
        assert!(result.contains("A -> B: stop"));
    }
}
