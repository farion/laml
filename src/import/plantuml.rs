use super::{Graph, Imported, line_error, words};

pub(super) fn convert(source: &str) -> Result<Vec<Imported>, String> {
    let mut lines = Vec::new();
    let mut started = false;
    let mut ended = false;
    for (n, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('\'') {
            continue;
        }
        if line.starts_with("@startuml") {
            if started {
                return Err(line_error(n + 1, "multiple diagrams in one input"));
            }
            started = true;
            continue;
        }
        if line == "@enduml" {
            ended = true;
            continue;
        }
        if !started || ended {
            return Err(line_error(n + 1, "content outside @startuml/@enduml"));
        }
        lines.push((n + 1, line));
    }
    if !started || !ended {
        return Err("PlantUML requires @startuml and @enduml".into());
    }
    if lines.is_empty() {
        return Err("empty PlantUML diagram".into());
    }
    let is_class = lines.iter().any(|(_, s)| {
        [
            "class ",
            "abstract class ",
            "interface ",
            "enum ",
            "package ",
        ]
        .iter()
        .any(|p| s.starts_with(p))
    });
    let out = if is_class {
        classes(&lines)?
    } else {
        sequence(&lines)?
    };
    Ok(vec![Imported {
        name: "diagram".into(),
        source: out,
    }])
}

fn declaration(line: &str) -> Result<Option<(String, String, String, bool)>, String> {
    let (kind, rest) = if let Some(rest) = line.strip_prefix("abstract class ") {
        ("abstract class", rest)
    } else if let Some(rest) = line.strip_prefix("class ") {
        ("class", rest)
    } else if let Some(rest) = line.strip_prefix("interface ") {
        ("interface", rest)
    } else if let Some(rest) = line.strip_prefix("enum ") {
        ("enum", rest)
    } else {
        return Ok(None);
    };
    let tokens = words(rest)?;
    let block = tokens.last().is_some_and(|s| s == "{");
    let args = &tokens[..tokens.len() - usize::from(block)];
    let (id, label) = match args {
        [id] => (id.clone(), id.clone()),
        [label, as_word, id] if as_word == "as" => (id.clone(), label.clone()),
        _ => return Err(format!("unsupported {kind} declaration: {line}")),
    };
    Ok(Some((kind.into(), id, label, block)))
}

