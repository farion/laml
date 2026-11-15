//! Reorder lifelines through authored horizontal constraints, not event moves.
use super::*;
pub fn reorder(source: &str, owner: &str, before: Option<&str>) -> Result<Vec<Edit>, String> {
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    let current: Vec<_> = doc.participants.iter().map(|p| p.id.clone()).collect();
    if !current.iter().any(|id| id == owner) {
        return Err("Participant no longer exists".into());
    }
    let mut order: Vec<_> = current
        .iter()
        .filter(|id| id.as_str() != owner)
        .cloned()
        .collect();
    let index = match before {
        Some(id) => order
            .iter()
            .position(|s| s == id)
            .ok_or("Choose another participant")?,
        None => order.len(),
    };
    order.insert(index, owner.into());
    if order == current {
        return Ok(vec![]);
    }
    let at = |id: &str| order.iter().position(|s| s == id).unwrap_or(usize::MAX);
    let mut edits = Vec::new();
    let mut existing = Vec::new();
    for rule in &doc.layout {
        if at(&rule.left) > at(&rule.right) {
            let r = record(source, rule.line)?;
            let body = code(&source[r.range.clone()]);
            let start = r.range.start + body.len() - body.trim_start().len();
            let end = r.range.start + body.find(':').unwrap_or(body.trim_end().len());
            // Retain authored gap attributes and trailing comments.
            let space = if body.contains(':') { " " } else { "" };
            edits.push(Edit {
                range: start..end,
                replacement: format!("{} < {}{space}", rule.right, rule.left),
            });
            existing.push((rule.right.clone(), rule.left.clone()));
        } else {
            existing.push((rule.left.clone(), rule.right.clone()));
        }
    }
    let rules = order
        .windows(2)
        .filter(|pair| !existing.contains(&(pair[0].clone(), pair[1].clone())))
        .map(|pair| format!("{} < {}", pair[0], pair[1]))
        .collect::<Vec<_>>();
    if !rules.is_empty() {
        edits.extend(insert(source, "layout", None, None, &rules.join("\n"))?);
    }
    let candidate = apply(source, &edits)?;
    let result = laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    if result.participants.iter().map(|p| &p.id).ne(order.iter()) {
        return Err("Conflicting participant order".into());
    }
    Ok(edits)
}
