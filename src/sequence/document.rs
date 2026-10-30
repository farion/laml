//! Top-level sequence sections and cross-section validation.

use super::*;
use crate::lex::without_comment;

pub fn parse(source: &str) -> Result<SequenceDiagram, ParseError> {
    parse_inner(source, true)
}

pub(super) fn parse_partial(source: &str) -> Result<SequenceDiagram, ParseError> {
    parse_inner(source, false)
}

fn parse_inner(source: &str, complete: bool) -> Result<SequenceDiagram, ParseError> {
    let (source, styling) = styling::extract(source)?;
    let mut diagram = SequenceDiagram {
        styling,
        ..SequenceDiagram::default()
    };
    let mut section = "";
    let mut seen_type = false;
    let mut seen_semantic_setting = false;
    let mut seen_semantics = false;
    let mut seen_layout = false;
    let mut calls: Vec<(String, String)> = Vec::new();
    let mut active: Vec<(String, usize)> = Vec::new();
    let mut frames = fragments::Frames::default();
    let mut coregions = coregions::Collector::default();
    let mut decompositions = decompositions::Collector::default();
    let mut pending_layout = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        let text = without_comment(raw).trim();
        if text.is_empty() {
            continue;
        }
        if section.is_empty() {
            section = match text {
                "settings {" => "settings",
                "semantics {" if !seen_semantics => {
                    seen_semantics = true;
                    "semantics"
                }
                "semantics {" => return Err(error(line, "duplicate semantics section")),
                "layout {" if !seen_layout => {
                    seen_layout = true;
                    "layout"
                }
                "layout {" => return Err(error(line, "duplicate layout section")),
                _ => {
                    return Err(error(
                        line,
                        "expected settings, semantics or layout section",
                    ));
                }
            };
        } else if section == "semantics"
            && (decompositions.consume(text, line, &mut diagram)?
                || coregions.consume(text, line, &mut diagram)?
                || frames.consume(text, line, &mut diagram, &mut calls, &mut active)?)
        {
            continue;
        } else if text == "}" {
            section = "";
        } else if section == "settings" {
            if text == "type: sequence" && !seen_type {
                seen_type = true;
            } else if matches!(text, "semantic-check: on" | "semantic-check: off")
                && !seen_semantic_setting
            {
                seen_semantic_setting = true;
                diagram.semantic_check_off = text.ends_with("off");
            } else {
                return Err(error(line, "unknown or duplicate sequence setting"));
            }
        } else if section == "layout" {
            pending_layout.push((text.to_string(), line));
        } else {
            parse_semantic(
                text,
                line,
                &mut diagram,
                &mut calls,
                &mut active,
                frames.operand_scope(),
                !frames.is_empty(),
            )?;
        }
    }
    if !section.is_empty() || !frames.is_empty() || coregions.is_open() || decompositions.is_open()
    {
        return Err(error(
            source.lines().count().max(1),
            "unclosed section or fragment",
        ));
    }
    if !seen_type || !seen_semantics {
        return Err(error(
            1,
            "sequence diagram requires type: sequence and semantics",
        ));
    }
    for (text, line) in pending_layout {
        layout_scoped::add(&mut diagram, &text, line)?;
    }
    if complete {
        layout_scoped::validate(&mut diagram)?;
    }
    // An execution may remain open at the end of the drawn excerpt; an extra
    // close is always invalid. Missing replies are equally permissible.
    Ok(diagram)
}
