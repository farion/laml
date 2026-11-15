//! Minimal source patches for object-local node style properties.
use crate::{document::Edit, source::SourceIndex};
use laml::syntax::StyleEntry;
use std::collections::{BTreeMap, BTreeSet};

/// Match the compiler's comment boundary rules, including quoted URLs.
pub(crate) fn code(line: &str) -> &str {
    let mut quoted = false;
    let mut escaped = false;
    for (i, ch) in line.char_indices() {
        if escaped {
            escaped = false;
        } else if quoted && ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            quoted = !quoted;
        } else if !quoted
            && line[i..].starts_with("//")
            && (i == 0
                || line[..i]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace))
        {
            return &line[..i];
        }
    }
    line
}

fn closing_line(lines: &[&str], opening: usize) -> Result<usize, String> {
    let mut depth = 0usize;
    for (i, line) in lines.iter().enumerate().skip(opening) {
        let text = code(line).trim();
        if text.ends_with('{') {
            depth += 1;
        }
        if text == "}" {
            depth = depth.checked_sub(1).ok_or("Unmatched source brace")?;
            if depth == 0 {
                return Ok(i);
            }
        }
    }
    Err("Cannot locate style block boundary".into())
}

/// None removes direct local declarations; imports and class rules stay untouched.
pub fn node_property(
    source: &str,
    ids: &[String],
    name: &str,
    value: Option<&str>,
) -> Result<Vec<Edit>, String> {
    if !crate::properties::NODE_PROPERTIES.contains(&name) {
        return Err("Unsupported inspector property".into());
    }
    if value.is_some_and(|v| v.contains(['\r', '\n', '{', '}'])) {
        return Err("Expected a single property value".into());
    }
    let diagram = laml::parser::parse(source).map_err(|e| e.message)?;
    for id in ids {
        if !diagram.nodes.iter().any(|node| &node.id == id) {
            return Err(format!("Unknown node: {id}"));
        }
    }
    local_property(
        source,
        &ids.iter()
            .map(|id| format!("node[{id}]"))
            .collect::<Vec<_>>(),
        name,
        value,
    )
}

/// Apply dependent properties as one source transaction, preserving untouched bytes.
pub fn node_properties(
    source: &str,
    ids: &[String],
    properties: &[(String, Option<String>)],
) -> Result<Vec<Edit>, String> {
    let mut candidate = source.to_owned();
    for (name, value) in properties {
        let edits = node_property(&candidate, ids, name, value.as_deref())?;
        candidate = crate::node_edit::candidate(&candidate, &edits)?;
    }
    Ok(crate::graph_edit::difference(source, &candidate))
}

