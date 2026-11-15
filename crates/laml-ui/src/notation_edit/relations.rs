//! Change the semantic relationship, including its required notation keywords.
use super::*;
pub fn relationship_type(source: &str, line: usize, key: &str) -> Result<Vec<Edit>, String> {
    let ty = crate::notation::Notation::of(source);
    let relation = ty.relation(key).ok_or("Unsupported relationship type")?;
    crate::relation_edit::declaration(source, line)?;
    let span = crate::relation_edit::endpoint_spans(source, line)?;
    let range = SourceIndex::new(source)
        .line(line)
        .ok_or("Missing relation")?;
    let text = code(&source[range.clone()]);
    let mut edits = vec![Edit {
        range: span.operator,
        replacement: relation.operator.into(),
    }];
    for token in crate::relation_edit::tokens(text) {
        let value = &text[token.clone()];
        if value.starts_with("<<")
            || value.starts_with('«')
            || (ty == crate::notation::Notation::Activity
                && ["control", "object"].contains(&value)
                && range.start + token.end <= span.from.start)
        {
            edits.push(Edit {
                range: range.start + token.start..range.start + token.end,
                replacement: String::new(),
            });
        }
    }
    if !relation.prefix.is_empty() {
        edits.push(Edit {
            range: span.from.start..span.from.start,
            replacement: relation.prefix.into(),
        });
    }
    if !relation.stereotype.is_empty() {
        edits.push(Edit {
            range: span.to.end..span.to.end,
            replacement: format!(" <<{}>>", relation.stereotype),
        });
    }
    let candidate = apply(source, &edits)?;
    crate::notation::parse_graph(&candidate).map_err(|e| e.message)?;
    Ok(crate::element_edit::merge_insertions(edits))
}
