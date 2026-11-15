//! Contextual BPMN transactions. Scope comes from source, never canvas coordinates.
use super::bpmn::rewrite_line;
use super::*;
use laml::bpmn::{Document as Bpmn, EdgeKind, Kind, Node};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Successor(String),
    Insert(usize),
    Content(usize),
    Boundary(String, String),
    Lane(usize),
}
fn checked(source: &str, candidate: String) -> Result<Vec<Edit>, String> {
    laml::bpmn::parse(&candidate)?;
    Ok(crate::graph_edit::difference(source, &candidate))
}
fn parent(source: &str, node: &Node) -> Result<Option<usize>, String> {
    Ok(record(source, node.line)?.parent)
}
fn qualified(doc: &Bpmn, parent: Option<usize>, source: &str, id: &str) -> String {
    let all = records(source);
    let mut at = parent;
    while let Some(line) = at {
        if let Some(n) = doc.nodes.iter().find(|n| n.line == line && n.kind == Kind::Subprocess) {
            return format!("{}.{id}", n.id);
        }
        if let Some(p) = doc.pools.iter().find(|p| p.line == line) {
            return format!("{}.{id}", p.id);
        }
        at = all.iter().find(|r| r.line == line).and_then(|r| r.parent);
    }
    id.into()
}
fn decl(ty: &str, id: &str, label: &str) -> Result<String, String> {
    if ty == "subprocess" {
        return Ok(format!("subprocess {id}(label={}) {{\n}}", crate::relation_edit::quote(label)));
    }
    if !bpmn::TYPES.iter().any(|(key, _)| *key == ty) {
        return Err("Choose a supported BPMN element".into());
    }
    let prefix = if ty == "task" {
        String::new()
    } else if ty.ends_with('@') || ty == "+" {
        ty.into()
    } else {
        format!("{ty} ")
    };
    Ok(format!("{prefix}{id}(label={})", crate::relation_edit::quote(label)))
}
/// Create a sibling successor, split one exact flow, add scoped content or attach an event.
pub fn create(
    source: &str,
    action: &Action,
    ty: &str,
    id: &str,
    label: &str,
    flow_label: &str,
) -> Result<Vec<Edit>, String> {
    if !crate::graph_edit::identifier(id) {
        return Err("Use a simple element ID".into());
    }
    if flow_label.contains(['\r', '\n', '{', '}']) {
        return Err("Enter a single-line flow label".into());
    }
    let doc = laml::bpmn::parse(source)?;
    let node = |id: &str| doc.nodes.iter().find(|n| n.id == id).ok_or("Missing BPMN element");
    let container = match action {
        Action::Successor(owner) | Action::Boundary(owner, _) => parent(source, node(owner)?)?,
        Action::Insert(index) => parent(source, node(&doc.edges.get(*index).ok_or("Missing flow")?.from)?)?,
        Action::Content(line) | Action::Lane(line) => Some(*line),
    };
    let new_id = qualified(&doc, container, source, id);
    if doc.nodes.iter().any(|n| n.id == new_id) {
        return Err("This scope already contains the ID".into());
    }
    let label = if label.is_empty() { id } else { label };
    let declaration = decl(ty, id, label)?;
    let flow_suffix = if flow_label.trim().is_empty() { String::new() } else { format!(" : {flow_label}") };
    let mut candidate = source.to_owned();
    let body = match action {
        Action::Content(_) => declaration,
        Action::Lane(line) => {
            if !doc.pools.iter().any(|p| p.line == *line) {
                return Err("Lanes belong directly to pools".into());
            }
            format!("lane {id}(label={}) {{\n}}", crate::relation_edit::quote(label))
        }
        Action::Successor(owner) => format!("{declaration}\n{owner} --> {new_id}{flow_suffix}"),
        Action::Boundary(owner, marker) => {
            if !["timer", "timer+", "message", "message+", "error"].contains(&marker.as_str()) {
                return Err("Choose a boundary event marker".into());
            }
            // The supported notation attaches an event together with its outgoing flow.
            let target = format!("{id}Target");
            let target_id = qualified(&doc, container, source, &target);
            if doc.nodes.iter().any(|n| n.id == target_id) {
                return Err("The boundary target ID already exists".into());
            }
            format!(
                "{}\n{owner} -({marker}@{id}:{})-> {target_id}",
                decl(ty, &target, &target)?,
                crate::relation_edit::quote(label)
            )
        }
        Action::Insert(index) => {
            let edge = &doc.edges[*index];
            if edge.kind != EdgeKind::Sequence {
                return Err("Insert elements on sequence flows only".into());
            }
            let mut changed = doc.clone();
            changed.edges[*index].to = new_id.clone();
            // Preserve the original label on the original (upstream) flow.
            let edit = rewrite_line(source, &changed, edge.line, &BTreeSet::new(), &BTreeSet::new(), None)?;
            candidate = apply(source, &[edit])?;
            let declaration_line = node(&edge.from)?.line;
            let mapped_line = crate::relation_edit::remap_line(
                source,
                declaration_line,
                &crate::graph_edit::difference(source, &candidate),
            );
            let edits = insert(&candidate, "semantics", None, Some(mapped_line), &declaration)?;
            candidate = apply(&candidate, &edits)?;
            let mapped = container.map(|l| {
                crate::relation_edit::remap_line(
                    source,
                    l,
                    &crate::graph_edit::difference(source, &candidate),
                )
            });
            let edits = insert(&candidate, "semantics", mapped, None, &format!("{new_id} --> {}", edge.to))?;
            candidate = apply(&candidate, &edits)?;
            candidate =
                flow_rules(&candidate, &edge.from, &edge.to, &edge.from, &new_id, Some((&new_id, &edge.to)))?;
            return checked(source, candidate);
        }
    };
    let edits = insert(&candidate, "semantics", container, None, &body)?;
    candidate = apply(&candidate, &edits)?;
    checked(source, candidate)
}
pub fn connect(source: &str, from: &str, to: &str, operator: &str) -> Result<Vec<Edit>, String> {
    if !["-->", "~~>", "..>", ".."].contains(&operator) {
        return Err("Choose a BPMN flow type".into());
    }
    let doc = laml::bpmn::parse(source)?;
    if ![from, to].iter().all(|id| doc.nodes.iter().any(|n| &n.id == id)) {
        return Err("Choose two BPMN nodes".into());
    }
    let edits = insert(source, "semantics", None, None, &format!("{from} {operator} {to}"))?;
    checked(source, apply(source, &edits)?)
}
/// Reconnect by exact edge index, retaining labels, types and attachment intent.
pub fn reconnect(source: &str, index: usize, from: bool, target: &str) -> Result<Vec<Edit>, String> {
    let mut doc = laml::bpmn::parse(source)?;
    if !doc.nodes.iter().any(|n| n.id == target) {
        return Err("Choose a BPMN node".into());
    }
    let edge = doc.edges.get_mut(index).ok_or("Missing flow")?;
    let (old_from, old_to, line) = (edge.from.clone(), edge.to.clone(), edge.line);
    if from {
        edge.from = target.into();
    } else {
        edge.to = target.into();
    }
    let (new_from, new_to) = (edge.from.clone(), edge.to.clone());
    if new_from == old_from && new_to == old_to {
        return Ok(Vec::new());
    }
    // Boundary source syntax defines ownership; moving that source would delete the event.
    if from && doc.nodes.iter().any(|n| n.id == old_from && n.kind == Kind::Boundary) {
        return Err("Move the owning activity to relocate a boundary event".into());
    }
    let boundary = doc.nodes.iter().find(|n| n.id == old_from && n.kind == Kind::Boundary);
    let owner_parent = boundary.map(|n| parent(source, n)).transpose()?.flatten();
    let edge = &doc.edges[index];
    let body = if let Some(n) = boundary {
        format!(
            "{} -({}{}@{}:{})-> {}",
            n.attached.as_deref().ok_or("Missing boundary owner")?,
            n.marker.as_deref().unwrap_or("timer"),
            if n.interrupting { "" } else { "+" },
            n.id.rsplit('.').next().unwrap(),
            crate::relation_edit::quote(&n.label),
            edge.to
        )
    } else {
        let op = match edge.kind {
            EdgeKind::Sequence => "-->",
            EdgeKind::Message => "~~>",
            EdgeKind::Data => "..>",
            EdgeKind::Association => "..",
        };
        format!(
            "{} {op} {}{}",
            edge.from,
            edge.to,
            edge.label.as_ref().map_or(String::new(), |l| format!(" : {l}"))
        )
    };
    let removed = BTreeSet::from([index]);
    let edit = rewrite_line(source, &doc, line, &BTreeSet::new(), &removed, None)?;
    let mut candidate = apply(source, &[edit])?;
    let mapped = owner_parent.map(|l| {
        crate::relation_edit::remap_line(source, l, &crate::graph_edit::difference(source, &candidate))
    });
    let edits = insert(&candidate, "semantics", mapped, None, &body)?;
    candidate = apply(&candidate, &edits)?;
    checked(source, flow_rules(&candidate, &old_from, &old_to, &new_from, &new_to, None)?)
}

