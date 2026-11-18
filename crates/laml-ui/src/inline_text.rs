//! Source-backed text targets. Picking uses measured compiler text, not node whitespace.
mod sequence_fragments;
#[cfg(test)]
mod tests;
use crate::{document::Edit, notation::Notation, notation_edit as edit, preview::Preview};
use laml::authoring::TextRegion;

#[derive(Clone, Debug)]
pub enum Target {
    Node(String),
    Port(String),
    Relation(usize),
    SequenceMessage(usize),
    SequenceGuard(usize),
    Field(edit::fields::Field),
    Attribute { line: usize, id: String },
    BpmnNode(String),
    BpmnFlow(usize),
}
#[derive(Clone, Debug)]
pub struct Text {
    pub target: Target,
    pub value: String,
    pub multiline: bool,
    pub line: usize,
    pub bounds: (f64, f64, f64, f64),
}
impl Text {
    pub fn edits(&self, source: &str, value: &str) -> Result<Vec<Edit>, String> {
        if !self.multiline && value.contains(['\r', '\n']) {
            return Err("This text does not support line breaks".into());
        }
        match &self.target {
            Target::Node(id) => crate::node_edit::label(source, id, value),
            Target::Port(id) => crate::port_edit::label(source, id, value),
            Target::Relation(line) => crate::relation_edit::label(source, *line, value),
            Target::SequenceMessage(line) => {
                if let Some(mut field) = edit::item_fields(source, *line, None)
                    .into_iter()
                    .find(|f| f.name == "Label")
                {
                    field.kind = edit::fields::Kind::Quoted;
                    return edit::fields::edit(source, &field, value);
                }
                let range = crate::source::SourceIndex::new(source)
                    .line(*line)
                    .ok_or("Missing message")?;
                let at = range.start
                    + crate::style_edit::code(&source[range.clone()])
                        .trim_end()
                        .len();
                Ok(vec![Edit {
                    range: at..at,
                    replacement: format!(": {}", crate::relation_edit::quote(value)),
                }])
            }
            Target::SequenceGuard(line) => sequence_fragments::edits(source, *line, value),
            Target::Field(field) => edit::fields::edit(source, field, value),
            Target::Attribute { line, id } => {
                edit::attribute(source, *line, Some(id), "label", value)
            }
            Target::BpmnNode(id) => edit::bpmn::node_property(source, id, "label", value),
            Target::BpmnFlow(index) => edit::bpmn::flow_property(source, *index, "label", value),
        }
    }
}
fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn matches(value: &str, drawn: &str) -> bool {
    let (value, drawn) = (normalize(value), normalize(drawn));
    !drawn.is_empty() && (value == drawn || format!(" {value} ").contains(&format!(" {drawn} ")))
}
fn distance(b: (f64, f64, f64, f64), x: f64, y: f64) -> f64 {
    (b.0 - x)
        .max(0.0)
        .max(x - b.2)
        .hypot((b.1 - y).max(0.0).max(y - b.3))
}
fn field_text(source: &str, line: usize, id: Option<&str>, drawn: &str) -> Vec<Text> {
    edit::item_fields(source, line, id)
        .into_iter()
        .filter(|f| {
            (matches!(f.kind, edit::fields::Kind::Quoted)
                || [
                    "Label",
                    "Text",
                    "Condition",
                    "Description",
                    "Technology",
                    "State",
                    "Initial state",
                ]
                .contains(&f.name.as_str()))
                && ![
                    "Source",
                    "Target",
                    "Reference",
                    "Name / reference",
                    "Type",
                    "Icon",
                ]
                .contains(&f.name.as_str())
                && matches(&f.value, drawn)
        })
        .map(|field| Text {
            multiline: matches!(field.kind, edit::fields::Kind::Quoted)
                || (Notation::of(source) == Notation::Sequence && field.name == "Label"),
            value: field.value.clone(),
            target: Target::Field(field),
            line,
            bounds: (0.0, 0.0, 0.0, 0.0),
        })
        .collect()
}
/// Resolve the smallest actual text hit, then the nearest matching semantic owner.
/// Repeated labels are disambiguated geometrically; equal candidates fail closed.
pub fn hit(preview: &Preview, source: &str, x: f64, y: f64) -> Option<Text> {
    if let Some(text) = sequence_fragments::hit(preview, source, x, y) {
        return Some(text);
    }
    let drawn = preview
        .texts
        .iter()
        .filter(|t| {
            x >= t.x - 2.0
                && x <= t.x + t.width + 2.0
                && y >= t.y - 2.0
                && y <= t.y + t.height + 2.0
        })
        .min_by(|a, b| (a.width * a.height).total_cmp(&(b.width * b.height)));
    if let Some(text) = drawn.and_then(|drawn| resolve(preview, source, drawn)) {
        return Some(text);
    }
    sequence_arrow(preview, source, x, y)
}
fn sequence_arrow(preview: &Preview, source: &str, x: f64, y: f64) -> Option<Text> {
    if preview.notation != Notation::Sequence {
        return None;
    }
    let mut hits = preview
        .regions
        .iter()
        .filter(|r| r.kind == "message" && !r.path.is_empty())
        .filter_map(|r| {
            let distance = r
                .path
                .windows(2)
                .map(|p| {
                    let (dx, dy) = (p[1].0 - p[0].0, p[1].1 - p[0].1);
                    let length = dx * dx + dy * dy;
                    let t = if length > 0.0 {
                        (((x - p[0].0) * dx + (y - p[0].1) * dy) / length).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    (x - p[0].0 - t * dx).hypot(y - p[0].1 - t * dy)
                })
                .fold(f64::INFINITY, f64::min);
            (distance <= 6.0).then_some((distance, r))
        })
        .collect::<Vec<_>>();
    hits.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (distance, region) = hits.first()?;
    if hits
        .get(1)
        .is_some_and(|next| (next.0 - distance).abs() < 1e-6)
    {
        return None;
    }
    let value = edit::item_fields(source, region.line, None)
        .into_iter()
        .find(|f| f.name == "Label")
        .map(|f| f.value)
        .unwrap_or_default();
    Some(Text {
        target: Target::SequenceMessage(region.line),
        value,
        multiline: true,
        line: region.line,
        bounds: (x, y - 24.0, x + 100.0, y),
    })
}
fn resolve(preview: &Preview, source: &str, drawn: &TextRegion) -> Option<Text> {
    let (x, y) = (drawn.x + drawn.width / 2.0, drawn.y + drawn.height / 2.0);
    let mut candidates: Vec<(f64, Text)> = vec![];
    let mut add = |score: f64, mut text: Text| {
        text.bounds = (
            drawn.x,
            drawn.y,
            drawn.x + drawn.width,
            drawn.y + drawn.height,
        );
        // Only offer source-backed edits supported by the existing semantic editor.
        if score.is_finite() && text.edits(source, &text.value).is_ok() {
            candidates.push((score, text));
        }
    };
    for node in &preview.nodes {
        let rect = (node.x, node.y, node.x + node.width, node.y + node.height);
        let score = distance(rect, x, y);
        let Some(decl) = preview.diagram.nodes.iter().find(|n| n.id == node.id) else {
            continue;
        };
        let label = decl.label.as_deref().unwrap_or(&decl.id);
        if matches(label, &drawn.text)
            && !matches!(
                decl.kind,
                laml::syntax::NodeKind::Object | laml::syntax::NodeKind::Junction
            )
            && decl.templates.is_empty()
        {
            add(
                score,
                Text {
                    target: Target::Node(decl.id.clone()),
                    value: label.into(),
                    multiline: true,
                    line: decl.line,
                    bounds: rect,
                },
            );
        } else {
            for field in field_text(source, decl.line, Some(&decl.id), &drawn.text) {
                add(score, field);
            }
        }
        for port in &node.ports {
            if matches(&port.label, &drawn.text) {
                add(
                    (port.x - x).hypot(port.y - y),
                    Text {
                        target: Target::Port(port.id.clone()),
                        value: port.label.clone(),
                        multiline: false,
                        line: decl.line,
                        bounds: rect,
                    },
                );
            }
        }
    }
    for route in &preview.routes {
        let Some(relation) = preview
            .diagram
            .relations
            .iter()
            .find(|r| r.line == route.source_line)
        else {
            continue;
        };
        if let Some(label) = &route.label
            && relation
                .label
                .as_deref()
                .is_some_and(|v| matches(v, &drawn.text))
        {
            let score = (label.x + label.width / 2.0 - x).hypot(label.y + label.height / 2.0 - y);
            add(
                score,
                Text {
                    target: Target::Relation(relation.line),
                    value: relation.label.clone().unwrap(),
                    multiline: false,
                    line: relation.line,
                    bounds: (0.0, 0.0, 0.0, 0.0),
                },
            );
        }
    }
    for region in &preview.regions {
        if region.kind.starts_with("sequence-")
            || matches!(
                region.kind.as_str(),
                "lifeline" | "lifeline-end" | "destroyed-end" | "execution"
            )
        {
            continue;
        }
        if preview.notation == Notation::Bpmn
            && (region.kind == "node" || region.id.starts_with("flow:"))
        {
            continue;
        }
        let score = distance(
            (
                region.x,
                region.y,
                region.x + region.width,
                region.y + region.height,
            ),
            x,
            y,
        );
        for field in field_text(source, region.line, Some(&region.id), &drawn.text) {
            add(score, field);
        }
        // Unlabelled participants/pools/lanes receive a label, never an ID rename.
        if matches!(region.kind.as_str(), "participant" | "pool" | "lane")
            && matches(
                region.id.rsplit('.').next().unwrap_or(&region.id),
                &drawn.text,
            )
            && !edit::item_fields(source, region.line, Some(&region.id))
                .iter()
                .any(|f| f.name == "Label")
        {
            add(
                score,
                Text {
                    target: Target::Attribute {
                        line: region.line,
                        id: region.id.clone(),
                    },
                    value: drawn.text.clone(),
                    multiline: true,
                    line: region.line,
                    bounds: (0.0, 0.0, 0.0, 0.0),
                },
            );
        }
    }
    if preview.notation == Notation::Bpmn {
        bpmn_candidates(preview, source, drawn, &mut add);
    }
    // Multiple regions may describe the same source field (e.g. message path + label).
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
    candidates.dedup_by(|a, b| same_target(&a.1.target, &b.1.target));
    let (score, best) = candidates.first()?;
    if *score > 100.0
        || candidates
            .get(1)
            .is_some_and(|next| (next.0 - score).abs() < 1e-6)
    {
        return None;
    }
    Some(best.clone())
}
fn same_target(a: &Target, b: &Target) -> bool {
    match (a, b) {
        (Target::Node(a), Target::Node(b))
        | (Target::Port(a), Target::Port(b))
        | (Target::BpmnNode(a), Target::BpmnNode(b)) => a == b,
        (Target::Relation(a), Target::Relation(b))
        | (Target::SequenceGuard(a), Target::SequenceGuard(b))
        | (Target::BpmnFlow(a), Target::BpmnFlow(b)) => a == b,
        (Target::Field(a), Target::Field(b)) => a.range == b.range,
        _ => false,
    }
}
fn bpmn_candidates(
    preview: &Preview,
    source: &str,
    drawn: &TextRegion,
    add: &mut impl FnMut(f64, Text),
) {
    let Ok(doc) = laml::bpmn::parse(source) else {
        return;
    };
    let (x, y) = (drawn.x + drawn.width / 2.0, drawn.y + drawn.height / 2.0);
    for node in &doc.nodes {
        if !matches(&node.label, &drawn.text) {
            continue;
        }
        if let Some(r) = preview
            .regions
            .iter()
            .find(|r| r.kind == "node" && r.id == node.id)
        {
            add(
                distance((r.x, r.y, r.x + r.width, r.y + r.height), x, y),
                Text {
                    target: Target::BpmnNode(node.id.clone()),
                    value: node.label.clone(),
                    multiline: true,
                    line: node.line,
                    bounds: (0.0, 0.0, 0.0, 0.0),
                },
            );
        }
    }
    for (i, edge) in doc.edges.iter().enumerate() {
        let boundary = doc
            .nodes
            .iter()
            .find(|n| n.id == edge.from && n.kind == laml::bpmn::Kind::Boundary);
        let Some(label) = edge
            .label
            .as_ref()
            .or_else(|| boundary.map(|n| &n.label))
            .filter(|v| matches(v, &drawn.text))
        else {
            continue;
        };
        let target = if edge.label.is_some() {
            Target::BpmnFlow(i)
        } else {
            Target::BpmnNode(edge.from.clone())
        };
        if let Some(r) = preview
            .regions
            .iter()
            .find(|r| r.id == format!("flow:{i}") && r.path.is_empty())
        {
            add(
                distance((r.x, r.y, r.x + r.width, r.y + r.height), x, y),
                Text {
                    target,
                    value: label.clone(),
                    multiline: false,
                    line: edge.line,
                    bounds: (0.0, 0.0, 0.0, 0.0),
                },
            );
        }
    }
}
