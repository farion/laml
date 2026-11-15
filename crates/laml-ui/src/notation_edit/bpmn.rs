//! BPMN inline declarations are expanded only on the edited source line.
//! This preserves unrelated nodes/flows instead of deleting an entire inline chain.
use super::*;
use laml::bpmn::{Document, EdgeKind, Kind, Node};
use std::collections::BTreeSet;

pub const TYPES: &[(&str, &str)] = &[
    ("task", "Task"), ("userTask", "User task"), ("serviceTask", "Service task"),
    ("sendTask", "Send task"), ("receiveTask", "Receive task"), ("scriptTask", "Script task"),
    ("manualTask", "Manual task"), ("startEvent", "Start event"), ("endEvent", "End event"),
    ("messageStart", "Message start"), ("timerStart", "Timer start"), ("messageEnd", "Message end"),
    ("errorEnd", "Error end"), ("terminateEnd", "Terminate end"),
    ("intermediateTimer", "Intermediate timer"), ("intermediateMessage", "Intermediate message"),
    ("<X>@", "Exclusive gateway"), ("<+>@", "Parallel gateway"), ("<O>@", "Inclusive gateway"),
    ("<event>@", "Event-based gateway"), ("+", "Collapsed subprocess"), ("data", "Data object"),
    ("store", "Data store"), ("note", "Annotation"),
];
pub fn keyword(n: &Node) -> &'static str {
    match (n.kind, n.marker.as_deref()) {
        (Kind::Task, _) => "task", (Kind::UserTask, _) => "userTask", (Kind::ServiceTask, _) => "serviceTask",
        (Kind::SendTask, _) => "sendTask", (Kind::ReceiveTask, _) => "receiveTask", (Kind::ScriptTask, _) => "scriptTask",
        (Kind::ManualTask, _) => "manualTask", (Kind::Start, Some("message")) => "messageStart",
        (Kind::Start, Some("timer")) => "timerStart", (Kind::Start, _) => "startEvent",
        (Kind::End, Some("message")) => "messageEnd", (Kind::End, Some("error")) => "errorEnd",
        (Kind::End, Some("terminate")) => "terminateEnd", (Kind::End, _) => "endEvent",
        (Kind::Intermediate, Some("timer")) => "intermediateTimer", (Kind::Intermediate, _) => "intermediateMessage",
        (Kind::Exclusive, _) => "<X>@", (Kind::Parallel, _) => "<+>@", (Kind::Inclusive, _) => "<O>@",
        (Kind::EventBased, _) => "<event>@", (Kind::Collapsed, _) => "+", (Kind::Data, _) => "data",
        (Kind::Store, _) => "store", (Kind::Annotation, _) => "note", _ => "",
    }
}
fn declaration(n: &Node, override_type: Option<&str>) -> String {
    let ty = override_type.unwrap_or_else(|| keyword(n));
    let prefix = if ty == "task" { String::new() } else if ty.ends_with('@') || ty == "+" { ty.into() } else { format!("{ty} ") };
    let mut attrs = format!("label={}", crate::relation_edit::quote(&n.label));
    if let Some(icon) = &n.icon { attrs.push_str(&format!("; icon={}", crate::relation_edit::quote(icon))); }
    format!("{prefix}{}({attrs})", n.id.rsplit('.').next().unwrap())
}
pub(super) fn rewrite_line(source: &str, doc: &Document, line: usize, deleted_nodes: &BTreeSet<String>, deleted_edges: &BTreeSet<usize>, changed_type: Option<(&str, &str)>) -> Result<Edit, String> {
    let r = record(source, line)?;
    if r.opens { return Err("Edit the container header or its contents separately".into()); }
    let raw = &source[r.range.clone()];
    let indent = &raw[..raw.len() - raw.trim_start().len()];
    let newline = crate::element_edit::newline(source);
    let mut bodies = Vec::new();
    let comment = &raw[code(raw).len()..];
    if !comment.is_empty() { bodies.push(comment.into()); }
    let mut attached = BTreeSet::new();
    for n in doc.nodes.iter().filter(|n| n.line == line && !deleted_nodes.contains(&n.id)) {
        if n.kind == Kind::Boundary { continue; }
        bodies.push(declaration(n, changed_type.filter(|(id, _)| *id == n.id).map(|(_, ty)| ty)));
    }
    for (index, edge) in doc.edges.iter().enumerate().filter(|(_, e)| e.line == line) {
        if deleted_edges.contains(&index) || deleted_nodes.contains(&edge.from) || deleted_nodes.contains(&edge.to) { continue; }
        let boundary = doc.nodes.iter().find(|n| n.id == edge.from && n.kind == Kind::Boundary && n.line == line);
        if let Some(n) = boundary.filter(|n| attached.insert(n.id.clone())) {
            let marker = n.marker.as_deref().unwrap_or("timer");
            bodies.push(format!("{} -({marker}{}@{}:{})-> {}", n.attached.as_deref().ok_or("Missing boundary owner")?,
                if n.interrupting { "" } else { "+" }, n.id.rsplit('.').next().unwrap(), crate::relation_edit::quote(&n.label), edge.to));
        } else {
            let operator = match edge.kind { EdgeKind::Sequence => "-->", EdgeKind::Message => "~~>", EdgeKind::Data => "..>", EdgeKind::Association => ".." };
            let label = edge.label.as_ref().map_or(String::new(), |label| format!(" : {label}"));
            bodies.push(format!("{} {operator} {}{label}", edge.from, edge.to));
        }
    }
    let replacement = bodies.into_iter().map(|body| format!("{indent}{body}{newline}")).collect();
    Ok(Edit { range: r.block, replacement })
}
pub fn change_type(source: &str, id: &str, ty: &str) -> Result<Vec<Edit>, String> {
    if !TYPES.iter().any(|(key, _)| *key == ty) { return Err("Choose a BPMN element type".into()); }
    let doc = laml::bpmn::parse(source)?;
    let node = doc.nodes.iter().find(|n| n.id == id).ok_or("Missing BPMN element")?;
    if matches!(node.kind, Kind::Boundary | Kind::Subprocess) { return Err("This structural element cannot change to a task or event".into()); }
    let edit = rewrite_line(source, &doc, node.line, &BTreeSet::new(), &BTreeSet::new(), Some((id, ty)))?;
    let candidate = apply(source, std::slice::from_ref(&edit))?;
    laml::bpmn::parse(&candidate)?;
    Ok(vec![edit])
}
pub fn delete_item(source: &str, id: &str) -> Result<Deletion, String> {
    let doc = laml::bpmn::parse(source)?;
    let mut nodes = BTreeSet::new(); let mut edges = BTreeSet::new();
    if let Some(index) = id.strip_prefix("flow:").and_then(|s| s.parse::<usize>().ok()) {
        if index >= doc.edges.len() { return Err("Missing BPMN flow".into()); }
        if let Some(boundary) = doc.nodes.iter().find(|n| n.kind == Kind::Boundary && n.id == doc.edges[index].from && n.line == doc.edges[index].line) {
            nodes.insert(boundary.id.clone());
            for (i, edge) in doc.edges.iter().enumerate() { if edge.from == boundary.id || edge.to == boundary.id { edges.insert(i); } }
        }
        edges.insert(index);
    } else {
        let node = doc.nodes.iter().find(|n| n.id == id).ok_or("Missing BPMN element")?;
        if node.kind == Kind::Subprocess { return delete(source, node.line, Some(id), false); }
        nodes.insert(id.into());
        for node in &doc.nodes { if node.attached.as_deref() == Some(id) { nodes.insert(node.id.clone()); } }
        for (i, edge) in doc.edges.iter().enumerate() { if nodes.contains(&edge.from) || nodes.contains(&edge.to) { edges.insert(i); } }
    }
    let lines: BTreeSet<_> = doc.nodes.iter().filter(|n| nodes.contains(&n.id)).map(|n| n.line)
        .chain(edges.iter().map(|i| doc.edges[*i].line)).collect();
    let mut edits = Vec::new();
    let mut affected: Vec<_> = nodes.iter().map(|id| format!("Element {id}")).collect();
    for i in &edges { affected.push(format!("Flow {} → {}", doc.edges[*i].from, doc.edges[*i].to)); }
    for line in lines { edits.push(rewrite_line(source, &doc, line, &nodes, &edges, None)?); }
    for r in records(source).into_iter().filter(|r| r.section == "layout") {
        let references = fields::tokens(&r.heading).iter().any(|span| nodes.contains(&r.heading[span.clone()]));
        if references { affected.push(format!("Layout {}", r.heading)); edits.push(Edit { range: r.block, replacement: String::new() }); }
    }
    Ok(Deletion { edits, affected })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deleting_one_inline_node_keeps_other_declarations_and_unrelated_flows() {
        let source = "settings {\n type: bpmn\n}\nsemantics {\n startEvent A\n A --> B(\"Task\") --> C --> D // retain\n}\n";
        let plan = delete_item(source, "B").unwrap();
        let candidate = apply(source, &plan.edits).unwrap();
        let doc = laml::bpmn::parse(&candidate).unwrap();
        assert!(doc.nodes.iter().all(|n| n.id != "B"));
        assert!(doc.nodes.iter().any(|n| n.id == "C"));
        assert_eq!(doc.edges.len(), 1); assert_eq!(doc.edges[0].to, "D");
        assert!(candidate.contains("// retain"));
    }
}

