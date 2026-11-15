//! Named bars have participant-local identity rather than a diagram-global ID.
use super::*;
use laml::sequence::{Event, SequenceDiagram};
fn owner(doc: &SequenceDiagram, target: usize, offset: isize, prefix: &str) -> Option<(String, String, Vec<usize>)> {
    for event in &doc.events {
        if let Event::Activate { participant, id: Some(id), line, .. } = event {
            if line.saturating_add_signed(offset) == target {
                let lines = doc.events.iter().filter_map(|event| match event {
                    Event::Activate { participant: p, id: Some(name), line, .. } | Event::Deactivate { participant: p, id: Some(name), line }
                        if p == participant && name == id => Some(line.saturating_add_signed(offset)), _ => None,
                }).collect();
                return Some((format!("{prefix}{participant}"), id.clone(), lines));
            }
        }
    }
    for child in &doc.decompositions {
        if let Some(result) = owner(&child.diagram, target, offset + child.line as isize - 5, &format!("{prefix}{}.", child.parent)) { return Some(result); }
    }
    None
}
pub(super) fn rename_field(source: &str, field: &fields::Field, new: &str) -> Option<Result<Vec<Edit>, String>> {
    if field.name != "Name / reference" || crate::notation::Notation::of(source) != crate::notation::Notation::Sequence { return None; }
    let doc = laml::sequence::parse(source).ok()?;
    let r = records(source).into_iter().find(|r| r.range.contains(&field.range.start))?;
    let (participant, old, lines) = owner(&doc, r.line, 0, "")?;
    if old != field.value { return None; }
    Some(rename(source, &participant, &old, &lines, new))
}
fn rename(source: &str, participant: &str, old: &str, lines: &[usize], new: &str) -> Result<Vec<Edit>, String> {
    if !crate::graph_edit::identifier(new) { return Err("Use a simple execution ID".into()); }
    let mut edits = Vec::new();
    for r in records(source) {
        let text = code(&source[r.range.clone()]); let ts = fields::tokens(text);
        let raw: Vec<_> = ts.iter().map(|s| &text[s.clone()]).collect();
        if lines.contains(&r.line) && raw.first() == Some(&old) && raw.get(1) == Some(&":") {
            edits.push(Edit { range: r.range.start + ts[0].start..r.range.start + ts[0].end, replacement: new.into() });
        }
        let scope = super::references::scope(source, &r, crate::notation::Notation::Sequence);
        let matches = |local: &str| local == participant || (!scope.is_empty() && format!("{scope}.{local}") == participant);
        if r.section == "layout" || r.heading.starts_with("at[") || r.heading.starts_with("between[") || r.heading.starts_with("order[") {
            for i in 0..raw.len().saturating_sub(5) {
                if raw[i] == "execution" && raw[i + 1] == "(" && matches(raw[i + 2]) && raw[i + 3] == "," && raw[i + 4] == old && raw[i + 5] == ")" {
                    edits.push(Edit { range: r.range.start + ts[i + 4].start..r.range.start + ts[i + 4].end, replacement: new.into() });
                }
            }
        }
        if r.section == "styling" && r.opens {
            for i in 0..raw.len().saturating_sub(6) {
                if raw[i] == "node" && raw[i + 1] == "[" && matches(raw[i + 2]) && raw[i + 3] == "]" && raw[i + 4] == ".activation" && raw[i + 5] == "[" && raw[i + 6] == old {
                    edits.push(Edit { range: r.range.start + ts[i + 6].start..r.range.start + ts[i + 6].end, replacement: new.into() });
                }
            }
        }
    }
    let candidate = apply(source, &edits)?;
    laml::sequence::parse(&candidate).map_err(|e| e.message)?; Ok(edits)
}
