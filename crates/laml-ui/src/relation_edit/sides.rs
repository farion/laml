use super::*;
use crate::source_patch::{insert_section, remove_line};

/// Reset removes only side=, retaining end-label attributes and comments verbatim.
pub fn endpoint_side(
    source: &str,
    line: usize,
    end: EndPosition,
    side: Option<&str>,
) -> Result<Vec<Edit>, String> {
    endpoint_sides(source, &[line], end, side)
}

pub fn endpoint_sides(
    source: &str,
    lines: &[usize],
    end: EndPosition,
    side: Option<&str>,
) -> Result<Vec<Edit>, String> {
    if side.is_some_and(|s| !["left", "right", "top", "bottom"].contains(&s)) {
        return Err("Invalid endpoint side".into());
    }
    let mut diagram = laml::parser::parse(source).map_err(|e| e.message)?;
    let mut edits = Vec::new();
    let mut declarations = Vec::new();
    for line in lines
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
    {
        let (_, relation) = declaration(source, line)?;
        if side.is_none() && relation.name.is_none() {
            continue;
        }
        let (id, naming) = named(source, &diagram, &relation)?;
        if let Some(r) = diagram.relations.iter_mut().find(|r| r.line == line) {
            r.name = Some(id.clone());
        }
        edits.extend(naming);
        edits.extend(one_side(
            source,
            &diagram,
            &id,
            end,
            side,
            &mut declarations,
        )?);
    }
    if !declarations.is_empty() {
        let newline = if source.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        edits.push(insert_section(
            source,
            &diagram,
            "layout",
            &declarations.join(&format!("{newline}    ")),
        )?);
    }
    Ok(edits)
}
fn one_side(
    source: &str,
    diagram: &Diagram,
    id: &str,
    end: EndPosition,
    side: Option<&str>,
    declarations: &mut Vec<String>,
) -> Result<Vec<Edit>, String> {
    let mut edits = Vec::new();
    let rule = diagram
        .relation_end_layout
        .iter()
        .find(|r| r.name == id && r.end == end);
    if let Some(rule) = rule {
        let range = SourceIndex::new(source)
            .line(rule.line)
            .ok_or("Missing side rule")?;
        let text = code(&source[range.clone()]);
        let colon = text.find(':').ok_or("Missing rule separator")?;
        let attrs = &text[colon + 1..];
        let mut offset = colon + 1;
        let mut existing = None;
        for part in attrs.split(';') {
            if let Some((key, value)) = part.split_once('=')
                && key.trim() == "side"
            {
                let start = offset + part.find('=').unwrap() + 1;
                let value_start = start + value.len() - value.trim_start().len();
                existing = Some((
                    offset..offset + part.len(),
                    value_start..offset + part.trim_end().len(),
                ));
            }
            offset += part.len() + 1;
        }
        match (existing, side) {
            (Some((_, value)), Some(side)) => edits.push(Edit {
                range: range.start + value.start..range.start + value.end,
                replacement: side.into(),
            }),
            (Some((part, value)), None) => {
                if rule.role.is_none() && rule.multiplicity.is_none() {
                    edits.push(remove_line(source, rule.line)?);
                } else {
                    let removal = if text.as_bytes().get(part.end) == Some(&b';') {
                        part.start..part.end + 1
                    } else {
                        part.start - 1..value.end
                    };
                    edits.push(Edit {
                        range: range.start + removal.start..range.start + removal.end,
                        replacement: String::new(),
                    });
                }
            }
            (None, Some(side)) => {
                let at = range.start + text.trim_end().len();
                edits.push(Edit {
                    range: at..at,
                    replacement: format!("; side={side}"),
                });
            }
            (None, None) => {}
        }
    } else if let Some(side) = side {
        let end = if end == EndPosition::Source {
            "source"
        } else {
            "target"
        };
        declarations.push(format!("relation[{id}].{end} : side={side}"));
    }
    Ok(edits)
}
