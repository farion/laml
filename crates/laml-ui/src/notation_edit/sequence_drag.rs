//! Move source events while keeping message endpoints and execution pairings intact.
use super::*;
use laml::sequence::{Event, SequenceDiagram};

fn bindings(doc: &SequenceDiagram) -> Vec<(String, usize, usize)> {
    let mut open: Vec<(String, usize)> = Vec::new();
    let mut frames = Vec::new();
    let mut pairs = Vec::new();
    for event in &doc.events {
        match event {
            Event::Activate {
                participant, line, ..
            } => open.push((participant.clone(), *line)),
            Event::Deactivate {
                participant, line, ..
            } => {
                if let Some(i) = open.iter().rposition(|(id, _)| id == participant) {
                    let (id, start) = open.remove(i);
                    pairs.push((id, start, *line));
                }
            }
            Event::Destroy { participant, line } => {
                let mut i = 0;
                while i < open.len() {
                    if open[i].0 == *participant {
                        let (id, start) = open.remove(i);
                        pairs.push((id, start, *line));
                    } else {
                        i += 1;
                    }
                }
            }
            Event::FrameStart { .. } => frames.push(open.clone()),
            Event::FrameBranch { .. } => {
                if let Some(snapshot) = frames.last() {
                    open = snapshot.clone();
                }
            }
            Event::FrameEnd { .. } => {
                frames.pop();
            }
            _ => {}
        }
    }
    pairs.extend(open.into_iter().map(|(id, start)| (id, start, 0)));
    pairs
}
fn check(
    old: &SequenceDiagram,
    new: &SequenceDiagram,
    map: impl Fn(usize) -> usize,
    closed_start: Option<usize>,
) -> Result<(), String> {
    for event in &old.events {
        if let Event::Message(m) = event
            && !new.events.iter().any(|e| {
                matches!(e, Event::Message(n)
                if n.line == map(m.line) && n.from == m.from && n.to == m.to && n.kind == m.kind)
            })
        {
            return Err("This move would retarget another message or shorthand reply".into());
        }
    }
    let new_pairs = bindings(new);
    for (id, start, end) in bindings(old) {
        if !new_pairs.iter().any(|(n, a, b)| {
            *n == id
                && *a == map(start)
                && (if end == 0 && closed_start == Some(start) {
                    *b != 0
                } else {
                    *b == if end == 0 { 0 } else { map(end) }
                })
        }) {
            return Err("This move would change which activation an end belongs to. Keep nested activations inside their original parent.".into());
        }
    }
    Ok(())
}
fn insertion_parent(source: &str, before: usize) -> Result<Option<usize>, String> {
    if before == 0 {
        return Ok(None);
    }
    let at = SourceIndex::new(source)
        .line(before)
        .ok_or("Insertion point no longer exists")?
        .start;
    Ok(records(source)
        .into_iter()
        .filter(|r| r.opens && r.section == "semantics" && r.range.start < at && at < r.block.end)
        .max_by_key(|r| r.range.start)
        .map(|r| r.line))
}

pub fn move_event(source: &str, line: usize, before: usize) -> Result<(Vec<Edit>, usize), String> {
    let old = laml::sequence::parse(source).map_err(|e| e.message)?;
    let r = record(source, line)?;
    if r.section != "semantics" {
        return Err("Choose an interaction element".into());
    }
    if insertion_parent(source, before)? != r.parent {
        return Err("Move within the same fragment operand".into());
    }
    let raw = &source[r.range.clone()];
    let indent = raw.len() - raw.trim_start().len();
    let body = source[r.block.clone()]
        .lines()
        .map(|l| l.get(indent..).unwrap_or(l))
        .collect::<Vec<_>>()
        .join("\n");
    let mut edits = super::sequence_insert::at(source, before, &body)?;
    let at = edits[0].range.start;
    if at == r.block.start || at == r.block.end {
        return Ok((vec![], line));
    }
    if r.block.contains(&at) {
        return Err("Cannot move an element inside itself".into());
    }
    let mapped_at = if at >= r.block.end {
        at - r.block.len()
    } else {
        at
    };
    edits.push(Edit {
        range: r.block.clone(),
        replacement: String::new(),
    });
    let candidate = apply(source, &edits)?;
    let new_line = candidate[..mapped_at]
        .bytes()
        .filter(|b| *b == b'\n')
        .count()
        + 1;
    let new = laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    let map = |old_line| {
        let offset = SourceIndex::new(source).line(old_line).map(|r| r.start);
        if offset.is_some_and(|at| r.block.contains(&at)) {
            new_line + old_line - line
        } else {
            crate::relation_edit::remap_line(source, old_line, &edits)
        }
    };
    check(&old, &new, map, None)?;
    Ok((edits, new_line))
}

pub fn close_open(source: &str, start: usize, before: usize) -> Result<(Vec<Edit>, usize), String> {
    let old = laml::sequence::parse(source).map_err(|e| e.message)?;
    let pairs = bindings(&old);
    let (owner, _, _) = pairs
        .iter()
        .find(|(_, a, b)| *a == start && *b == 0)
        .ok_or("This activation already has an end")?;
    let r = record(source, start)?;
    if insertion_parent(source, before)? != r.parent {
        return Err("End the activation in the same fragment operand".into());
    }
    let edits = super::sequence_insert::at(source, before, &format!("{owner}-"))?;
    let candidate = apply(source, &edits)?;
    let new = laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    let map = |line| crate::relation_edit::remap_line(source, line, &edits);
    check(&old, &new, map, Some(start))?;
    Ok((edits.clone(), map(start)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source(events: &str) -> String {
        format!("settings {{\n type: sequence\n}}\nsemantics {{\n A\n B\n{events}\n}}\n")
    }
    fn line(source: &str, text: &str) -> usize {
        source.lines().position(|s| s.trim() == text).unwrap() + 1
    }
    #[test]
    fn resize_preserves_bound_call_and_its_end() {
        let s = source(" A+\n A-\n call: A -> B+ : work\n B-");
        let (edits, moved) =
            move_event(&s, line(&s, "call: A -> B+ : work"), line(&s, "A-")).unwrap();
        let candidate = apply(&s, &edits).unwrap();
        assert_eq!(
            candidate.lines().nth(moved - 1).unwrap().trim(),
            "call: A -> B+ : work"
        );
        assert!(candidate.find("call:").unwrap() < candidate.find(" A-").unwrap());
    }
    #[test]
    fn nested_end_cannot_silently_close_a_different_activation() {
        let s = source(" outer: A+\n inner: A+\n A-\n A-");
        let inner_end = line(&s, "A-");
        assert!(move_event(&s, inner_end, 0).is_err());
    }
    #[test]
    fn open_execution_gets_an_explicit_end() {
        let s = source(" A+");
        let (edits, _) = close_open(&s, line(&s, "A+"), 0).unwrap();
        let candidate = apply(&s, &edits).unwrap();
        assert!(candidate.contains("A-"));
        assert!(
            bindings(&laml::sequence::parse(&candidate).unwrap())
                .iter()
                .all(|(_, _, end)| *end != 0)
        );
    }
    #[test]
    fn moving_a_declaration_keeps_comments_and_crlf() {
        let s = source(" A+ // start\n B+\n B-\n A-").replace('\n', "\r\n");
        let (edits, _) = move_event(&s, line(&s, "A+ // start"), line(&s, "B-")).unwrap();
        let candidate = apply(&s, &edits).unwrap();
        assert!(candidate.contains("A+ // start\r\n"));
        assert!(!candidate.replace("\r\n", "").contains('\n'));
    }
}
