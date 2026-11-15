//! Structural changes preserve unrelated bytes and are preflighted by the compiler.
use super::*;
use std::collections::BTreeSet;

pub fn move_record(
    source: &str,
    line: usize,
    parent: Option<usize>,
    before: Option<usize>,
) -> Result<Vec<Edit>, String> {
    let r = record(source, line)?;
    if parent.is_some_and(|p| record(source, p).is_ok_and(|p| r.block.contains(&p.range.start)))
        || before.is_some_and(|p| record(source, p).is_ok_and(|p| r.block.contains(&p.range.start)))
    {
        return Err("A construct cannot be moved into itself".into());
    }
    let raw = &source[r.block.clone()];
    let indent = source[r.range.clone()].len() - source[r.range.clone()].trim_start().len();
    let content = raw
        .lines()
        .map(|l| l.get(indent..).unwrap_or(l))
        .collect::<Vec<_>>()
        .join("\n");
    let mut edits = insert(source, &r.section, parent, before, &content)?;
    if edits[0].range.start == r.block.end || edits[0].range.start == r.block.start {
        return Ok(vec![]);
    }
    edits.push(Edit {
        range: r.block,
        replacement: String::new(),
    });
    Ok(edits)
}
pub fn reorder(source: &str, line: usize, down: bool) -> Result<Vec<Edit>, String> {
    let all = records(source);
    let r = record(source, line)?;
    let mut siblings: Vec<Record> = Vec::new();
    for entry in all
        .iter()
        .filter(|s| s.section == r.section && s.parent == r.parent)
    {
        if siblings
            .last()
            .is_some_and(|last| entry.range.start < last.block.end)
        {
            continue;
        }
        siblings.push(record(source, entry.line)?);
    }
    let at = siblings
        .iter()
        .position(|s| s.line == line)
        .ok_or("Missing construct")?;
    let target = if down {
        siblings.get(at + 1)
    } else {
        at.checked_sub(1).and_then(|i| siblings.get(i))
    }
    .ok_or("Already at the end of this container")?;
    let (first, second) = if down { (&r, target) } else { (target, &r) };
    let middle = &source[first.block.end..second.block.start];
    Ok(vec![Edit {
        range: first.block.start..second.block.end,
        replacement: format!(
            "{}{middle}{}",
            &source[second.block.clone()],
            &source[first.block.clone()]
        ),
    }])
}
#[derive(Clone, Debug)]
pub struct Deletion {
    pub edits: Vec<Edit>,
    pub affected: Vec<String>,
}
pub fn delete(
    source: &str,
    line: usize,
    id: Option<&str>,
    keep_contents: bool,
) -> Result<Deletion, String> {
    let r = record(source, line)?;
    if keep_contents && !r.opens {
        return Err("This construct has no contents".into());
    }
    let all = records(source);
    if keep_contents {
        let replacement = unpack(source, &r, &all)?;
        return Ok(Deletion {
            edits: vec![Edit {
                range: r.block,
                replacement,
            }],
            affected: vec![format!(
                "Remove {} and its branch wrappers; keep all contents",
                r.heading
            )],
        });
    }
    let mut lines = BTreeSet::from([line]);
    let mut names = BTreeSet::new();
    let notation = crate::notation::Notation::of(source);
    let sequence = if notation == crate::notation::Notation::Sequence {
        laml::sequence::parse(source).ok()
    } else {
        None
    };
    let mut namespace = BTreeSet::new();
    if let Some(doc) = &sequence {
        super::sequence_dependencies::identities(doc, "", &mut namespace);
    }
    if let Ok(doc) = laml::bpmn::parse(source) {
        namespace.extend(doc.nodes.iter().map(|n| n.id.clone()));
        namespace.extend(doc.pools.iter().map(|p| p.id.clone()));
    }

    if let Ok(graph) = crate::notation::parse_graph(source) {
        namespace.extend(graph.nodes.iter().map(|n| n.id.clone()));
        for edge in graph.relations.iter().filter(|edge| {
            all.iter()
                .find(|r| r.line == edge.line)
                .is_some_and(|entry| r.block.contains(&entry.range.start))
        }) {
            if let Some(name) = &edge.name {
                names.insert(name.clone());
            }
        }
        for node in graph.nodes.iter().filter(|n| {
            all.iter()
                .find(|r| r.line == n.line)
                .is_some_and(|n| r.block.contains(&n.range.start))
        }) {
            names.insert(node.id.clone());
            let short = node.id.rsplit('.').next().unwrap();
            if graph
                .nodes
                .iter()
                .filter(|n| n.id.rsplit('.').next() == Some(short))
                .count()
                == 1
            {
                names.insert(short.into());
            }
        }
    }
    if let Some(id) = id.filter(|id| !id.starts_with("line:") && !id.starts_with("flow:")) {
        names.insert(id.to_owned());
        // Unqualified references are safe only for a uniquely declared local name.
        let short = id.rsplit('.').next().unwrap();
        if namespace
            .iter()
            .filter(|id| id.rsplit('.').next() == Some(short))
            .count()
            <= 1
        {
            names.insert(short.to_owned());
        }
    }
    // Deleting a whole block removes its nested constructs as one range.
    for child in all.iter().filter(|c| r.block.contains(&c.range.start)) {
        lines.insert(child.line);
    }
    if crate::notation::Notation::of(source) == crate::notation::Notation::Timing {
        if let Some(id) = id {
            let doc = laml::timing::parse(source).map_err(|e| e.message)?;
            if let Some(lifeline) = doc.lifelines.iter().find(|p| p.id == id) {
                lines.extend(lifeline.changes.iter().map(|c| c.line));
                lines.extend(lifeline.events.iter().map(|e| e.line));
            }
        }
        let selected: Vec<_> = all.iter().filter(|r| lines.contains(&r.line)).collect();
        return Ok(Deletion {
            affected: selected
                .iter()
                .map(|r| format!("Line {}: {}", r.line, r.heading))
                .collect(),
            edits: selected
                .iter()
                .map(|r| Edit {
                    range: r.block.clone(),
                    replacement: String::new(),
                })
                .collect(),
        });
    }
    loop {
        let old = (lines.len(), names.len());
        for entry in &all {
            if !lines.contains(&entry.line) {
                continue;
            }
            for child in all.iter().filter(|c| entry.block.contains(&c.range.start)) {
                lines.insert(child.line);
            }
        }
        if let Some(doc) = &sequence {
            super::sequence_dependencies::expand(source, doc, 0, "", &mut lines, &mut names);
        }
        // Exact lexical references only: labels, comments and partial names are opaque.
        for entry in &all {
            let scope = super::references::scope(source, entry, notation);
            let references = super::references::spans(&entry.heading, &entry.section, false)
                .iter()
                .any(|span| {
                    let raw = entry.heading[span.clone()].trim_matches(['+', '-', '!', '@']);
                    let scoped = format!("{scope}.{raw}");
                    let token = if !scope.is_empty()
                        && namespace.iter().any(|id| {
                            scoped == *id
                                || scoped.starts_with(&format!("{id}."))
                                || scoped.starts_with(&format!("{id}::"))
                        }) {
                        scoped.as_str()
                    } else {
                        raw
                    };
                    names.iter().any(|name| {
                        token == name
                            || token.starts_with(&format!("{name}."))
                            || token.starts_with(&format!("{name}::"))
                    })
                });
            if references {
                lines.insert(entry.line);
            }
        }
        if old == (lines.len(), names.len()) {
            break;
        }
    }
    let selected: Vec<_> = all.iter().filter(|r| lines.contains(&r.line)).collect();
    let roots: Vec<_> = selected
        .iter()
        .filter(|r| {
            !selected
                .iter()
                .any(|p| p.line != r.line && p.block.contains(&r.range.start))
        })
        .collect();
    Ok(Deletion {
        affected: selected
            .iter()
            .map(|r| format!("Line {}: {}", r.line, r.heading))
            .collect(),
        edits: roots
            .iter()
            .map(|r| Edit {
                range: r.block.clone(),
                replacement: String::new(),
            })
            .collect(),
    })
}

