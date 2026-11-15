//! Minimal, validated patches to the document's settings block.
use crate::{document::Edit, source::SourceIndex, style_edit::code};
use laml::syntax::{FlowDirection, LabelPlacement, Settings};

pub fn value(settings: &Settings, name: &str) -> String {
    match name {
        "direction" => match settings.direction {
            FlowDirection::LeftToRight => "left-to-right",
            FlowDirection::RightToLeft => "right-to-left",
            FlowDirection::TopToBottom => "top-to-bottom",
            FlowDirection::BottomToTop => "bottom-to-top",
        }
        .into(),
        "relation-label" => match settings.relation_label {
            LabelPlacement::Near => "near",
            LabelPlacement::Along => "along",
            LabelPlacement::Inline => "inline",
        }
        .into(),
        "allow-diagonal" => settings.allow_diagonal.to_string(),
        "naming-convention" => if settings.strict_case {
            "uppercase-nodes-lowercase-relations"
        } else {
            "default"
        }
        .into(),
        "theme" => match settings.theme {
            laml::theme::Theme::Modern => "modern",
            laml::theme::Theme::Plain => "plain",
            laml::theme::Theme::ModernMono => "modern-mono",
            laml::theme::Theme::ModernDark => "modern-dark",
            laml::theme::Theme::PlainDark => "plain-dark",
            laml::theme::Theme::ModernMonoDark => "modern-mono-dark",
        }
        .into(),
        "background" => if settings.transparent {
            "transparent"
        } else {
            "opaque"
        }
        .into(),
        "font-file" => settings.font_file.clone().unwrap_or_default(),
        "font-italic-file" => settings.font_italic_file.clone().unwrap_or_default(),
        _ => String::new(),
    }
}

pub fn property(source: &str, name: &str, value: Option<&str>) -> Result<Vec<Edit>, String> {
    if ![
        "direction",
        "relation-label",
        "allow-diagonal",
        "naming-convention",
        "theme",
        "background",
        "font-file",
        "font-italic-file",
    ]
    .contains(&name)
    {
        return Err("Unknown document setting".into());
    }
    crate::notation::parse_graph(source).map_err(|e| e.message)?;
    if value.is_some_and(|v| v.contains(['\r', '\n'])) {
        return Err("Enter a single setting value".into());
    }
    let value = value
        .filter(|v| !v.is_empty() && !(name == "naming-convention" && *v == "default"))
        .map(|v| {
            if matches!(name, "font-file" | "font-italic-file") {
                crate::relation_edit::quote(v)
            } else {
                v.into()
            }
        });
    let index = SourceIndex::new(source);
    let lines: Vec<_> = source.lines().collect();
    let eol = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut depth = 0;
    let mut opening = None;
    for (i, line) in lines.iter().enumerate() {
        let text = code(line).trim();
        if depth == 0 && text == "settings {" {
            opening = Some(i);
            break;
        }
        if text.ends_with('{') {
            depth += 1;
        }
        if text == "}" {
            depth -= 1;
        }
    }
    let edit = if let Some(opening) = opening {
        let closing = (opening + 1..lines.len())
            .find(|i| code(lines[*i]).trim() == "}")
            .ok_or("Settings block has no closing brace")?;
        let existing = (opening + 1..closing).find(|i| {
            code(lines[*i])
                .split_once(':')
                .is_some_and(|(key, _)| key.trim() == name)
        });
        if let Some(i) = existing {
            let line = lines[i];
            let text = code(line);
            let range = index.line(i + 1).ok_or("Setting line unavailable")?;
            if let Some(value) = value {
                let colon = text.find(':').ok_or("Setting separator unavailable")?;
                let rest = &text[colon + 1..];
                let start = colon + 1 + rest.len() - rest.trim_start().len();
                Edit {
                    range: range.start + start..range.start + text.trim_end().len(),
                    replacement: value,
                }
            } else if text.len() != line.len() {
                Edit {
                    range: range.start..range.start + text.len(),
                    replacement: line.chars().take_while(|c| c.is_whitespace()).collect(),
                }
            } else {
                Edit {
                    range: range.start..index.line(i + 2).map_or(range.end, |r| r.start),
                    replacement: String::new(),
                }
            }
        } else if let Some(value) = value {
            let indent = (opening + 1..closing)
                .find_map(|i| {
                    (!code(lines[i]).trim().is_empty()).then(|| {
                        lines[i]
                            .chars()
                            .take_while(|c| c.is_whitespace())
                            .collect::<String>()
                    })
                })
                .unwrap_or_else(|| "    ".into());
            let at = index
                .line(closing + 1)
                .ok_or("Settings boundary unavailable")?
                .start;
            Edit {
                range: at..at,
                replacement: format!("{indent}{name}: {value}{eol}"),
            }
        } else {
            return Ok(vec![]);
        }
    } else if let Some(value) = value {
        Edit {
            range: 0..0,
            replacement: format!("settings {{{eol}    {name}: {value}{eol}}}{eol}{eol}"),
        }
    } else {
        return Ok(vec![]);
    };
    let candidate = crate::node_edit::candidate(source, std::slice::from_ref(&edit))?;
    crate::notation::parse_graph(&candidate).map_err(|e| e.message)?;
    Ok(vec![edit])
}
