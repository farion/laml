//! Inline part decompositions live in the same source; no file loading.

use super::{Event, SequenceDiagram, error};
use crate::lex::{quoted_string, valid_identifier};
use crate::parser::ParseError;

#[derive(Clone, Debug, PartialEq)]
pub struct Decomposition {
    pub parent: String,
    pub title: String,
    pub strict: bool,
    pub diagram: Box<SequenceDiagram>,
    pub line: usize,
}

struct Builder {
    parent: String,
    title: String,
    strict: bool,
    line: usize,
    depth: usize,
    lines: Vec<String>,
}

#[derive(Default)]
pub(super) struct Collector(Option<Builder>);

impl Collector {
    pub(super) fn is_open(&self) -> bool {
        self.0.is_some()
    }

    pub(super) fn consume(
        &mut self,
        text: &str,
        line: usize,
        diagram: &mut SequenceDiagram,
    ) -> Result<bool, ParseError> {
        if let Some(builder) = &mut self.0 {
            if text == "}" {
                builder.depth -= 1;
                if builder.depth == 0 {
                    let builder = self.0.take().expect("open decomposition");
                    let source = format!(
                        "settings {{\ntype: sequence\nsemantic-check: {}\n}}\nsemantics {{\n{}\n}}\n",
                        if diagram.semantic_check_off {
                            "off"
                        } else {
                            "on"
                        },
                        builder.lines.join("\n")
                    );
                    let nested = super::document::parse_partial(&source).map_err(|mut err| {
                        err.line = builder.line.saturating_add(err.line).saturating_sub(5);
                        err.message =
                            format!("decomposition '{}': {}", builder.parent, err.message);
                        err
                    })?;
                    if nested.participants.is_empty() {
                        return Err(error(
                            builder.line,
                            "decomposition needs at least one inner lifeline",
                        ));
                    }
                    let index = diagram.decompositions.len();
                    diagram.decompositions.push(Decomposition {
                        parent: builder.parent,
                        title: builder.title,
                        strict: builder.strict,
                        diagram: Box::new(nested),
                        line: builder.line,
                    });
                    diagram.events.push(Event::Decomposition {
                        index,
                        line: builder.line,
                    });
                    return Ok(true);
                }
            } else if text.ends_with('{') {
                builder.depth += 1;
            }
            builder.lines.push(text.into());
            return Ok(true);
        }
        let Some(body) = text.strip_prefix("decompose ") else {
            return Ok(false);
        };
        let body = body
            .strip_suffix('{')
            .ok_or_else(|| error(line, "decomposition needs '{'"))?
            .trim();
        let (parent, title) = body
            .split_once(' ')
            .ok_or_else(|| error(line, "expected decompose Parent \"Interaction\" {"))?;
        if !valid_identifier(parent) {
            return Err(error(line, "invalid decomposed participant"));
        }
        if diagram.decompositions.iter().any(|d| d.parent == parent) {
            return Err(error(
                line,
                format!("duplicate decomposition for '{parent}'"),
            ));
        }
        let (title, strict) = match title.trim().strip_suffix(" strict") {
            Some(title) => (title, true),
            None => (title.trim(), false),
        };
        let title = quoted_string(title).map_err(|msg| error(line, msg))?;
        if title.is_empty() {
            return Err(error(line, "decomposition title cannot be empty"));
        }
        self.0 = Some(Builder {
            parent: parent.into(),
            title,
            strict,
            line,
            depth: 1,
            lines: Vec::new(),
        });
        Ok(true)
    }
}

pub(super) fn validate(diagram: &SequenceDiagram) -> Result<(), ParseError> {
    for d in &diagram.decompositions {
        let parent = diagram
            .participants
            .iter()
            .find(|p| p.id == d.parent)
            .ok_or_else(|| {
                error(
                    d.line,
                    format!("unknown decomposed participant '{}'", d.parent),
                )
            })?;
        if parent.created_line.is_some_and(|created| d.line < created)
            || parent
                .destroyed_line
                .is_some_and(|destroyed| d.line >= destroyed)
        {
            return Err(error(
                d.line,
                format!("decomposition of '{}' outside its lifetime", d.parent),
            ));
        }
    }
    Ok(())
}