pub fn node_property(source: &str, id: &str, property: &str, value: &str) -> Result<Vec<Edit>, String> {
    let mut doc = laml::bpmn::parse(source)?;
    let node = doc.nodes.iter_mut().find(|n| n.id == id).ok_or("Missing BPMN element")?;
    match property {
        "label" => node.label = value.into(),
        "icon" => node.icon = (!value.is_empty()).then(|| value.into()),
        "marker" if node.kind == Kind::Boundary && ["timer", "message", "error"].contains(&value) => node.marker = Some(value.into()),
        "interrupting" if node.kind == Kind::Boundary && ["true", "false"].contains(&value) => node.interrupting = value == "true",
        _ => return Err("Property does not apply to this BPMN element".into()),
    }
    let line = node.line;
    if node.kind == Kind::Subprocess { return attribute(source, line, Some(id), property, value); }
    let edit = rewrite_line(source, &doc, line, &BTreeSet::new(), &BTreeSet::new(), None)?;
    let candidate = apply(source, std::slice::from_ref(&edit))?;
    laml::bpmn::parse(&candidate)?;
    Ok(vec![edit])
}
pub fn flow_property(source: &str, index: usize, property: &str, value: &str) -> Result<Vec<Edit>, String> {
    if value.contains(['\r', '\n', '{', '}']) { return Err("Enter one flow property".into()); }
    let mut doc = laml::bpmn::parse(source)?;
    let edge = doc.edges.get_mut(index).ok_or("Missing BPMN flow")?;
    match property {
        "from" => edge.from = value.into(), "to" => edge.to = value.into(),
        "label" => edge.label = (!value.is_empty()).then(|| value.into()),
        "type" => edge.kind = match value { "-->" => EdgeKind::Sequence, "~~>" => EdgeKind::Message, "..>" => EdgeKind::Data, ".." => EdgeKind::Association, _ => return Err("Unknown flow kind".into()) },
        _ => return Err("Unknown flow property".into()),
    }
    let line = edge.line;
    let edit = rewrite_line(source, &doc, line, &BTreeSet::new(), &BTreeSet::new(), None)?;
    let candidate = apply(source, std::slice::from_ref(&edit))?;
    laml::bpmn::parse(&candidate)?;
    Ok(vec![edit])
}

