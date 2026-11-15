//! Source-preserving BPMN membership and process wrapping.
use super::*;

pub fn relocate(source: &str, id: &str, destination: usize) -> Result<Vec<Edit>, String> {
    let doc = laml::bpmn::parse(source)?;
    let node = doc
        .nodes
        .iter()
        .find(|n| n.id == id)
        .ok_or("Missing BPMN element")?;
    if node.kind == Kind::Boundary {
        return Err("Move the task that owns this boundary event".into());
    }
    let target = record(source, destination)?;
    if !target.opens {
        return Err("Choose a pool, lane or expanded subprocess".into());
    }
    let all = records(source);
    let mut ancestors = vec![destination];
    let mut parent = target.parent;
    while let Some(line) = parent {
        ancestors.push(line);
        parent = all.iter().find(|r| r.line == line).and_then(|r| r.parent);
    }
    let pool = doc.pools.iter().find(|p| ancestors.contains(&p.line));
    let subprocess = ancestors.iter().find_map(|line| {
        doc.nodes
            .iter()
            .find(|n| n.kind == Kind::Subprocess && n.line == *line)
    });
    let short = node.id.rsplit('.').next().unwrap();
    let scope = subprocess
        .map(|n| n.id.as_str())
        .or_else(|| pool.map(|p| p.id.as_str()));
    let new_id = scope.map_or_else(|| short.to_owned(), |scope| format!("{scope}.{short}"));
    if new_id != id && doc.nodes.iter().any(|n| n.id == new_id) {
        return Err("The destination already contains this ID".into());
    }
    if node.kind == Kind::Subprocess {
        return relocate_subprocess(source, &doc, node, destination, &new_id);
    }
    let attached: BTreeSet<_> = doc
        .nodes
        .iter()
        .filter(|n| n.attached.as_deref() == Some(id))
        .map(|n| n.id.clone())
        .collect();
    let mut moving = attached.clone();
    moving.insert(id.into());
    let map_id = |old: &str| {
        if moving.contains(old) {
            new_id.rsplit_once('.').map_or_else(
                || old.rsplit('.').next().unwrap().to_owned(),
                |(prefix, _)| format!("{prefix}.{}", old.rsplit('.').next().unwrap()),
            )
        } else {
            old.to_owned()
        }
    };
    let incident: BTreeSet<_> = doc
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| moving.contains(&e.from) || moving.contains(&e.to))
        .map(|(i, _)| i)
        .collect();
    let lines: BTreeSet<_> = doc
        .nodes
        .iter()
        .filter(|n| moving.contains(&n.id))
        .map(|n| n.line)
        .chain(incident.iter().map(|i| doc.edges[*i].line))
        .collect();
    let mut normalized = doc.clone();
    for n in normalized
        .nodes
        .iter_mut()
        .filter(|n| moving.contains(&n.id))
    {
        n.line = 0;
    }
    let mut edits = Vec::new();
    for line in lines {
        edits.push(rewrite_line(
            source,
            &normalized,
            line,
            &BTreeSet::new(),
            &incident,
            None,
        )?);
    }
    let after = apply(source, &edits)?;
    let destination = crate::relation_edit::remap_line(source, destination, &edits);
    let mut declarations = vec![declaration(node, None)];
    let mut boundary_edges = BTreeSet::new();
    for boundary in doc.nodes.iter().filter(|n| attached.contains(&n.id)) {
        let (i, edge) = doc
            .edges
            .iter()
            .enumerate()
            .find(|(_, e)| e.from == boundary.id && e.line == boundary.line)
            .ok_or("Missing attached-event flow")?;
        boundary_edges.insert(i);
        declarations.push(format!(
            "{new_id} -({}{}@{}:{})-> {}",
            boundary.marker.as_deref().unwrap_or("timer"),
            if boundary.interrupting { "" } else { "+" },
            boundary.id.rsplit('.').next().unwrap(),
            crate::relation_edit::quote(&boundary.label),
            map_id(&edge.to)
        ));
    }
    let added = insert(
        &after,
        "semantics",
        Some(destination),
        None,
        &declarations.join("\n"),
    )?;
    let mut after = apply(&after, &added)?;
    let flows = incident
        .iter()
        .filter(|i| !boundary_edges.contains(i))
        .map(|i| {
            let edge = &doc.edges[*i];
            let from = map_id(&edge.from);
            let to = map_id(&edge.to);
            let op = match edge.kind {
                EdgeKind::Sequence => "-->",
                EdgeKind::Message => "~~>",
                EdgeKind::Data => "..>",
                EdgeKind::Association => "..",
            };
            format!(
                "{from} {op} {to}{}",
                edge.label
                    .as_ref()
                    .map_or(String::new(), |l| format!(" : {l}"))
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if !flows.is_empty() {
        let edits = insert(&after, "semantics", None, None, &flows)?;
        after = apply(&after, &edits)?;
    }
    // Qualified selectors and placement references retain the moved identity.
    if new_id != id {
        let mut changes = Vec::new();
        let index = SourceIndex::new(&after);
        for r in records(&after)
            .into_iter()
            .filter(|r| r.section != "semantics")
        {
            let range = index.line(r.line).unwrap();
            for span in super::references::spans(code(&after[range.clone()]), &r.section, false) {
                let token = &after[range.start + span.start..range.start + span.end];
                if let Some(old) = moving.iter().find(|old| {
                    token == old.as_str()
                        || (old.rsplit('.').next() == Some(token)
                            && doc
                                .nodes
                                .iter()
                                .filter(|n| n.id.rsplit('.').next() == Some(token))
                                .count()
                                == 1)
                }) {
                    changes.push(Edit {
                        range: range.start + span.start..range.start + span.end,
                        replacement: map_id(old),
                    });
                }
            }
        }
        after = apply(&after, &changes)?;
    }
    laml::bpmn::parse(&after)?;
    Ok(crate::graph_edit::difference(source, &after))
}

pub fn wrap_root_process(source: &str, id: &str, label: &str) -> Result<Vec<Edit>, String> {
    if !crate::graph_edit::identifier(id) {
        return Err("Use a simple pool ID".into());
    }
    let doc = laml::bpmn::parse(source)?;
    if !doc.pools.is_empty() {
        return Err("The process already uses pools; add another pool instead".into());
    }
    let index = SourceIndex::new(source);
    let implicit: BTreeSet<_> = doc
        .nodes
        .iter()
        .filter(|n| {
            let short = n.id.rsplit('.').next().unwrap();
            (short.starts_with("gateway_") || short.starts_with("boundary_"))
                && index
                    .line(n.line)
                    .is_some_and(|range| !source[range].contains(&format!("@{short}")))
        })
        .map(|n| n.line)
        .collect();
    if !implicit.is_empty() {
        let edits = implicit
            .into_iter()
            .map(|line| rewrite_line(source, &doc, line, &BTreeSet::new(), &BTreeSet::new(), None))
            .collect::<Result<Vec<_>, _>>()?;
        let normalized = apply(source, &edits)?;
        let wrap = wrap_root_process(&normalized, id, label)?;
        return Ok(crate::graph_edit::difference(
            source,
            &apply(&normalized, &wrap)?,
        ));
    }
    let mut depth = None;
    let mut start = 0;
    let mut end = None;
    for (i, raw) in source.lines().enumerate() {
        let text = code(raw).trim();
        if depth.is_none() {
            if text == "semantics {" {
                depth = Some(0usize);
                start = index.line(i + 2).map_or(source.len(), |r| r.start);
            }
        } else if text == "}" {
            if depth == Some(0) {
                end = Some(index.line(i + 1).unwrap().start);
                break;
            }
            depth = depth.map(|d| d - 1);
        } else if text.ends_with('{') {
            depth = depth.map(|d| d + 1);
        }
    }
    let end = end.ok_or("Missing semantics boundary")?;
    let newline = crate::element_edit::newline(source);
    let body = source[start..end]
        .split_inclusive('\n')
        .map(|line| format!("    {line}"))
        .collect::<String>();
    let replacement = format!(
        "    pool {id}(label={}) {{{newline}{body}    }}{newline}",
        crate::relation_edit::quote(label)
    );
    let candidate = apply(
        source,
        &[Edit {
            range: start..end,
            replacement,
        }],
    )?;
    let mut refs = Vec::new();
    let index = SourceIndex::new(&candidate);
    for r in records(&candidate) {
        if r.section == "settings" || (r.section == "styling" && !r.opens) {
            continue;
        }
        let range = index.line(r.line).unwrap();
        let text = code(&candidate[range.clone()]);
        for span in super::references::spans(text, &r.section, false) {
            let token = &text[span.clone()];
            if let Some(node) = doc
                .nodes
                .iter()
                .find(|n| n.id == token && (r.section != "semantics" || token.contains('.')))
            {
                refs.push(Edit {
                    range: range.start + span.start..range.start + span.end,
                    replacement: format!("{id}.{}", node.id),
                });
            }
        }
    }
    let candidate = apply(&candidate, &refs)?;
    laml::bpmn::parse(&candidate)?;
    Ok(crate::graph_edit::difference(source, &candidate))
}

fn relocate_subprocess(
    source: &str,
    doc: &laml::bpmn::Document,
    node: &laml::bpmn::Node,
    destination: usize,
    new_id: &str,
) -> Result<Vec<Edit>, String> {
    let group = record(source, node.line)?;
    let boundary: BTreeSet<_> = doc
        .nodes
        .iter()
        .filter(|n| n.attached.as_deref() == Some(&node.id))
        .map(|n| n.id.clone())
        .collect();
    let moves = |id: &str| {
        id == node.id || id.starts_with(&format!("{}.", node.id)) || boundary.contains(id)
    };
    let mapped = |id: &str| {
        if boundary.contains(id) {
            new_id.rsplit_once('.').map_or_else(
                || id.rsplit('.').next().unwrap().to_owned(),
                |(prefix, _)| format!("{prefix}.{}", id.rsplit('.').next().unwrap()),
            )
        } else if id == node.id || id.starts_with(&format!("{}.", node.id)) {
            format!("{new_id}{}", &id[node.id.len()..])
        } else {
            id.into()
        }
    };
    let mut edits = move_record(source, node.line, Some(destination), None)?;
    let index = SourceIndex::new(source);
    let inside = |line| {
        index
            .line(line)
            .is_some_and(|r| group.block.contains(&r.start))
    };
    let external: BTreeSet<_> = doc
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| !inside(e.line) && (moves(&e.from) || moves(&e.to)))
        .map(|(i, _)| i)
        .collect();
    let mut normalized = doc.clone();
    for n in normalized
        .nodes
        .iter_mut()
        .filter(|n| boundary.contains(&n.id))
    {
        n.line = 0;
    }
    let lines: BTreeSet<_> = external
        .iter()
        .map(|i| doc.edges[*i].line)
        .chain(
            doc.nodes
                .iter()
                .filter(|n| boundary.contains(&n.id))
                .map(|n| n.line),
        )
        .collect();
    for line in lines {
        edits.push(rewrite_line(
            source,
            &normalized,
            line,
            &BTreeSet::new(),
            &external,
            None,
        )?);
    }
    let mut boundary_flows = BTreeSet::new();
    let mut declarations = Vec::new();
    for n in doc.nodes.iter().filter(|n| boundary.contains(&n.id)) {
        let (i, edge) = doc
            .edges
            .iter()
            .enumerate()
            .find(|(_, e)| e.from == n.id && e.line == n.line)
            .ok_or("Missing boundary-event flow")?;
        boundary_flows.insert(i);
        declarations.push(format!(
            "{new_id} -({}{}@{}:{})-> {}",
            n.marker.as_deref().unwrap_or("timer"),
            if n.interrupting { "" } else { "+" },
            n.id.rsplit('.').next().unwrap(),
            crate::relation_edit::quote(&n.label),
            mapped(&edge.to)
        ));
    }
    if !declarations.is_empty() {
        edits.extend(insert(
            source,
            "semantics",
            Some(destination),
            None,
            &declarations.join("\n"),
        )?);
    }
    let edits = crate::element_edit::merge_insertions(edits);
    let mut candidate = apply(source, &edits)?;
    let flows = external
        .iter()
        .filter(|i| !boundary_flows.contains(i))
        .map(|i| {
            let e = &doc.edges[*i];
            let op = match e.kind {
                EdgeKind::Sequence => "-->",
                EdgeKind::Message => "~~>",
                EdgeKind::Data => "..>",
                EdgeKind::Association => "..",
            };
            format!(
                "{} {op} {}{}",
                mapped(&e.from),
                mapped(&e.to),
                e.label
                    .as_ref()
                    .map_or(String::new(), |l| format!(" : {l}"))
            )
        })
        .collect::<Vec<_>>();
    if !flows.is_empty() {
        let edits = insert(&candidate, "semantics", None, None, &flows.join("\n"))?;
        candidate = apply(&candidate, &edits)?;
    }
    let mut refs = Vec::new();
    for r in records(&candidate) {
        let raw = code(&candidate[r.range.clone()]);
        for span in super::references::spans(raw, &r.section, false) {
            let value = &raw[span.clone()];
            if moves(value) && mapped(value) != value {
                refs.push(Edit {
                    range: r.range.start + span.start..r.range.start + span.end,
                    replacement: mapped(value),
                });
            }
        }
    }
    candidate = apply(&candidate, &refs)?;
    laml::bpmn::parse(&candidate)?;
    Ok(crate::graph_edit::difference(source, &candidate))
}