pub fn relative(source: &str, from: &str, to: &str, op: &str) -> Result<Vec<Edit>, String> {
    if from == to {
        return Err("Choose a different reference".into());
    }
    let horizontal = matches!(op, "<" | ">");
    let mut rules = Vec::new();
    for r in records(source)
        .into_iter()
        .filter(|r| r.section == "layout")
    {
        let ts = fields::tokens(&r.heading);
        if ts.len() < 3 {
            continue;
        }
        let (a, operator, b) = (
            &r.heading[ts[0].clone()],
            &r.heading[ts[1].clone()],
            &r.heading[ts[2].clone()],
        );
        if !["<", ">", "above", "below"].contains(&operator)
            || matches!(operator, "<" | ">") != horizontal
        {
            continue;
        }
        let (a, b) = if matches!(operator, ">" | "below") {
            (b, a)
        } else {
            (a, b)
        };
        rules.push((a.to_owned(), b.to_owned(), r.line));
    }
    let (before, after) = if matches!(op, ">" | "below") {
        (to, from)
    } else {
        (from, to)
    };
    let reachable = |a: &str, b: &str| {
        let mut stack = vec![a.to_owned()];
        let mut visited = BTreeSet::new();
        while let Some(n) = stack.pop() {
            if n == b {
                return true;
            }
            if visited.insert(n.clone()) {
                stack.extend(
                    rules
                        .iter()
                        .filter(|(a, _, _)| a == &n)
                        .map(|(_, b, _)| b.clone()),
                );
            }
        }
        false
    };
    let mut edits = Vec::new();
    for (a, b, line) in &rules {
        if (before == from && b == from && reachable(after, a))
            || (after == from && a == from && reachable(b, before))
        {
            let r = record(source, *line)?;
            edits.push(Edit {
                range: r.block,
                replacement: String::new(),
            });
        }
    }
    if !rules.iter().any(|(a, b, _)| a == before && b == after) {
        edits.extend(insert(
            source,
            "layout",
            None,
            None,
            &format!("{from} {op} {to}"),
        )?);
    }
    Ok(edits)
}