pub fn relocate(source: &str, id: &str, destination: usize) -> Result<Vec<Edit>, String> {
    let doc = laml::bpmn::parse(source)?;
    let node = doc.nodes.iter().find(|n| n.id == id).ok_or("Missing BPMN element")?;
    if node.kind == Kind::Boundary { return Err("Move the task that owns this boundary event".into()); }
    let target = record(source, destination)?;
    if !target.opens { return Err("Choose a pool, lane or expanded subprocess".into()); }
    let all = records(source);
    let mut ancestors = vec![destination];
    let mut parent = target.parent;
    while let Some(line) = parent { ancestors.push(line); parent = all.iter().find(|r| r.line == line).and_then(|r| r.parent); }
    let pool = doc.pools.iter().find(|p| ancestors.contains(&p.line));
    let subprocess = ancestors.iter().find_map(|line| doc.nodes.iter().find(|n| n.kind == Kind::Subprocess && n.line == *line));
    let short = node.id.rsplit('.').next().unwrap();
    let scope = subprocess.map(|n| n.id.as_str()).or_else(|| pool.map(|p| p.id.as_str()));
    let new_id = scope.map_or_else(|| short.to_owned(), |scope| format!("{scope}.{short}"));
    if new_id != id && doc.nodes.iter().any(|n| n.id == new_id) { return Err("The destination already contains this ID".into()); }
    if node.kind == Kind::Subprocess {
        return relocate_subprocess(source, &doc, node, destination, &new_id);
    }
    let attached: BTreeSet<_> = doc.nodes.iter().filter(|n| n.attached.as_deref() == Some(id)).map(|n| n.id.clone()).collect();
    let mut moving = attached.clone(); moving.insert(id.into());
    let map_id = |old: &str| if moving.contains(old) {
        new_id.rsplit_once('.').map_or_else(|| old.rsplit('.').next().unwrap().to_owned(), |(prefix, _)| format!("{prefix}.{}", old.rsplit('.').next().unwrap()))
    } else { old.to_owned() };
    let incident: BTreeSet<_> = doc.edges.iter().enumerate().filter(|(_, e)| moving.contains(&e.from) || moving.contains(&e.to)).map(|(i, _)| i).collect();
    let lines: BTreeSet<_> = doc.nodes.iter().filter(|n| moving.contains(&n.id)).map(|n| n.line).chain(incident.iter().map(|i| doc.edges[*i].line)).collect();
    let mut normalized = doc.clone();
    for n in normalized.nodes.iter_mut().filter(|n| moving.contains(&n.id)) { n.line = 0; }
    let mut edits = Vec::new();
    for line in lines { edits.push(rewrite_line(source, &normalized, line, &BTreeSet::new(), &incident, None)?); }
    let after = apply(source, &edits)?;
    let destination = crate::relation_edit::remap_line(source, destination, &edits);
    let mut declarations = vec![declaration(node, None)];
    let mut boundary_edges = BTreeSet::new();
    for boundary in doc.nodes.iter().filter(|n| attached.contains(&n.id)) {
        let (i, edge) = doc.edges.iter().enumerate().find(|(_, e)| e.from == boundary.id && e.line == boundary.line).ok_or("Missing attached-event flow")?;
        boundary_edges.insert(i);
        declarations.push(format!("{new_id} -({}{}@{}:{})-> {}", boundary.marker.as_deref().unwrap_or("timer"),
            if boundary.interrupting { "" } else { "+" }, boundary.id.rsplit('.').next().unwrap(), crate::relation_edit::quote(&boundary.label), map_id(&edge.to)));
    }
    let added = insert(&after, "semantics", Some(destination), None, &declarations.join("\n"))?;
    let mut after = apply(&after, &added)?;
    let flows = incident.iter().filter(|i| !boundary_edges.contains(i)).map(|i| {
        let edge = &doc.edges[*i];
        let from = map_id(&edge.from);
        let to = map_id(&edge.to);
        let op = match edge.kind { EdgeKind::Sequence => "-->", EdgeKind::Message => "~~>", EdgeKind::Data => "..>", EdgeKind::Association => ".." };
        format!("{from} {op} {to}{}", edge.label.as_ref().map_or(String::new(), |l| format!(" : {l}")))
    }).collect::<Vec<_>>().join("\n");
    if !flows.is_empty() { let edits = insert(&after, "semantics", None, None, &flows)?; after = apply(&after, &edits)?; }
    // Qualified selectors and placement references retain the moved identity.
    if new_id != id {
        let mut changes = Vec::new();
        let index = SourceIndex::new(&after);
        for r in records(&after).into_iter().filter(|r| r.section != "semantics") {
            let range = index.line(r.line).unwrap();
            for span in super::references::spans(code(&after[range.clone()]), &r.section, false) {
                let token = &after[range.start + span.start..range.start + span.end];
                if let Some(old) = moving.iter().find(|old| token == old.as_str()
                    || (old.rsplit('.').next() == Some(token) && doc.nodes.iter().filter(|n| n.id.rsplit('.').next() == Some(token)).count() == 1)) {
                    changes.push(Edit { range: range.start + span.start..range.start + span.end, replacement: map_id(old) });
                }
            }
        }
        after = apply(&after, &changes)?;
    }
    laml::bpmn::parse(&after)?;
    Ok(crate::graph_edit::difference(source, &after))
}

