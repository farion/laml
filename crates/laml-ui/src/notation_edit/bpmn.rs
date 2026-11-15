//! BPMN inline declarations are expanded only on the edited source line.
//! This preserves unrelated nodes/flows instead of deleting an entire inline chain.
use super::*;
use laml::bpmn::{Document, EdgeKind, Kind, Node};
use std::collections::BTreeSet;

pub const TYPES: &[(&str, &str)] = &[
    ("task", "Task"),
    ("userTask", "User task"),
    ("serviceTask", "Service task"),
    ("sendTask", "Send task"),
    ("receiveTask", "Receive task"),
    ("scriptTask", "Script task"),
    ("manualTask", "Manual task"),
    ("startEvent", "Start event"),
    ("endEvent", "End event"),
    ("messageStart", "Message start"),
    ("timerStart", "Timer start"),
    ("messageEnd", "Message end"),
    ("errorEnd", "Error end"),
    ("terminateEnd", "Terminate end"),
    ("intermediateTimer", "Intermediate timer"),
    ("intermediateMessage", "Intermediate message"),
    ("<X>@", "Exclusive gateway"),
    ("<+>@", "Parallel gateway"),
    ("<O>@", "Inclusive gateway"),
    ("<event>@", "Event-based gateway"),
    ("+", "Collapsed subprocess"),
    ("data", "Data object"),
    ("store", "Data store"),
    ("note", "Annotation"),
];
pub fn keyword(n: &Node) -> &'static str {
    match (n.kind, n.marker.as_deref()) {
        (Kind::Task, _) => "task",
        (Kind::UserTask, _) => "userTask",
        (Kind::ServiceTask, _) => "serviceTask",
        (Kind::SendTask, _) => "sendTask",
        (Kind::ReceiveTask, _) => "receiveTask",
        (Kind::ScriptTask, _) => "scriptTask",
        (Kind::ManualTask, _) => "manualTask",
        (Kind::Start, Some("message")) => "messageStart",
        (Kind::Start, Some("timer")) => "timerStart",
        (Kind::Start, _) => "startEvent",
        (Kind::End, Some("message")) => "messageEnd",
        (Kind::End, Some("error")) => "errorEnd",
        (Kind::End, Some("terminate")) => "terminateEnd",
        (Kind::End, _) => "endEvent",
        (Kind::Intermediate, Some("timer")) => "intermediateTimer",
        (Kind::Intermediate, _) => "intermediateMessage",
        (Kind::Exclusive, _) => "<X>@",
        (Kind::Parallel, _) => "<+>@",
        (Kind::Inclusive, _) => "<O>@",
        (Kind::EventBased, _) => "<event>@",
        (Kind::Collapsed, _) => "+",
        (Kind::Data, _) => "data",
        (Kind::Store, _) => "store",
        (Kind::Annotation, _) => "note",
        _ => "",
    }
}
fn declaration(n: &Node, override_type: Option<&str>) -> String {
    let ty = override_type.unwrap_or_else(|| keyword(n));
    let prefix = if ty == "task" {
        String::new()
    } else if ty.ends_with('@') || ty == "+" {
        ty.into()
    } else {
        format!("{ty} ")
    };
    let mut attrs = format!("label={}", crate::relation_edit::quote(&n.label));
    if let Some(icon) = &n.icon {
        attrs.push_str(&format!("; icon={}", crate::relation_edit::quote(icon)));
    }
    format!("{prefix}{}({attrs})", n.id.rsplit('.').next().unwrap())
}
pub(super) fn rewrite_line(
    source: &str,
    doc: &Document,
    line: usize,
    deleted_nodes: &BTreeSet<String>,
    deleted_edges: &BTreeSet<usize>,
    changed_type: Option<(&str, &str)>,
) -> Result<Edit, String> {
    let r = record(source, line)?;
    if r.opens {
        return Err("Edit the container header or its contents separately".into());
    }
    let raw = &source[r.range.clone()];
    let indent = &raw[..raw.len() - raw.trim_start().len()];
    let newline = crate::element_edit::newline(source);
    let mut bodies = Vec::new();
    let comment = &raw[code(raw).len()..];
    if !comment.is_empty() {
        bodies.push(comment.into());
    }
    let mut attached = BTreeSet::new();
    for n in doc
        .nodes
        .iter()
        .filter(|n| n.line == line && !deleted_nodes.contains(&n.id))
    {
        if n.kind == Kind::Boundary {
            continue;
        }
        bodies.push(declaration(
            n,
            changed_type.filter(|(id, _)| *id == n.id).map(|(_, ty)| ty),
        ));
    }
    for (index, edge) in doc.edges.iter().enumerate().filter(|(_, e)| e.line == line) {
        if deleted_edges.contains(&index)
            || deleted_nodes.contains(&edge.from)
            || deleted_nodes.contains(&edge.to)
        {
            continue;
        }
        let boundary = doc
            .nodes
            .iter()
            .find(|n| n.id == edge.from && n.kind == Kind::Boundary && n.line == line);
        if let Some(n) = boundary.filter(|n| attached.insert(n.id.clone())) {
            let marker = n.marker.as_deref().unwrap_or("timer");
            bodies.push(format!(
                "{} -({marker}{}@{}:{})-> {}",
                n.attached.as_deref().ok_or("Missing boundary owner")?,
                if n.interrupting { "" } else { "+" },
                n.id.rsplit('.').next().unwrap(),
                crate::relation_edit::quote(&n.label),
                edge.to
            ));
        } else {
            let operator = match edge.kind {
                EdgeKind::Sequence => "-->",
                EdgeKind::Message => "~~>",
                EdgeKind::Data => "..>",
                EdgeKind::Association => "..",
            };
            let label = edge
                .label
                .as_ref()
                .map_or(String::new(), |label| format!(" : {label}"));
            bodies.push(format!("{} {operator} {}{label}", edge.from, edge.to));
        }
    }
    let replacement = bodies
        .into_iter()
        .map(|body| format!("{indent}{body}{newline}"))
        .collect();
    Ok(Edit {
        range: r.block,
        replacement,
    })
}
pub fn change_type(source: &str, id: &str, ty: &str) -> Result<Vec<Edit>, String> {
    if !TYPES.iter().any(|(key, _)| *key == ty) {
        return Err("Choose a BPMN element type".into());
    }
    let doc = laml::bpmn::parse(source)?;
    let node = doc
        .nodes
        .iter()
        .find(|n| n.id == id)
        .ok_or("Missing BPMN element")?;
    if matches!(node.kind, Kind::Boundary | Kind::Subprocess) {
        return Err("This structural element cannot change to a task or event".into());
    }
    let edit = rewrite_line(
        source,
        &doc,
        node.line,
        &BTreeSet::new(),
        &BTreeSet::new(),
        Some((id, ty)),
    )?;
    let candidate = apply(source, std::slice::from_ref(&edit))?;
    laml::bpmn::parse(&candidate)?;
    Ok(vec![edit])
}
pub fn delete_item(source: &str, id: &str) -> Result<Deletion, String> {
    let doc = laml::bpmn::parse(source)?;
    let mut nodes = BTreeSet::new();
    let mut edges = BTreeSet::new();
    if let Some(index) = id
        .strip_prefix("flow:")
        .and_then(|s| s.parse::<usize>().ok())
    {
        if index >= doc.edges.len() {
            return Err("Missing BPMN flow".into());
        }
        if let Some(boundary) = doc.nodes.iter().find(|n| {
            n.kind == Kind::Boundary
                && n.id == doc.edges[index].from
                && n.line == doc.edges[index].line
        }) {
            nodes.insert(boundary.id.clone());
            for (i, edge) in doc.edges.iter().enumerate() {
                if edge.from == boundary.id || edge.to == boundary.id {
                    edges.insert(i);
                }
            }
        }
        edges.insert(index);
    } else {
        let node = doc
            .nodes
            .iter()
            .find(|n| n.id == id)
            .ok_or("Missing BPMN element")?;
        if node.kind == Kind::Subprocess {
            return delete(source, node.line, Some(id), false);
        }
        nodes.insert(id.into());
        for node in &doc.nodes {
            if node.attached.as_deref() == Some(id) {
                nodes.insert(node.id.clone());
            }
        }
        for (i, edge) in doc.edges.iter().enumerate() {
            if nodes.contains(&edge.from) || nodes.contains(&edge.to) {
                edges.insert(i);
            }
        }
    }
    let lines: BTreeSet<_> = doc
        .nodes
        .iter()
        .filter(|n| nodes.contains(&n.id))
        .map(|n| n.line)
        .chain(edges.iter().map(|i| doc.edges[*i].line))
        .collect();
    let mut edits = Vec::new();
    let mut affected: Vec<_> = nodes.iter().map(|id| format!("Element {id}")).collect();
    for i in &edges {
        affected.push(format!(
            "Flow {} → {}",
            doc.edges[*i].from, doc.edges[*i].to
        ));
    }
    for line in lines {
        edits.push(rewrite_line(source, &doc, line, &nodes, &edges, None)?);
    }
    for r in records(source)
        .into_iter()
        .filter(|r| r.section == "layout")
    {
        let references = fields::tokens(&r.heading)
            .iter()
            .any(|span| nodes.contains(&r.heading[span.clone()]));
        if references {
            affected.push(format!("Layout {}", r.heading));
            edits.push(Edit {
                range: r.block,
                replacement: String::new(),
            });
        }
    }
    Ok(Deletion { edits, affected })
}
pub fn node_property(
    source: &str,
    id: &str,
    property: &str,
    value: &str,
) -> Result<Vec<Edit>, String> {
    let mut doc = laml::bpmn::parse(source)?;
    let node = doc
        .nodes
        .iter_mut()
        .find(|n| n.id == id)
        .ok_or("Missing BPMN element")?;
    match property {
        "label" => node.label = value.into(),
        "icon" => node.icon = (!value.is_empty()).then(|| value.into()),
        "marker"
            if node.kind == Kind::Boundary && ["timer", "message", "error"].contains(&value) =>
        {
            node.marker = Some(value.into())
        }
        "interrupting" if node.kind == Kind::Boundary && ["true", "false"].contains(&value) => {
            node.interrupting = value == "true"
        }
        _ => return Err("Property does not apply to this BPMN element".into()),
    }
    let line = node.line;
    if node.kind == Kind::Subprocess {
        return attribute(source, line, Some(id), property, value);
    }
    let edit = rewrite_line(source, &doc, line, &BTreeSet::new(), &BTreeSet::new(), None)?;
    let candidate = apply(source, std::slice::from_ref(&edit))?;
    laml::bpmn::parse(&candidate)?;
    Ok(vec![edit])
}
pub fn flow_property(
    source: &str,
    index: usize,
    property: &str,
    value: &str,
) -> Result<Vec<Edit>, String> {
    if value.contains(['\r', '\n', '{', '}']) {
        return Err("Enter one flow property".into());
    }
    let mut doc = laml::bpmn::parse(source)?;
    let edge = doc.edges.get_mut(index).ok_or("Missing BPMN flow")?;
    match property {
        "from" => edge.from = value.into(),
        "to" => edge.to = value.into(),
        "label" => edge.label = (!value.is_empty()).then(|| value.into()),
        "type" => {
            edge.kind = match value {
                "-->" => EdgeKind::Sequence,
                "~~>" => EdgeKind::Message,
                "..>" => EdgeKind::Data,
                ".." => EdgeKind::Association,
                _ => return Err("Unknown flow kind".into()),
            }
        }
        _ => return Err("Unknown flow property".into()),
    }
    let line = edge.line;
    let edit = rewrite_line(source, &doc, line, &BTreeSet::new(), &BTreeSet::new(), None)?;
    let candidate = apply(source, std::slice::from_ref(&edit))?;
    laml::bpmn::parse(&candidate)?;
    Ok(vec![edit])
}

mod placement;
pub use placement::{relocate, wrap_root_process};

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
        assert_eq!(doc.edges.len(), 1);
        assert_eq!(doc.edges[0].to, "D");
        assert!(candidate.contains("// retain"));
    }
}
