use super::{Graph, Imported, line_error, quoted};

pub(super) fn convert(source: &str) -> Result<Vec<Imported>, String> {
    let lines: Vec<_> = source
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let line = line.trim();
            (!line.is_empty() && !line.starts_with("%%")).then_some((i + 1, line))
        })
        .collect();
    let Some((_, header)) = lines.first() else {
        return Err("empty Mermaid source".into());
    };
    let result = if header.starts_with("flowchart ") || header.starts_with("graph ") {
        flow(&lines)?
    } else if *header == "classDiagram" {
        classes(&lines)?
    } else if *header == "sequenceDiagram" {
        sequence(&lines)?
    } else {
        return Err(line_error(
            lines[0].0,
            format!("Mermaid diagram type '{header}' requires a dedicated mapping"),
        ));
    };
    Ok(vec![Imported {
        name: "diagram".into(),
        source: result,
    }])
}

fn flow(lines: &[(usize, &str)]) -> Result<String, String> {
    let mut graph = Graph::default();
    let mut group: Option<String> = None;
    graph.direction = Some(
        direction(
            lines[0].1.split_whitespace().nth(1).unwrap_or(""),
            lines[0].0,
        )?
        .into(),
    );
    for &(line, text) in &lines[1..] {
        if text.starts_with("direction ") {
            graph.direction =
                Some(direction(text.split_whitespace().nth(1).unwrap_or(""), line)?.into());
            continue;
        }
        if let Some(rest) = text.strip_prefix("subgraph ") {
            if group.is_some() {
                return Err(line_error(
                    line,
                    "nested subgraphs need a nested package mapping",
                ));
            }
            let (id, label, _) = flow_node(rest).map_err(|e| line_error(line, e))?;
            if graph.index.contains_key(&id) || graph.groups.iter().any(|(other, _)| other == &id) {
                return Err(line_error(line, "duplicate subgraph identifier"));
            }
            graph
                .groups
                .push((id.clone(), label.unwrap_or_else(|| id.clone())));
            group = Some(id);
            continue;
        }
        if text == "end" {
            if group.take().is_none() {
                return Err(line_error(line, "end without subgraph"));
            }
            continue;
        }
        if text.contains("-.->") || text.contains("==>") {
            return Err(line_error(
                line,
                "dashed and thick flow links need a relation-style mapping",
            ));
        }
        if text.contains(';') || text.contains('&') || text.contains("@{") {
            return Err(line_error(
                line,
                "combined edges, chained nodes and expanded shapes need a dedicated mapping",
            ));
        }
        if let Some((left, operator, right)) = find_arrow(text, &["-->", "---"]) {
            let (from, from_label, from_shape) =
                flow_node(left).map_err(|e| line_error(line, e))?;
            let right = right.trim();
            let (edge_label, right) = if let Some(tail) = right.strip_prefix('|') {
                let (label, remaining) = tail
                    .split_once('|')
                    .ok_or_else(|| line_error(line, "unclosed edge label"))?;
                (label.trim(), remaining.trim())
            } else {
                ("", right)
            };
            let (to, to_label, to_shape) = flow_node(right).map_err(|e| line_error(line, e))?;
            add_flow_node(
                &mut graph,
                &from,
                from_label.as_deref(),
                from_shape.as_deref(),
                group.as_deref(),
            )
            .map_err(|e| line_error(line, e))?;
            add_flow_node(
                &mut graph,
                &to,
                to_label.as_deref(),
                to_shape.as_deref(),
                group.as_deref(),
            )
            .map_err(|e| line_error(line, e))?;
            let op = if operator == "---" { "--" } else { "->" };
            graph
                .edge(&from, &to, op, edge_label)
                .map_err(|e| line_error(line, e))?;
            continue;
        }
        let (id, label, shape) = flow_node(text).map_err(|e| line_error(line, e))?;
        add_flow_node(
            &mut graph,
            &id,
            label.as_deref(),
            shape.as_deref(),
            group.as_deref(),
        )
        .map_err(|e| line_error(line, e))?;
    }
    if group.is_some() {
        return Err("unclosed Mermaid subgraph".into());
    }
    graph.render()
}

