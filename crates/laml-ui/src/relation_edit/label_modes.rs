use super::*;
use crate::{
    element_edit::newline,
    source_patch::{insert_section, remove_line},
};

/// Per-relation label placement reuses the delivered layout syntax.
pub fn label_modes(source: &str, lines: &[usize], mode: Option<&str>) -> Result<Vec<Edit>, String> {
    if mode.is_some_and(|m| !["near", "along", "inline"].contains(&m)) {
        return Err("Unknown relation label mode".into());
    }
    let mut diagram = crate::notation::parse_graph(source).map_err(|e| e.message)?;
    let mut edits = Vec::new();
    let mut declarations = Vec::new();
    for line in lines
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
    {
        let (_, relation) = declaration(source, line)?;
        if relation.name.is_none() && mode.is_none() {
            continue;
        }
        let (id, naming) = named(source, &diagram, &relation)?;
        if let Some(r) = diagram.relations.iter_mut().find(|r| r.line == line) {
            r.name = Some(id.clone());
        }
        edits.extend(naming);
        let rule = diagram.relation_layout.iter().find(|r| r.name == id);
        match (rule, mode) {
            (Some(rule), None) => edits.push(remove_line(source, rule.line)?),
            (Some(rule), Some(mode)) => {
                let range = SourceIndex::new(source)
                    .line(rule.line)
                    .ok_or("Missing label rule")?;
                let text = code(&source[range.clone()]);
                let equal = text.find('=').ok_or("Missing label mode")?;
                let tail = &text[equal + 1..];
                let start = range.start + equal + 1 + tail.len() - tail.trim_start().len();
                edits.push(Edit {
                    range: start..range.start + text.trim_end().len(),
                    replacement: mode.into(),
                });
            }
            (None, Some(mode)) => declarations.push(format!("relation[{id}] : label={mode}")),
            (None, None) => {}
        }
    }
    if !declarations.is_empty() {
        edits.push(insert_section(
            source,
            &diagram,
            "layout",
            &declarations.join(&format!("{}    ", newline(source))),
        )?);
    }
    Ok(edits)
}