fn classes(lines: &[(usize, &str)]) -> Result<String, String> {
    let mut graph = Graph {
        kind: "class".into(),
        ..Default::default()
    };
    let mut inside: Option<String> = None;
    let mut group: Option<String> = None;
    for &(n, line) in lines {
        if let Some(owner) = inside.as_deref() {
            if line == "}" {
                inside = None;
                continue;
            }
            if line.contains(['{', '}']) {
                return Err(line_error(n, "nested classifier body"));
            }
            graph.nodes[graph.index[owner]]
                .members
                .push(line.to_owned());
            continue;
        }
        if line == "}" {
            if group.take().is_some() {
                continue;
            }
            return Err(line_error(n, "unmatched }"));
        }
        if let Some(rest) = line.strip_prefix("package ") {
            if group.is_some() {
                return Err(line_error(n, "nested packages need a nested mapping"));
            }
            let tokens = words(rest).map_err(|e| line_error(n, e))?;
            if tokens.last().map(String::as_str) != Some("{") {
                return Err(line_error(n, "package requires a block"));
            }
            let (id, label) = match &tokens[..tokens.len() - 1] {
                [id] => (id.clone(), id.clone()),
                [label, as_word, id] if as_word == "as" => (id.clone(), label.clone()),
                _ => return Err(line_error(n, "package needs an explicit ASCII alias")),
            };
            super::identifier(&id).map_err(|e| line_error(n, e))?;
            if graph.index.contains_key(&id) || graph.groups.iter().any(|(old, _)| old == &id) {
                return Err(line_error(n, "duplicate package"));
            }
            graph.groups.push((id.clone(), label));
            group = Some(id);
            continue;
        }
        if line == "left to right direction" {
            graph.direction = Some("left-to-right".into());
            continue;
        }
        if let Some((kind, id, label, block)) = declaration(line).map_err(|e| line_error(n, e))? {
            graph
                .node(&id, Some(&label), &kind)
                .map_err(|e| line_error(n, e))?;
            if let Some(name) = &group {
                graph.nodes[graph.index[&id]].group = Some(name.clone());
            }
            if block {
                inside = Some(id);
            }
            continue;
        }
        if let Some((a, body)) = line.split_once(" : ")
            && !line.contains("--")
            && !line.contains("..")
        {
            let id = a.trim();
            graph
                .node(id, None, "class")
                .map_err(|e| line_error(n, e))?;
            graph.nodes[graph.index[id]]
                .members
                .push(body.trim().to_owned());
            continue;
        }
        if let Some((from, op, to)) = arrow(
            line,
            &[
                "<|--", "<|..", "--|>", "..|>", "*--", "o--", "-->", "..>", "--",
            ],
        ) {
            let (to, label) = to.split_once(" : ").map_or((to, ""), |(b, l)| (b, l));
            let (a, b, mapped) = match op {
                "<|--" => (to.trim(), from.trim(), "--|>"),
                "<|.." => (to.trim(), from.trim(), "..|>"),
                _ => (from.trim(), to.trim(), op),
            };
            graph.node(a, None, "class").map_err(|e| line_error(n, e))?;
            graph.node(b, None, "class").map_err(|e| line_error(n, e))?;
            graph
                .edge(a, b, mapped, label.trim())
                .map_err(|e| line_error(n, e))?;
            continue;
        }
        return Err(line_error(
            n,
            format!("unmapped PlantUML class construct: {line}"),
        ));
    }
    if inside.is_some() {
        return Err("unclosed PlantUML classifier".into());
    }
    if group.is_some() {
        return Err("unclosed PlantUML package".into());
    }
    graph.render()
}

fn arrow<'a>(line: &'a str, operators: &[&'a str]) -> Option<(&'a str, &'a str, &'a str)> {
    operators
        .iter()
        .filter_map(|op| line.find(op).map(|i| (i, *op)))
        .min_by_key(|(i, _)| *i)
        .map(|(i, op)| (&line[..i], op, &line[i + op.len()..]))
}
fn sequence(lines: &[(usize, &str)]) -> Result<String, String> {
    let mut translated = vec![(0, "sequenceDiagram".to_owned())];
    for &(n, line) in lines {
        if line == "hide footbox" {
            continue;
        }
        if line.starts_with("actor ") {
            return Err(line_error(
                n,
                "actor icon needs a sequence participant mapping",
            ));
        }
        if let Some(rest) = line.strip_prefix("participant ") {
            let args = words(rest).map_err(|e| line_error(n, e))?;
            let translated_line = match args.as_slice() {
                [id] => format!("participant {id}"),
                [label, as_word, id] if as_word == "as" => format!("participant {id} as {label}"),
                _ => return Err(line_error(n, "unsupported participant declaration")),
            };
            translated.push((n, translated_line));
            continue;
        }
        if line.starts_with("activate ") || line.starts_with("deactivate ") {
            translated.push((n, line.to_owned()));
            continue;
        }
        if let Some((a, op, b)) = arrow(line, &["-->", "->"]) {
            let (to, label) = b
                .split_once(':')
                .ok_or_else(|| line_error(n, "message requires label"))?;
            let mapped = if op == "-->" { "-->>" } else { "->>" };
            translated.push((
                n,
                format!("{}{}{}: {}", a.trim(), mapped, to.trim(), label.trim()),
            ));
            continue;
        }
        return Err(line_error(
            n,
            format!("unmapped PlantUML sequence construct: {line}"),
        ));
    }
    let borrowed: Vec<_> = translated.iter().map(|(n, s)| (*n, s.as_str())).collect();
    super::mermaid::sequence(&borrowed)
}