fn flow_rules(
    source: &str,
    from: &str,
    to: &str,
    new_from: &str,
    new_to: &str,
    downstream: Option<(&str, &str)>,
) -> Result<String, String> {
    let mut edits = Vec::new();
    for r in records(source).into_iter().filter(|r| r.section == "layout") {
        let Some(rest) = r.heading.strip_prefix("flow[") else {
            continue;
        };
        let Some((ends, tail)) = rest.split_once(']') else {
            continue;
        };
        let Some((a, b)) = ends.split_once("->") else {
            continue;
        };
        let matches =
            |full: &str, short: &str| full == short.trim() || full.ends_with(&format!(".{}", short.trim()));
        if !matches(from, a) || !matches(to, b) {
            continue;
        }
        let raw = &source[r.range.clone()];
        let indent = &raw[..raw.len() - raw.trim_start().len()];
        let nl = crate::element_edit::newline(source);
        let comment = &raw[code(raw).len()..];
        let mut body = format!("{indent}flow[{new_from}->{new_to}]{tail}{comment}{nl}");
        if let Some((a, b)) = downstream {
            let properties = tail
                .trim()
                .trim_start_matches(':')
                .split(';')
                .map(str::trim)
                .filter(|p| p.starts_with("to=") || p.starts_with("to ="))
                .collect::<Vec<_>>();
            let upstream = tail
                .trim()
                .trim_start_matches(':')
                .split(';')
                .map(str::trim)
                .filter(|p| !p.starts_with("to=") && !p.starts_with("to ="))
                .collect::<Vec<_>>();
            body = if upstream.is_empty() {
                format!("{indent}{comment}{nl}")
            } else {
                format!("{indent}flow[{new_from}->{new_to}] : {}{comment}{nl}", upstream.join("; "))
            };
            if !properties.is_empty() {
                body.push_str(&format!("{indent}flow[{a}->{b}] : {}{nl}", properties.join("; ")));
            }
        }
        edits.push(Edit { range: r.block, replacement: body });
    }
    apply(source, &edits)
}

