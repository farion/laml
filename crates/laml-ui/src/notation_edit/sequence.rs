//! Sequence-specific source actions preserve explicit event order and lifetimes.
use super::*;
pub fn place_annotation(source: &str, id: &str, target: &str, continuation: bool, right: bool) -> Result<Vec<Edit>, String> {
    if id.starts_with("line:") || target.starts_with("line:") { return Err("Name this construct before referencing it in layout".into()); }
    let operator = if continuation { "spans" } else if right { "right of" } else { "over" };
    let old = records(source).into_iter().find(|r| r.section == "layout" && fields::tokens(&r.heading).first().is_some_and(|span| &r.heading[span.clone()] == id));
    let declaration = format!("{id} {operator} {target}");
    let edits = if let Some(old) = old {
        let raw = &source[old.range.clone()]; let body = code(raw);
        let start = body.len() - body.trim_start().len();
        let suffix = body.find(':').map(|i| &body[i..body.trim_end().len()]).unwrap_or("");
        vec![Edit { range: old.range.start + start..old.range.start + body.trim_end().len(), replacement: format!("{declaration}{}{suffix}", if suffix.is_empty() { "" } else { " " }) }]
    } else { insert(source, "layout", None, None, &declaration)? };
    let candidate = apply(source, &edits)?; laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    Ok(edits)
}
pub fn assign_name(source: &str, line: usize, name: &str) -> Result<Vec<Edit>, String> {
    if !crate::graph_edit::identifier(name) { return Err("Use a simple identifier".into()); }
    let r = record(source, line)?;
    let raw = &source[r.range.clone()];
    let at = r.range.start + raw.len() - raw.trim_start().len();
    let edits = vec![Edit { range: at..at, replacement: format!("{name}: ") }];
    let candidate = apply(source, &edits)?; laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    Ok(edits)
}
