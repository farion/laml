//! Source-level alignment commands; edits are preflighted by the real compiler.
use crate::{
    document::{Document, Edit},
    placement_edit::description,
    source_patch::{insert_section, remove_line, replace_code},
    style_edit::code,
};
use laml::syntax::{Alignment, AlignmentRule, FreeAlignment, LayoutRule, Placement};
use std::collections::BTreeSet;
#[cfg(test)]
mod tests;

pub fn attached(rule: &LayoutRule) -> Option<FreeAlignment> {
    Some(match (rule.placement, rule.align?) {
        (_, Alignment::Left) => FreeAlignment::Left,
        (_, Alignment::Right) => FreeAlignment::Right,
        (_, Alignment::Top) => FreeAlignment::Top,
        (_, Alignment::Bottom) => FreeAlignment::Bottom,
        (Placement::Left | Placement::Right, Alignment::Center) => FreeAlignment::CenterY,
        (_, Alignment::Center) => FreeAlignment::CenterX,
    })
}
pub fn declaration(first: &str, second: &str, align: FreeAlignment) -> String {
    format!("{first} ? {second} : align={}", align.keyword())
}
pub fn remaining(align: FreeAlignment, edit_x: bool, edit_y: bool) -> Option<FreeAlignment> {
    let (x, y) = align.anchors();
    match (x.filter(|_| !edit_x), y.filter(|_| !edit_y)) {
        (Some(_), Some(_)) => Some(align),
        (Some(0.0), _) => Some(FreeAlignment::Left),
        (Some(1.0), _) => Some(FreeAlignment::Right),
        (Some(_), _) => Some(FreeAlignment::CenterX),
        (_, Some(0.0)) => Some(FreeAlignment::Top),
        (_, Some(1.0)) => Some(FreeAlignment::Bottom),
        (_, Some(_)) => Some(FreeAlignment::CenterY),
        _ => None,
    }
}
#[derive(Clone, Debug)]
pub struct AlignmentPlan {
    pub edits: Vec<Edit>,
    pub removed: Vec<String>,
    pub preserved: Vec<String>,
    pub added: Vec<String>,
    pub source: String,
}
fn edge(rule: &LayoutRule) -> (&str, &str) {
    match rule.placement {
        Placement::Left | Placement::Above => (&rule.first, &rule.second),
        _ => (&rule.second, &rule.first),
    }
}
fn reachable(edges: &[(&str, &str)], from: &str, to: &str) -> bool {
    let mut queue = vec![from];
    let mut seen = BTreeSet::new();
    while let Some(node) = queue.pop() {
        if node == to {
            return true;
        }
        if seen.insert(node) {
            queue.extend(edges.iter().filter(|(a, _)| *a == node).map(|(_, b)| *b));
        }
    }
    false
}