fn direction(token: &str, line: usize) -> Result<&'static str, String> {
    match token {
        "LR" => Ok("left-to-right"),
        "RL" => Ok("right-to-left"),
        "TB" | "TD" => Ok("top-to-bottom"),
        "BT" => Ok("bottom-to-top"),
        _ => Err(line_error(
            line,
            format!("unknown flow direction '{token}'"),
        )),
    }
}
fn find_arrow<'a>(text: &'a str, operators: &[&'a str]) -> Option<(&'a str, &'a str, &'a str)> {
    operators
        .iter()
        .filter_map(|op| text.find(op).map(|i| (i, *op)))
        .min_by_key(|(i, _)| *i)
        .map(|(i, op)| (&text[..i], op, &text[i + op.len()..]))
}
fn flow_node(text: &str) -> Result<(String, Option<String>, Option<String>), String> {
    let text = text.trim();
    let opening = text.find(['[', '(', '{']);
    let (id, label, shape) = match opening {
        None => (text, None, None),
        Some(index) => {
            let (id, rest) = text.split_at(index);
            let (left, right, shape) = if rest.starts_with("((") {
                ("((", "))", "ellipse")
            } else if rest.starts_with('(') {
                ("(", ")", "ellipse")
            } else if rest.starts_with('{') {
                ("{", "}", "diamond")
            } else {
                ("[", "]", "rectangle")
            };
            let value = rest
                .strip_prefix(left)
                .and_then(|s| s.strip_suffix(right))
                .ok_or("unclosed or unsupported node shape")?;
            if value.starts_with(['[', '(', '{']) || value.ends_with([']', ')', '}']) {
                return Err("nested Mermaid node shape requires a visual mapping".into());
            }
            (
                id.trim(),
                Some(value.trim_matches('"').to_owned()),
                Some(shape.to_owned()),
            )
        }
    };
    super::identifier(id)?;
    if label
        .as_ref()
        .is_some_and(|s| s.contains(['<', '>']) || s.contains('`'))
    {
        return Err("HTML/Markdown node labels require an explicit text conversion".into());
    }
    Ok((id.to_owned(), label, shape))
}
fn add_flow_node(
    graph: &mut Graph,
    id: &str,
    label: Option<&str>,
    shape: Option<&str>,
    group: Option<&str>,
) -> Result<(), String> {
    graph.node(id, label, "")?;
    let node = &mut graph.nodes[graph.index[id]];
    if let Some(group) = group {
        if node.group.as_deref().is_some_and(|old| old != group) {
            return Err(format!("node '{id}' belongs to two subgraphs"));
        }
        node.group = Some(group.to_owned());
    }
    if let Some(shape) = shape {
        node.shape = (shape != "rectangle").then(|| shape.to_owned());
    }
    Ok(())
}

