//! Source transactions for the rebuilt sequence interactions. No canvas state.
use super::*;
use laml::sequence::{Event, Message, MessageKind};

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Boundary {
    Start(usize),
    End { start: usize, end: Option<usize> },
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Dock {
    pub owner: String,
    pub before: usize,
    pub boundary: Option<Boundary>,
}

pub fn participant(
    source: &str,
    id: &str,
    label: &str,
    neighbor: Option<(&str, bool)>,
) -> Result<Vec<Edit>, String> {
    if let Some((owner, left)) = neighbor
        && let Some(scope) = super::sequence_scope::Scope::of(source, owner)?
    {
        let edits = participant(&scope.source, id, label, Some((&scope.owner(owner)?, left)))?;
        return scope.commit(source, &edits);
    }
    if !crate::graph_edit::identifier(id) {
        return Err("Use a simple identifier".into());
    }
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    if doc.participants.iter().any(|p| p.id == id) {
        return Err("Choose an unused identifier".into());
    }
    let first = records(source)
        .into_iter()
        .find(|r| r.section == "semantics" && r.parent.is_none())
        .map(|r| r.line);
    let mut edits = insert(
        source,
        "semantics",
        None,
        first,
        &format!(
            "{id}(label={})",
            crate::relation_edit::quote(if label.is_empty() { id } else { label })
        ),
    )?;
    let mut order: Vec<_> = doc.participants.iter().map(|p| p.id.as_str()).collect();
    let index = if let Some((near, left)) = neighbor {
        order
            .iter()
            .position(|p| *p == near)
            .ok_or("Participant no longer exists")?
            + usize::from(!left)
    } else {
        order.len()
    };
    order.insert(index, id);
    let rules = order
        .windows(2)
        .map(|p| format!("{} < {}", p[0], p[1]))
        .collect::<Vec<_>>();
    if !rules.is_empty() {
        edits.extend(insert(source, "layout", None, None, &rules.join("\n"))?);
    }
    checked(source, edits, None)
}

fn messages(doc: &laml::sequence::SequenceDiagram) -> Vec<&Message> {
    doc.events
        .iter()
        .filter_map(|e| {
            if let Event::Message(m) = e {
                Some(m)
            } else {
                None
            }
        })
        .collect()
}
fn checked(source: &str, edits: Vec<Edit>, changed: Option<usize>) -> Result<Vec<Edit>, String> {
    let edits = crate::element_edit::merge_insertions(edits);
    let candidate = apply(source, &edits)?;
    let old = laml::sequence::parse(source).map_err(|e| e.message)?;
    let new = laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    for m in messages(&old)
        .into_iter()
        .filter(|m| Some(m.line) != changed)
    {
        let line = crate::relation_edit::remap_line(source, m.line, &edits);
        if !messages(&new)
            .iter()
            .any(|n| n.line == line && n.from == m.from && n.to == m.to && n.kind == m.kind)
        {
            return Err("This change would retarget an existing message or reply".into());
        }
    }
    Ok(edits)
}
fn bind(
    source: &str,
    message: usize,
    dock: &Dock,
    receive: bool,
) -> Result<(Vec<Edit>, usize), String> {
    if dock.boundary.is_none() {
        return Ok((vec![], message));
    }
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    let ordinal = messages(&doc)
        .iter()
        .position(|m| m.line == message)
        .ok_or("Missing message occurrence")?;
    let (edits, _) = super::sequence_docking::align(source, message, dock, receive)?;
    let candidate = apply(source, &edits)?;
    let selected = super::sequence_docking::message_line(&candidate, ordinal)?;
    Ok((edits, selected))
}
fn mapped(source: &str, dock: &Dock, edits: &[Edit]) -> Dock {
    let map = |line| {
        if line == 0 {
            0
        } else {
            crate::relation_edit::remap_line(source, line, edits)
        }
    };
    if let Some(boundary) = &dock.boundary {
        let start = match boundary {
            Boundary::Start(s) => *s,
            Boundary::End { start, .. } => *start,
        };
        if let (Ok(old), Ok(candidate)) = (laml::sequence::parse(source), apply(source, edits)) {
            let old = laml::sequence::activation_layout::executions(&old);
            if let Some(index) = old
                .iter()
                .filter(|e| e.owner == dock.owner)
                .position(|e| e.start == start)
                && let Ok(new) = laml::sequence::parse(&candidate)
            {
                let new = laml::sequence::activation_layout::executions(&new);
                if let Some(e) = new.iter().filter(|e| e.owner == dock.owner).nth(index) {
                    let boundary = match boundary {
                        Boundary::Start(_) => Boundary::Start(e.start),
                        Boundary::End { .. } => Boundary::End {
                            start: e.start,
                            end: e.end,
                        },
                    };
                    return Dock {
                        owner: dock.owner.clone(),
                        before: match boundary {
                            Boundary::Start(s) => s,
                            Boundary::End { end, .. } => end.unwrap_or(0),
                        },
                        boundary: Some(boundary),
                    };
                }
            }
        }
    }
    Dock {
        owner: dock.owner.clone(),
        before: map(dock.before),
        boundary: dock.boundary.as_ref().map(|b| match *b {
            Boundary::Start(s) => Boundary::Start(map(s)),
            Boundary::End { start, end } => Boundary::End {
                start: map(start),
                end: end.map(map),
            },
        }),
    }
}

pub fn message(source: &str, from: &Dock, to: &Dock, operator: &str) -> Result<Vec<Edit>, String> {
    message_with_line(source, from, to, operator).map(|(edits, _)| edits)
}
pub fn message_with_line(
    source: &str,
    from: &Dock,
    to: &Dock,
    operator: &str,
) -> Result<(Vec<Edit>, usize), String> {
    if let Some(scope) = super::sequence_scope::Scope::of(source, &from.owner)? {
        let (edits, line) = message_with_line(
            &scope.source,
            &scope.dock(from)?,
            &scope.dock(to)?,
            operator,
        )?;
        return Ok((scope.commit(source, &edits)?, scope.absolute(line)));
    }
    if !["->", "->>", "<--"].contains(&operator) {
        return Err("Choose a supported message type".into());
    }
    if super::sequence_drag::insertion_parent(source, from.before)?
        != super::sequence_drag::insertion_parent(source, to.before)?
    {
        return Err("Messages must stay inside the same fragment operand".into());
    }
    let declaration = if operator == "<--" {
        format!("{} <-- {}", to.owner, from.owner)
    } else {
        format!("{} {operator} {}", from.owner, to.owner)
    };
    let before = if from.owner == to.owner {
        to.before
    } else {
        super::sequence_docking::insertion_before(source, to)?
    };
    let mut edits = super::sequence_insert::at(source, before, &declaration)?;
    let line = source[..edits[0].range.start]
        .bytes()
        .filter(|b| *b == b'\n')
        .count()
        + 1;
    let mut candidate = apply(source, &edits)?;
    // Starting a message chooses its sender and operand, not an activation
    // mutation. In particular, named/labelled sender activations stay intact.
    let to = mapped(source, to, &edits);
    let (binding, line) = bind(&candidate, line, &to, true)?;
    candidate = apply(&candidate, &binding)?;
    // A new ordinary message stays horizontal explicitly; a later source-end
    // reconnection can replace this height binding independently.
    if from.owner != to.owner && to.boundary.is_some() {
        let (horizontal, _) = super::sequence_docking::align(&candidate, line, &to, false)?;
        candidate = apply(&candidate, &horizontal)?;
    }
    edits = crate::graph_edit::difference(source, &candidate);
    // Difference can span old lines; compare the ordered pre-existing messages
    // using the insert/bind operations' semantic endpoint invariants instead.
    let old = laml::sequence::parse(source).map_err(|e| e.message)?;
    let new = laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    let mut actual = messages(&new);
    let inserted = actual
        .iter()
        .position(|m| m.line == line)
        .ok_or("New message could not be identified")?;
    actual.remove(inserted);
    if messages(&old)
        .iter()
        .map(|m| (&m.from, &m.to, m.kind))
        .ne(actual.iter().map(|m| (&m.from, &m.to, m.kind)))
    {
        return Err("This insertion would change an existing message or reply".into());
    }
    Ok((edits, line))
}

pub fn reconnect(
    source: &str,
    line: usize,
    moving_source: bool,
    dock: &Dock,
    same_row: bool,
) -> Result<(Vec<Edit>, usize), String> {
    if let Some(scope) = super::sequence_scope::Scope::of(source, &dock.owner)? {
        let (edits, line) = reconnect(
            &scope.source,
            scope.line(line),
            moving_source,
            &scope.dock(dock)?,
            same_row,
        )?;
        return Ok((scope.commit(source, &edits)?, scope.absolute(line)));
    }
    if dock.boundary.is_some() {
        let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
        if messages(&doc)
            .iter()
            .any(|m| m.line == line && (if moving_source { &m.from } else { &m.to }) == &dock.owner)
            && super::sequence_docking::align(source, line, dock, !moving_source)
                .is_ok_and(|(e, _)| e.is_empty())
        {
            return Ok((vec![], line));
        }
    }
    // Moving an endpoint of its own marker-bearing self-message can transfer
    // that marker between send and receive, rather than equating both heights.
    if matches!(dock.boundary, Some(Boundary::Start(start)) if start == line) {
        return super::sequence_reconnect::reconnect(
            source,
            line,
            moving_source,
            &dock.owner,
            None,
            Some(line),
        );
    }
    if matches!(dock.boundary, Some(Boundary::End {end:Some(end),..}) if end == line) {
        return super::sequence_activation::bind_end(
            source,
            vec![],
            Some(line),
            &dock.owner,
            line,
            !moving_source,
        );
    }
    let detached = super::sequence_docking::detach(source, line, !moving_source)?;
    let intermediate = apply(source, &detached)?;
    let mapped_line = crate::relation_edit::remap_line(source, line, &detached);
    let mut dock = mapped(source, dock, &detached);
    let before = if same_row || dock.boundary.is_some() {
        None
    } else {
        Some(dock.before)
    };
    let result = super::sequence_reconnect::reconnect(
        &intermediate,
        mapped_line,
        moving_source,
        &dock.owner,
        before,
        None,
    );
    let (movement, selected) = match result {
        Err(_) if dock.boundary.is_some() => super::sequence_reconnect::reconnect(
            &intermediate,
            mapped_line,
            moving_source,
            &dock.owner,
            Some(super::sequence_docking::insertion_before(
                &intermediate,
                &dock,
            )?),
            None,
        )?,
        result => result?,
    };
    dock = mapped(&intermediate, &dock, &movement);
    let mut candidate = apply(&intermediate, &movement)?;
    let (binding, _) = super::sequence_docking::align(&candidate, selected, &dock, !moving_source)?;
    let ordinal = messages(&laml::sequence::parse(&candidate).map_err(|e| e.message)?)
        .iter()
        .position(|m| m.line == selected)
        .ok_or("Missing reconnected message")?;
    candidate = apply(&candidate, &binding)?;
    let selected = super::sequence_docking::message_line(&candidate, ordinal)?;
    Ok((crate::graph_edit::difference(source, &candidate), selected))
}

pub fn message_type(source: &str, line: usize, operator: &str) -> Result<Vec<Edit>, String> {
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    let found = super::sequence_lookup::message(&doc, line).ok_or("Choose a message")?;
    if let Some(scope) = super::sequence_scope::Scope::of(source, &found.from)? {
        let edits = message_type(&scope.source, scope.line(line), operator)?;
        return scope.commit(source, &edits);
    }
    let m = &found;
    if !["->", "->>", "<--"].contains(&operator)
        || matches!(m.kind, MessageKind::Create | MessageKind::Delete)
    {
        return Err("This message type cannot be changed without changing its lifetime".into());
    }
    let fields = fields::of(source, line);
    let op = fields
        .iter()
        .find(|f| f.name == "Type")
        .ok_or("Missing message operator")?;
    let marker = |name: &str| {
        fields
            .iter()
            .find(|f| f.name == format!("{name} activation / lifetime"))
            .map_or("", |f| f.value.as_str())
    };
    let from = format!("{}{}", m.from, marker("Source"));
    let to = format!("{}{}", m.to, marker("Target"));
    let (left, right) = if operator == "<--" {
        (to, from)
    } else {
        (from, to)
    };
    let mut edits = vec![Edit {
        range: op.range.clone(),
        replacement: operator.into(),
    }];
    let endpoints: Vec<_> = fields
        .iter()
        .filter(|f| f.name == "Source" || f.name == "Target")
        .collect();
    if endpoints.len() == 2 {
        let mut endpoints = endpoints;
        endpoints.sort_by_key(|f| f.range.start);
        for (field, value) in endpoints.into_iter().zip([left, right]) {
            let suffix = fields.iter().find(|f| {
                f.range.start == field.range.end && f.name.ends_with("activation / lifetime")
            });
            edits.push(Edit {
                range: field.range.start..suffix.map_or(field.range.end, |f| f.range.end),
                replacement: value,
            });
        }
    } else {
        // Expand a shorthand reply without losing its implicit activation end.
        let closes = doc.events.iter().any(|e| matches!(e, Event::Deactivate { line: at, participant, .. } if *at == line && *participant == m.from));
        edits[0].replacement = if operator == "<--" {
            format!("{} <-- {}{}", m.to, m.from, if closes { "-" } else { "" })
        } else {
            format!(
                "{}{} {operator} {}",
                m.from,
                if closes { "-" } else { "" },
                m.to
            )
        };
    }
    let edits = checked(source, edits, Some(line))?;
    let candidate = apply(source, &edits)?;
    let new = laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    if !messages(&new)
        .iter()
        .any(|n| n.line == line && n.from == m.from && n.to == m.to)
    {
        return Err("Type change would reverse the endpoints".into());
    }
    Ok(edits)
}

pub fn destroy(source: &str, owner: &str, before: usize) -> Result<Vec<Edit>, String> {
    if let Some(scope) = super::sequence_scope::Scope::of(source, owner)? {
        let edits = destroy(&scope.source, &scope.owner(owner)?, scope.line(before))?;
        return scope.commit(source, &edits);
    }
    let edits = super::sequence_insert::at(source, before, &format!("{owner}!"))?;
    checked(source, edits, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    const SOURCE: &str = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n}\n";
    fn dock(owner: &str, before: usize) -> Dock {
        Dock {
            owner: owner.into(),
            before,
            boundary: None,
        }
    }
    #[test]
    fn participants_insert_on_both_sides_and_in_empty_diagram() {
        for left in [true, false] {
            let edits = participant(SOURCE, "C", "See", Some(("A", left))).unwrap();
            let s = apply(SOURCE, &edits).unwrap();
            let d = laml::sequence::parse(&s).unwrap();
            let ids: Vec<_> = d.participants.iter().map(|p| p.id.as_str()).collect();
            assert_eq!(
                ids,
                if left {
                    vec!["C", "A", "B"]
                } else {
                    vec!["A", "C", "B"]
                }
            );
        }
        let empty = "settings {\n type: sequence\n}\nsemantics {\n}\n";
        assert!(participant(empty, "First", "", None).is_ok());
    }
    #[test]
    fn message_creation_and_type_changes_preserve_direction() {
        let edits = message(SOURCE, &dock("A", 0), &dock("B", 0), "->").unwrap();
        let s = apply(SOURCE, &edits).unwrap();
        let d = laml::sequence::parse(&s).unwrap();
        let line = messages(&d)[0].line;
        for op in ["->>", "<--", "->"] {
            let s = apply(&s, &message_type(&s, line, op).unwrap()).unwrap();
            let d = laml::sequence::parse(&s).unwrap();
            assert_eq!((&*messages(&d)[0].from, &*messages(&d)[0].to), ("A", "B"));
        }
    }
    #[test]
    fn new_message_binds_existing_activation_without_creating_a_second() {
        let s = SOURCE.replace(" B\n", " B\n B+\n B-\n");
        let to = Dock {
            owner: "B".into(),
            before: 7,
            boundary: Some(Boundary::Start(7)),
        };
        let s = apply(&s, &message(&s, &dock("A", 7), &to, "->").unwrap()).unwrap();
        let d = laml::sequence::parse(&s).unwrap();
        assert_eq!(laml::sequence::activation_layout::executions(&d).len(), 1);
        assert!(s.contains(" B+\n"));
        assert!(s.contains(".receive ? B+ : align=center-y; reference=second"));
    }
}