pub fn wrap_root_process(source: &str, id: &str, label: &str) -> Result<Vec<Edit>, String> {
    if !crate::graph_edit::identifier(id) { return Err("Use a simple pool ID".into()); }
    let doc = laml::bpmn::parse(source)?;
    if !doc.pools.is_empty() { return Err("The process already uses pools; add another pool instead".into()); }
    let index = SourceIndex::new(source);
    let implicit: BTreeSet<_> = doc.nodes.iter().filter(|n| {
        let short = n.id.rsplit('.').next().unwrap();
        (short.starts_with("gateway_") || short.starts_with("boundary_"))
            && index.line(n.line).is_some_and(|range| !source[range].contains(&format!("@{short}")))
    }).map(|n| n.line).collect();
    if !implicit.is_empty() {
        let edits = implicit.into_iter().map(|line| rewrite_line(source, &doc, line, &BTreeSet::new(), &BTreeSet::new(), None)).collect::<Result<Vec<_>, _>>()?;
        let normalized = apply(source, &edits)?;
        let wrap = wrap_root_process(&normalized, id, label)?;
        return Ok(crate::graph_edit::difference(source, &apply(&normalized, &wrap)?));
    }
    let mut depth = None; let mut start = 0; let mut end = None;
    for (i, raw) in source.lines().enumerate() {
        let text = code(raw).trim();
        if depth.is_none() {
            if text == "semantics {" { depth = Some(0usize); start = index.line(i + 2).map_or(source.len(), |r| r.start); }
        } else if text == "}" {
            if depth == Some(0) { end = Some(index.line(i + 1).unwrap().start); break; }
            depth = depth.map(|d| d - 1);
        } else if text.ends_with('{') { depth = depth.map(|d| d + 1); }
    }
    let end = end.ok_or("Missing semantics boundary")?;
    let newline = crate::element_edit::newline(source);
    let body = source[start..end].split_inclusive('\n').map(|line| format!("    {line}")).collect::<String>();
    let replacement = format!("    pool {id}(label={}) {{{newline}{body}    }}{newline}", crate::relation_edit::quote(label));
    let candidate = apply(source, &[Edit { range: start..end, replacement }])?;
    let mut refs = Vec::new(); let index = SourceIndex::new(&candidate);
    for r in records(&candidate) {
        if r.section == "settings" || (r.section == "styling" && !r.opens) { continue; }
        let range = index.line(r.line).unwrap(); let text = code(&candidate[range.clone()]);
        for span in super::references::spans(text, &r.section, false) {
            let token = &text[span.clone()];
            if let Some(node) = doc.nodes.iter().find(|n| n.id == token && (r.section != "semantics" || token.contains('.'))) {
                refs.push(Edit { range: range.start + span.start..range.start + span.end, replacement: format!("{id}.{}", node.id) });
            }
        }
    }
    let candidate = apply(&candidate, &refs)?;
    laml::bpmn::parse(&candidate)?;
    Ok(crate::graph_edit::difference(source, &candidate))
}