fn classes(lines: &[(usize, &str)]) -> Result<String, String> {
    let mut graph = Graph {
        kind: "class".into(),
        ..Default::default()
    };
    let mut inside: Option<String> = None;
    for &(line, text) in &lines[1..] {
        if let Some(owner) = inside.as_deref() {
            if text == "}" {
                inside = None;
                continue;
            }
            if text.contains(['{', '}']) {
                return Err(line_error(line, "nested Mermaid class body"));
            }
            graph.nodes[graph.index[owner]]
                .members
                .push(text.to_owned());
            continue;
        }
        if let Some(id) = text
            .strip_prefix("class ")
            .and_then(|t| t.strip_suffix('{'))
        {
            let id = id.trim();
            graph
                .node(id, None, "class")
                .map_err(|e| line_error(line, e))?;
            inside = Some(id.to_owned());
            continue;
        }
        if let Some(id) = text.strip_prefix("class ") {
            graph
                .node(id.trim(), None, "class")
                .map_err(|e| line_error(line, e))?;
            continue;
        }
        if text.starts_with("direction ") {
            graph.direction =
                Some(direction(text.split_whitespace().nth(1).unwrap_or(""), line)?.into());
            continue;
        }
        if let Some((id, member)) = text.split_once(" : ") {
            graph
                .node(id.trim(), None, "class")
                .map_err(|e| line_error(line, e))?;
            graph.nodes[graph.index[id.trim()]]
                .members
                .push(member.trim().to_owned());
            continue;
        }
        if let Some((a, op, b)) = find_arrow(
            text,
            &[
                "<|--", "<|..", "..|>", "--|>", "*--", "o--", "-->", "..>", "--",
            ],
        ) {
            let (target, label) = b.split_once(" : ").map_or((b, ""), |(b, l)| (b, l));
            let (from, to, mapped) = match op {
                "<|--" => (target.trim(), a.trim(), "--|>"),
                "<|.." => (target.trim(), a.trim(), "..|>"),
                _ => (a.trim(), target.trim(), op),
            };
            graph
                .node(from, None, "class")
                .map_err(|e| line_error(line, e))?;
            graph
                .node(to, None, "class")
                .map_err(|e| line_error(line, e))?;
            graph
                .edge(from, to, mapped, label.trim())
                .map_err(|e| line_error(line, e))?;
            continue;
        }
        return Err(line_error(
            line,
            format!("unmapped Mermaid class construct: {text}"),
        ));
    }
    if inside.is_some() {
        return Err("unclosed Mermaid class body".into());
    }
    graph.render()
}

pub(super) fn sequence(lines: &[(usize, &str)]) -> Result<String, String> {
    let mut participants = Graph::default();
    let mut messages = Vec::new();
    for &(line, text) in &lines[1..] {
        if text.starts_with("actor ") {
            return Err(line_error(
                line,
                "actor icon needs a sequence participant mapping",
            ));
        }
        if let Some(rest) = text.strip_prefix("participant ") {
            let (id, label) = rest
                .split_once(" as ")
                .map_or((rest, None), |(a, b)| (a, Some(b)));
            participants
                .node(id.trim(), label.map(str::trim), "")
                .map_err(|e| line_error(line, e))?;
            continue;
        }
        if let Some(rest) = text.strip_prefix("activate ") {
            let id = rest.trim();
            participants
                .node(id, None, "")
                .map_err(|e| line_error(line, e))?;
            messages.push(format!("    {id}+"));
            continue;
        }
        if let Some(rest) = text.strip_prefix("deactivate ") {
            let id = rest.trim();
            participants
                .node(id, None, "")
                .map_err(|e| line_error(line, e))?;
            messages.push(format!("    {id}-"));
            continue;
        }
        if let Some((a, op, b)) = find_arrow(text, &["-->>", "->>", "--)", "-)"]) {
            let (to, label) = b
                .split_once(':')
                .ok_or_else(|| line_error(line, "message requires a label"))?;
            let from = a.trim();
            let to = to.trim();
            participants
                .node(from, None, "")
                .map_err(|e| line_error(line, e))?;
            participants
                .node(to, None, "")
                .map_err(|e| line_error(line, e))?;
            // Dotted arrows depict replies; explicit endpoint syntax checks call matching.
            let mapped = match op {
                "-->>" => format!("{to} <-- {from}"),
                "->>" => format!("{from} -> {to}"),
                _ => format!("{from} ->> {to}"),
            };
            messages.push(format!("    {mapped} : {}", label.trim()));
            continue;
        }
        return Err(line_error(
            line,
            format!("unmapped Mermaid sequence construct: {text}"),
        ));
    }
    if participants.nodes.is_empty() {
        return Err("sequence has no participants".into());
    }
    let mut out = String::from("settings {\n    type: sequence\n}\nsemantics {\n");
    for node in &participants.nodes {
        let label = if node.id == node.label {
            String::new()
        } else {
            format!("(label={})", quoted(&node.label))
        };
        out.push_str(&format!("    {}{label}\n", node.id));
    }
    for message in messages {
        out.push_str(&message);
        out.push('\n');
    }
    out.push_str("}\n");
    Ok(out)
}