#[cfg(test)]
mod tests {
    use super::*;
    const SOURCE: &str = "settings {\n type: bpmn\n}\nsemantics {\n pool P {\n  lane L {\n   startEvent A\n   A --> B(\"Work\") --> C // keep\n  }\n }\n}\n";
    #[test]
    fn insert_preserves_inline_neighbors_and_membership() {
        let edits = create(SOURCE, &Action::Insert(1), "task", "New", "Inserted", "").unwrap();
        let candidate = apply(SOURCE, &edits).unwrap();
        let doc = laml::bpmn::parse(&candidate).unwrap();
        let n = doc.nodes.iter().find(|n| n.id == "P.New").unwrap();
        assert_eq!(n.lane.as_deref(), Some("L"));
        assert!(doc.edges.iter().any(|e| e.from == "P.B" && e.to == "P.New"));
        assert!(doc.edges.iter().any(|e| e.from == "P.New" && e.to == "P.C"));
        assert!(candidate.contains("// keep"));
    }
    #[test]
    fn successors_and_boundaries_use_owner_scope() {
        for action in [Action::Successor("P.B".into()), Action::Boundary("P.B".into(), "timer+".into())] {
            let edits = create(SOURCE, &action, "task", "Added", "Added", "yes").unwrap();
            let candidate = apply(SOURCE, &edits).unwrap();
            let doc = laml::bpmn::parse(&candidate).unwrap();
            assert!(doc.nodes.iter().any(|n| n.id == "P.Added" && n.lane.as_deref() == Some("L")));
        }
    }
    #[test]
    fn reconnect_preserves_other_inline_edges() {
        let edits = reconnect(SOURCE, 1, true, "P.A").unwrap();
        let candidate = apply(SOURCE, &edits).unwrap();
        let doc = laml::bpmn::parse(&candidate).unwrap();
        assert!(doc.edges.iter().any(|e| e.from == "P.A" && e.to == "P.B"));
        assert!(doc.edges.iter().any(|e| e.from == "P.A" && e.to == "P.C"));
    }
}