pub fn plan(source: &str, ids: &[String], align: FreeAlignment) -> Result<AlignmentPlan, String> {
    let graph = crate::notation::parse_graph(source).map_err(|e| e.message)?;
    let Some(reference) = ids.last() else {
        return Err("Select at least two nodes".into());
    };
    let selected: BTreeSet<_> = ids.iter().map(String::as_str).collect();
    if selected.len() < 2 || selected.len() != ids.len() {
        return Err("Select at least two distinct nodes".into());
    }
    let node = |id: &str| {
        graph
            .nodes
            .iter()
            .find(|node| node.id == id)
            .ok_or_else(|| format!("Unknown node: {id}"))
    };
    let parent = &node(reference)?.parent;
    for id in ids {
        if &node(id)?.parent != parent {
            return Err("Visual alignment currently requires nodes in the same container".into());
        }
    }
    let moved: BTreeSet<_> = ids[..ids.len() - 1].iter().map(String::as_str).collect();
    let (x, y) = align.anchors();
    let (edit_x, edit_y) = (x.is_some(), y.is_some());
    let mut plan = AlignmentPlan {
        edits: vec![],
        removed: vec![],
        preserved: vec![],
        added: vec![],
        source: String::new(),
    };
    if ids[..ids.len() - 1].iter().all(|id| {
        graph
            .alignments
            .iter()
            .any(|rule| matches(rule, id, reference, align))
    }) {
        plan.source = source.into();
        return Ok(plan);
    }
    let original = |line: usize| {
        code(source.lines().nth(line - 1).unwrap())
            .trim()
            .to_string()
    };
    for rule in &graph.layout {
        let horizontal = matches!(rule.placement, Placement::Left | Placement::Right);
        let axis_edited = if horizontal { edit_x } else { edit_y };
        let edges: Vec<_> = graph
            .layout
            .iter()
            .filter(|r| matches!(r.placement, Placement::Left | Placement::Right) == horizontal)
            .map(edge)
            .collect();
        let (a, b) = edge(rule);
        let cut = axis_edited
            && ((moved.contains(a) && selected.iter().any(|id| reachable(&edges, b, id)))
                || (moved.contains(b) && selected.iter().any(|id| reachable(&edges, id, a))));
        let pair_selected =
            selected.contains(rule.first.as_str()) && selected.contains(rule.second.as_str());
        let old_alignment = attached(rule);
        if cut {
            plan.removed.push(original(rule.line));
            let keep = old_alignment.and_then(|old| {
                if pair_selected {
                    remaining(old, edit_x, edit_y)
                } else {
                    Some(old)
                }
            });
            if let Some(keep) = keep {
                let text = declaration(&rule.first, &rule.second, keep);
                plan.edits
                    .push(replace_code(source, rule.line, text.clone())?);
                plan.preserved.push(text);
            } else {
                plan.edits.push(remove_line(source, rule.line)?);
            }
        } else if pair_selected
            && let Some(old) = old_alignment
            && remaining(old, edit_x, edit_y) != Some(old)
        {
            // Keep the placement and its gap; only supersede the edited alignment attribute.
            let mut text = description(&rule.first, &rule.second, rule.placement);
            if let Some(gap) = rule.gap {
                text.push_str(&format!(" : gap={gap}"));
            }
            plan.removed.push(original(rule.line));
            plan.preserved.push(text.clone());
            plan.edits.push(replace_code(source, rule.line, text)?);
        }
    }
    for rule in &graph.alignments {
        if !(selected.contains(rule.first.as_str()) && selected.contains(rule.second.as_str())) {
            continue;
        }
        let keep = remaining(rule.align, edit_x, edit_y);
        if keep == Some(rule.align) {
            continue;
        }
        plan.removed.push(original(rule.line));
        if let Some(keep) = keep {
            let text = declaration(&rule.first, &rule.second, keep);
            plan.edits
                .push(replace_code(source, rule.line, text.clone())?);
            plan.preserved.push(text);
        } else {
            plan.edits.push(remove_line(source, rule.line)?);
        }
    }
    for id in &ids[..ids.len() - 1] {
        plan.added.push(declaration(id, reference, align));
    }
    let newline = if source.contains("\r\n") {
        "\r\n    "
    } else {
        "\n    "
    };
    plan.edits.push(insert_section(
        source,
        &graph,
        "layout",
        &plan.added.join(newline),
    )?);
    let mut doc = Document::new(source.into());
    doc.apply(0, "Align nodes", plan.edits.clone())
        .map_err(|e| format!("Invalid alignment edit: {e:?}"))?;
    crate::notation::parse_graph(doc.source()).map_err(|e| e.message)?;
    plan.source = doc.source().into();
    Ok(plan)
}

/// Center/edge-equivalence paths on one axis, excluding the moved node itself.
pub fn connected_without(
    graph: &laml::syntax::Diagram,
    moved: &str,
    from: &str,
    target: &str,
    x: bool,
    anchor: f64,
) -> bool {
    let mut edges = Vec::new();
    let mut add = |a: &str, b: &str, align: FreeAlignment| {
        let (ax, ay) = align.anchors();
        if a != moved && b != moved && (if x { ax } else { ay }) == Some(anchor) {
            edges.push((a.to_string(), b.to_string()));
            edges.push((b.to_string(), a.to_string()));
        }
    };
    for rule in &graph.alignments {
        add(&rule.first, &rule.second, rule.align);
    }
    for rule in &graph.layout {
        if let Some(align) = attached(rule) {
            add(&rule.first, &rule.second, align);
        }
    }
    let edges: Vec<_> = edges
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    reachable(&edges, from, target)
}

pub fn without_alignment(rule: &LayoutRule) -> String {
    let mut text = description(&rule.first, &rule.second, rule.placement);
    if let Some(gap) = rule.gap {
        text.push_str(&format!(" : gap={gap}"));
    }
    text
}

/// Validate postconditions without depending on source spelling.
pub fn matches(rule: &AlignmentRule, a: &str, b: &str, align: FreeAlignment) -> bool {
    rule.align == align
        && ((rule.first == a && rule.second == b) || (rule.first == b && rule.second == a))
}