fn unpack(source: &str, group: &Record, all: &[Record]) -> Result<String, String> {
    let index = SourceIndex::new(source);
    let newline = crate::element_edit::newline(source);
    let wrappers: Vec<_> = all
        .iter()
        .filter(|r| {
            r.line == group.line
                || (r.parent == group.parent
                    && group.block.contains(&r.range.start)
                    && (r.heading.starts_with("else ") || r.heading.starts_with("and ")))
        })
        .collect();
    let mut out = String::new();
    let mut previous = group.block.start;
    for wrapper in wrappers {
        out.push_str(&source[previous..wrapper.block.start]);
        let content = index
            .line(wrapper.line + 1)
            .ok_or("Missing container body")?
            .start;
        let close = crate::element_edit::closing_start(source, wrapper.block.end);
        let opening = &source[wrapper.range.clone()];
        let indent = opening.len() - opening.trim_start().len();
        let body = &source[content..close];
        let child = body
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(indent);
        let delta = child.saturating_sub(indent);
        let comment = &opening[code(opening).len()..];
        if comment.starts_with("//") {
            out.push_str(&format!(
                "{}{}{newline}",
                " ".repeat(indent),
                comment.trim_end_matches(['\r', '\n'])
            ));
        }
        for line in body.split_inclusive('\n') {
            let whitespace = line.len() - line.trim_start().len();
            out.push_str(&line[delta.min(whitespace)..]);
        }
        let closing = &source[close..wrapper.block.end];
        let comment = &closing[code(closing).len()..];
        if comment.starts_with("//") {
            out.push_str(&format!(
                "{}{}{newline}",
                " ".repeat(indent),
                comment.trim_end_matches(['\r', '\n'])
            ));
        }
        previous = wrapper.block.end;
    }
    out.push_str(&source[previous..group.block.end]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edits_preserve_comments_and_crlf() {
        let source = "settings {\r\n type: sequence\r\n}\r\nsemantics {\r\n A\r\n B\r\n A ->> B : hi // keep\r\n}\r\n";
        let f = fields::of(source, 7)
            .into_iter()
            .find(|f| f.name == "Label")
            .unwrap();
        let changed = apply(source, &fields::edit(source, &f, "hello").unwrap()).unwrap();
        assert!(changed.contains("hello // keep\r\n"));
        assert!(laml::sequence::parse(&changed).is_ok());
    }
    #[test]
    fn nested_move_and_reorder_are_single_source_transactions() {
        let source = "semantics {\n A\n B\n}\n";
        let changed = apply(source, &reorder(source, 2, true).unwrap()).unwrap();
        assert_eq!(changed, "semantics {\n B\n A\n}\n");
    }
}