pub(crate) fn local_property(
    source: &str,
    selectors: &[String],
    name: &str,
    value: Option<&str>,
) -> Result<Vec<Edit>, String> {
    if value.is_some_and(|v| v.contains(['\r', '\n', '{', '}'])) {
        return Err("Expected a single property value".into());
    }
    let diagram = if source.trim().is_empty() {
        laml::syntax::Diagram::default()
    } else {
        laml::parser::parse(source).map_err(|e| e.message)?
    };
    let index = SourceIndex::new(source);
    let lines: Vec<_> = source.lines().collect();
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut edits = Vec::new();
    let mut insertions: BTreeMap<usize, String> = BTreeMap::new();
    let mut new_rules = String::new();
    for selector in selectors.iter().collect::<BTreeSet<_>>() {
        let rules: Vec<_> = diagram
            .styling
            .iter()
            .filter_map(|entry| match entry {
                StyleEntry::Rule(rule) if &rule.selector == selector => Some(rule),
                _ => None,
            })
            .collect();
        let properties: Vec<_> = rules
            .iter()
            .flat_map(|rule| &rule.properties)
            .filter(|property| property.name == name)
            .collect();
        if let Some(value) = value {
            if let Some(property) = properties.last() {
                let line = index.line(property.line).ok_or("Missing property line")?;
                let original = &source[line.clone()];
                let body = code(original);
                let colon = body.find(':').ok_or("Missing property separator")?;
                let tail = &body[colon + 1..];
                let start = line.start + colon + 1 + tail.len() - tail.trim_start().len();
                let end = line.start + body.trim_end().len();
                edits.push(Edit {
                    range: start..end,
                    replacement: value.into(),
                });
            } else if let Some(rule) = rules.last() {
                let closing = closing_line(&lines, rule.line - 1)?;
                let position = index.line(closing + 1).ok_or("Missing closing line")?.start;
                let indent: String = lines[rule.line - 1]
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .collect();
                insertions
                    .entry(position)
                    .or_default()
                    .push_str(&format!("{indent}    {name}: {value}{newline}"));
            } else {
                new_rules.push_str(&format!(
                    "    {selector} {{{newline}        {name}: {value}{newline}    }}{newline}"
                ));
            }
        } else {
            for property in properties {
                let line = index.line(property.line).ok_or("Missing property line")?;
                let original = &source[line.clone()];
                let body = code(original);
                // Keep a trailing comment and its indentation, or remove the complete line.
                let (range, replacement) = if body.len() < original.len() {
                    let indent: String =
                        original.chars().take_while(|c| c.is_whitespace()).collect();
                    (line.clone(), format!("{indent}{}", &original[body.len()..]))
                } else {
                    let end = index
                        .line(property.line + 1)
                        .map_or(source.len(), |next| next.start);
                    (line.start..end, String::new())
                };
                edits.push(Edit { range, replacement });
            }
        }
    }
    if !new_rules.is_empty() {
        let declaration = new_rules
            .trim_end_matches(['\r', '\n'])
            .strip_prefix("    ")
            .unwrap_or(&new_rules);
        let insertion =
            crate::source_patch::insert_section(source, &diagram, "styling", declaration)?;
        insertions
            .entry(insertion.range.start)
            .or_default()
            .push_str(&insertion.replacement);
    }
    edits.extend(insertions.into_iter().map(|(position, replacement)| Edit {
        range: position..position,
        replacement,
    }));
    Ok(edits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    fn apply(source: &str, ids: &[&str], value: Option<&str>) -> String {
        let edits = node_property(
            source,
            &ids.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            "fill",
            value,
        )
        .unwrap();
        let mut doc = Document::new(source.into());
        doc.apply(0, "Style", edits).unwrap();
        laml::parser::parse(doc.source()).unwrap();
        doc.source().into()
    }
    #[test]
    fn replaces_only_value_preserving_crlf_and_comment() {
        let source = "semantics {\r\n A\r\n}\r\nstyling {\r\n node[A] {\r\n  fill: #ffffff  // keep\r\n }\r\n}\r\n";
        assert_eq!(
            apply(source, &["A"], Some("#123456")),
            source.replace("#ffffff", "#123456")
        );
        let reset = apply(source, &["A"], None);
        assert!(reset.contains("  // keep\r\n"));
        assert!(!reset.contains("fill:"));
    }
    #[test]
    fn multi_selection_inserts_one_block_and_keeps_unrelated_text() {
        let source = "// untouched\nsemantics {\n A\n B\n}\n";
        let updated = apply(source, &["A", "B", "A"], Some("#112233"));
        assert!(updated.starts_with(source));
        assert_eq!(updated.matches("styling {").count(), 1);
        assert_eq!(updated.matches("fill:").count(), 2);
    }
    #[test]
    fn reset_removes_all_direct_overrides_but_not_class_rules() {
        let source = "semantics {\n A\n}\nstyling {\n node.red {\n fill: #ff0000\n }\n node[A] {\n fill: #ffffff\n }\n node[A] {\n fill: #000000\n }\n}\n";
        let updated = apply(source, &["A"], None);
        assert!(updated.contains("fill: #ff0000"));
        assert_eq!(updated.matches("fill:").count(), 1);
    }
    #[test]
    fn adds_to_existing_rule_or_empty_style_section() {
        for style in [
            "styling {\n}\n",
            "styling {\n node[A] {\n border-width: 2\n }\n}\n",
        ] {
            let source = format!("semantics {{\n A\n}}\n{style}");
            let updated = apply(&source, &["A"], Some("#123456"));
            assert_eq!(updated.matches("node[A]").count(), 1);
            assert_eq!(updated.matches("styling {").count(), 1);
        }
    }
    #[test]
    fn notes_with_literal_open_braces_do_not_hide_the_existing_style_section() {
        let source = "semantics {\n note N {\n example {\n }\n A\n}\nstyling {\n}\n";
        let updated = apply(source, &["A"], Some("#123456"));
        assert_eq!(updated.matches("styling {").count(), 1);
        assert!(updated.contains("example {\n }\n A"));
    }
}