fn relocate_subprocess(source: &str, doc: &laml::bpmn::Document, node: &laml::bpmn::Node, destination: usize, new_id: &str) -> Result<Vec<Edit>, String> {
    let group = record(source, node.line)?;
    let boundary: BTreeSet<_> = doc.nodes.iter().filter(|n| n.attached.as_deref() == Some(&node.id)).map(|n| n.id.clone()).collect();
    let moves = |id: &str| id == node.id || id.starts_with(&format!("{}.", node.id)) || boundary.contains(id);
    let mapped = |id: &str| if boundary.contains(id) {
        new_id.rsplit_once('.').map_or_else(|| id.rsplit('.').next().unwrap().to_owned(), |(prefix, _)| format!("{prefix}.{}", id.rsplit('.').next().unwrap()))
    } else if id == node.id || id.starts_with(&format!("{}.", node.id)) { format!("{new_id}{}", &id[node.id.len()..]) } else { id.into() };
    let mut edits = move_record(source, node.line, Some(destination), None)?;
    let index = SourceIndex::new(source);
    let inside = |line| index.line(line).is_some_and(|r| group.block.contains(&r.start));
    let external: BTreeSet<_> = doc.edges.iter().enumerate().filter(|(_, e)| !inside(e.line) && (moves(&e.from) || moves(&e.to))).map(|(i, _)| i).collect();
    let mut normalized = doc.clone();
    for n in normalized.nodes.iter_mut().filter(|n| boundary.contains(&n.id)) { n.line = 0; }
    let lines: BTreeSet<_> = external.iter().map(|i| doc.edges[*i].line).chain(doc.nodes.iter().filter(|n| boundary.contains(&n.id)).map(|n| n.line)).collect();
    for line in lines { edits.push(rewrite_line(source, &normalized, line, &BTreeSet::new(), &external, None)?); }
    let mut boundary_flows = BTreeSet::new(); let mut declarations = Vec::new();
    for n in doc.nodes.iter().filter(|n| boundary.contains(&n.id)) {
        let (i, edge) = doc.edges.iter().enumerate().find(|(_, e)| e.from == n.id && e.line == n.line).ok_or("Missing boundary-event flow")?;
        boundary_flows.insert(i);
        declarations.push(format!("{new_id} -({}{}@{}:{})-> {}", n.marker.as_deref().unwrap_or("timer"), if n.interrupting { "" } else { "+" }, n.id.rsplit('.').next().unwrap(), crate::relation_edit::quote(&n.label), mapped(&edge.to)));
    }
    if !declarations.is_empty() { edits.extend(insert(source, "semantics", Some(destination), None, &declarations.join("\n"))?); }
    let edits = crate::element_edit::merge_insertions(edits);
    let mut candidate = apply(source, &edits)?;
    let flows = external.iter().filter(|i| !boundary_flows.contains(i)).map(|i| {
        let e = &doc.edges[*i]; let op = match e.kind { EdgeKind::Sequence => "-->", EdgeKind::Message => "~~>", EdgeKind::Data => "..>", EdgeKind::Association => ".." };
        format!("{} {op} {}{}", mapped(&e.from), mapped(&e.to), e.label.as_ref().map_or(String::new(), |l| format!(" : {l}")))
    }).collect::<Vec<_>>();
    if !flows.is_empty() { let edits = insert(&candidate, "semantics", None, None, &flows.join("\n"))?; candidate = apply(&candidate, &edits)?; }
    let mut refs = Vec::new();
    for r in records(&candidate) {
        let raw = code(&candidate[r.range.clone()]);
        for span in super::references::spans(raw, &r.section, false) {
            let value = &raw[span.clone()];
            if moves(value) && mapped(value) != value {
                refs.push(Edit { range: r.range.start + span.start..r.range.start + span.end, replacement: mapped(value) });
            }
        }
    }
    candidate = apply(&candidate, &refs)?;
    laml::bpmn::parse(&candidate)?;
    Ok(crate::graph_edit::difference(source, &candidate))
}
