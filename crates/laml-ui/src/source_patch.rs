//! Source-preserving patches at compiler-recognized block boundaries.
use crate::{document::Edit, source::SourceIndex, style_edit::code};
use laml::syntax::{Diagram, StyleEntry};
use std::collections::BTreeSet;

pub(crate) fn insert_section(
    source: &str,
    diagram: &Diagram,
    section: &str,
    declaration: &str,
) -> Result<Edit, String> {
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let index = SourceIndex::new(source);
    let openings: BTreeSet<_> = diagram
        .nodes
        .iter()
        .map(|node| node.line)
        .chain(diagram.styling.iter().filter_map(|entry| match entry {
            StyleEntry::Rule(rule) => Some(rule.line),
            _ => None,
        }))
        .collect();
    let mut depth = 0;
    let mut inside = false;
    for (i, line) in source.lines().enumerate() {
        let text = code(line).trim();
        if depth == 0
            && matches!(
                text,
                "semantics {" | "settings {" | "layout {" | "styling {"
            )
        {
            inside = text == format!("{section} {{");
            depth = 1;
        } else if text == "}" {
            if inside && depth == 1 {
                let position = index.line(i + 1).ok_or("Missing section boundary")?.start;
                return Ok(Edit {
                    range: position..position,
                    replacement: format!("    {declaration}{newline}"),
                });
            }
            depth -= 1;
        } else if openings.contains(&(i + 1)) && text.ends_with('{') {
            depth += 1;
        }
    }
    if inside {
        return Err("Cannot locate section boundary".into());
    }
    let separator = if source.is_empty() || source.ends_with('\n') {
        ""
    } else {
        newline
    };
    Ok(Edit {
        range: source.len()..source.len(),
        replacement: format!(
            "{separator}{newline}{section} {{{newline}    {declaration}{newline}}}{newline}"
        ),
    })
}

pub(crate) fn replace_code(
    source: &str,
    line_number: usize,
    replacement: String,
) -> Result<Edit, String> {
    let range = SourceIndex::new(source)
        .line(line_number)
        .ok_or("Missing source line")?;
    let original = &source[range.clone()];
    let body = code(original);
    let start = range.start + body.len() - body.trim_start().len();
    let end = range.start + body.trim_end().len();
    Ok(Edit {
        range: start..end,
        replacement,
    })
}

pub(crate) fn remove_line(source: &str, line_number: usize) -> Result<Edit, String> {
    let index = SourceIndex::new(source);
    let range = index.line(line_number).ok_or("Missing source line")?;
    let original = &source[range.clone()];
    let body = code(original);
    if body.len() < original.len() {
        let indent: String = original
            .chars()
            .take_while(|ch| ch.is_whitespace())
            .collect();
        Ok(Edit {
            range,
            replacement: format!("{indent}{}", &original[body.len()..]),
        })
    } else {
        let end = index
            .line(line_number + 1)
            .map_or(source.len(), |next| next.start);
        Ok(Edit {
            range: range.start..end,
            replacement: String::new(),
        })
    }
}
