//! Optional semantic attributes and notation-specific local styling.
use super::*;

pub fn item_fields(source: &str, line: usize, id: Option<&str>) -> Vec<fields::Field> {
    let mut fields = fields::of(source, line);
    if crate::notation::Notation::of(source) != crate::notation::Notation::Bpmn {
        return fields;
    }
    let Some(id) = id.filter(|id| !id.starts_with("flow:")) else {
        return fields;
    };
    let Some(range) = SourceIndex::new(source).line(line) else {
        return fields;
    };
    let text = code(&source[range.clone()]);
    let short = id.rsplit('.').next().unwrap();
    if let Some(span) = crate::relation_edit::tokens(text).into_iter().find(|r| {
        let token = text[r.clone()].trim_start_matches('+');
        token == short
            || token.starts_with(&format!("{short}("))
            || token.starts_with(&format!("<X>@{short}"))
            || token.starts_with(&format!("<+>@{short}"))
            || token.starts_with(&format!("<O>@{short}"))
            || token.starts_with(&format!("<event>@{short}"))
    }) {
        fields.retain(|f| {
            f.range.start >= range.start + span.start && f.range.end <= range.start + span.end
        });
    }
    fields
}

pub fn attribute(
    source: &str,
    line: usize,
    id: Option<&str>,
    name: &str,
    value: &str,
) -> Result<Vec<Edit>, String> {
    if !crate::graph_edit::identifier(name) && name != "style-classes" {
        return Err("Invalid attribute name".into());
    }
    let encoded = if name == "style-classes" {
        value.to_owned()
    } else {
        crate::relation_edit::quote(value)
    };
    let range = SourceIndex::new(source)
        .line(line)
        .ok_or("Missing declaration")?;
    let text = code(&source[range.clone()]);
    let ts = fields::tokens(text);
    let selected = id.map(|s| s.rsplit('.').next().unwrap());
    let name_token = if let Some(id) = selected {
        ts.iter()
            .position(|r| text[r.clone()].trim_start_matches(['+', '@']) == id)
    } else {
        None
    }
    .or_else(|| {
        ts.iter()
            .position(|r| &text[r.clone()] == "(")
            .and_then(|i| i.checked_sub(1))
    })
    .or_else(|| {
        ts.iter().position(|r| {
            let t = &text[r.clone()];
            crate::graph_edit::identifier(t)
                && ![
                    "person",
                    "external",
                    "system",
                    "container",
                    "component",
                    "class",
                    "object",
                    "state",
                    "note",
                ]
                .contains(&t)
        })
    })
    .ok_or("Select an element declaration")?;
    let name_end = ts[name_token].end;
    let open = ts
        .get(name_token + 1)
        .filter(|r| &text[(*r).clone()] == "(");
    if let Some(open) = open {
        let close = ts
            .iter()
            .find(|r| r.start > open.start && &text[(*r).clone()] == ")")
            .ok_or("Missing attribute closing parenthesis")?;
        for pair in ts
            .windows(3)
            .filter(|p| p[0].start > open.start && p[2].end <= close.start)
        {
            if &text[pair[0].clone()] == name && &text[pair[1].clone()] == "=" {
                return Ok(vec![Edit {
                    range: range.start + pair[2].start..range.start + pair[2].end,
                    replacement: encoded.clone(),
                }]);
            }
        }
        let separator = if text[open.end..close.start].trim().is_empty() {
            ""
        } else {
            "; "
        };
        return Ok(vec![Edit {
            range: range.start + close.start..range.start + close.start,
            replacement: format!("{separator}{name}={}", encoded),
        }]);
    }
    Ok(vec![Edit {
        range: range.start + name_end..range.start + name_end,
        replacement: format!("({name}={})", encoded),
    }])
}

pub fn style(
    source: &str,
    selector: &str,
    property: &str,
    value: Option<&str>,
) -> Result<Vec<Edit>, String> {
    if !property
        .chars()
        .all(|c| c.is_ascii_alphabetic() || c == '-')
    {
        return Err("Invalid style property".into());
    }
    if let Some(value) = value {
        laml::styles::property_value(property, value)?;
    }
    let all = records(source);
    let rules: Vec<_> = all
        .iter()
        .filter(|r| r.section == "styling" && r.opens && r.heading == selector)
        .collect();
    let matches: Vec<_> = all
        .iter()
        .filter(|r| {
            rules.iter().any(|p| r.parent == Some(p.line))
                && r.heading
                    .split_once(':')
                    .is_some_and(|(p, _)| p.trim() == property)
        })
        .collect();
    if let Some(last) = matches.last() {
        let mut edits = Vec::new();
        for r in &matches {
            if Some(r.line) != value.map(|_| last.line) {
                edits.push(Edit {
                    range: r.block.clone(),
                    replacement: String::new(),
                });
            }
        }
        if let Some(value) = value {
            let raw = &source[last.range.clone()];
            let start = raw.find(':').ok_or("Missing style property separator")? + 1;
            let body = code(raw);
            let begin =
                start + raw[start..body.len()].len() - raw[start..body.len()].trim_start().len();
            edits.push(Edit {
                range: last.range.start + begin..last.range.start + body.trim_end().len(),
                replacement: value.into(),
            });
        }
        return Ok(edits);
    }
    let Some(value) = value else {
        return Ok(vec![]);
    };
    if let Some(rule) = rules.last() {
        insert(
            source,
            "styling",
            Some(rule.line),
            None,
            &format!("{property}: {value}"),
        )
    } else {
        insert(
            source,
            "styling",
            None,
            None,
            &format!("{selector} {{\n    {property}: {value}\n}}"),
        )
    }
}

pub fn setting(source: &str, name: &str, value: Option<&str>) -> Result<Vec<Edit>, String> {
    if name == "type" {
        return Err("Change the diagram type in the source".into());
    }
    if value.is_some_and(|s| s.contains(['\n', '\r', '{', '}'])) {
        return Err("Enter one setting value".into());
    }
    let r = records(source).into_iter().find(|r| {
        r.section == "settings"
            && r.heading
                .split_once(':')
                .is_some_and(|(key, _)| key.trim() == name)
    });
    let edits = if let Some(r) = r {
        match value {
            None => vec![Edit {
                range: r.block,
                replacement: String::new(),
            }],
            Some(value) => {
                let raw = &source[r.range.clone()];
                let body = code(raw);
                let colon = body.find(':').ok_or("Missing setting separator")? + 1;
                let start = colon + body[colon..].len() - body[colon..].trim_start().len();
                vec![Edit {
                    range: r.range.start + start..r.range.start + body.trim_end().len(),
                    replacement: value.into(),
                }]
            }
        }
    } else if let Some(value) = value {
        insert(source, "settings", None, None, &format!("{name}: {value}"))?
    } else {
        vec![]
    };
    let candidate = apply(source, &edits)?;
    crate::notation::inspect(&candidate)?;
    Ok(edits)
}
