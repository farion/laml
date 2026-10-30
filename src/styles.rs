//! LSS validation, safe local imports, and deterministic per-property cascade.

use crate::parser::parse_stylesheet;
mod selectors;
use crate::syntax::{Diagram, Property, StyleEntry};
pub use selectors::{Element, Selector};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Color(String), // canonical #rrggbb or "none"
}

#[derive(Clone, Debug, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub value: Value,
    pub line: usize,
    pub path: PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub selector: Selector,
    pub declarations: Vec<Declaration>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleSheet {
    pub rules: Vec<Rule>,
    ancestors: Vec<String>,
}

impl StyleSheet {
    pub(crate) fn with_ancestor(&self, id: &str, classes: &[String]) -> Self {
        let mut scoped = self.clone();
        scoped.ancestors.push(id.into());
        scoped
            .ancestors
            .extend(classes.iter().map(|class| format!(".{class}:{id}")));
        scoped
    }

    /// Return a winning declaration; object-specific beats class-specific,
    /// equally specific later declarations override earlier ones.
    pub fn get<'a>(&'a self, element: &Element<'_>, property: &str) -> Option<&'a Declaration> {
        let mut winner = None;
        let mut specificity = 0;
        for rule in &self.rules {
            if !rule.selector.matches(element, &self.ancestors) {
                continue;
            }
            let current = rule.selector.specificity();
            if current < specificity {
                continue;
            }
            for declaration in &rule.declarations {
                if declaration.name == property && applicable(property, element) {
                    winner = Some(declaration);
                    specificity = current;
                }
            }
        }
        winner
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyleError {
    pub path: PathBuf,
    pub line: usize,
    pub message: String,
}

fn err(path: &Path, line: usize, message: impl Into<String>) -> StyleError {
    StyleError {
        path: path.into(),
        line,
        message: message.into(),
    }
}

/// Load styles in source order. Imports must remain inside the source file's
/// directory tree (including after following symlinks); no remote imports.
pub fn load(diagram: &Diagram, source_path: &Path) -> Result<StyleSheet, StyleError> {
    let root = source_path.parent().unwrap_or_else(|| Path::new("."));
    let root = fs::canonicalize(root).map_err(|e| err(source_path, 0, e.to_string()))?;
    let mut result = StyleSheet::default();
    let mut stack = Vec::new();
    expand(
        &diagram.styling,
        source_path,
        &root,
        &root,
        &mut stack,
        &mut result,
    )?;
    Ok(result)
}

fn expand(
    entries: &[StyleEntry],
    current: &Path,
    base: &Path,
    root: &Path,
    stack: &mut Vec<PathBuf>,
    sheet: &mut StyleSheet,
) -> Result<(), StyleError> {
    for entry in entries {
        match entry {
            StyleEntry::Import { path, line } => {
                let candidate = base.join(path);
                let canonical = fs::canonicalize(&candidate)
                    .map_err(|e| err(current, *line, format!("cannot import '{path}': {e}")))?;
                if !canonical.starts_with(root) {
                    return Err(err(
                        current,
                        *line,
                        "style import escapes diagram directory",
                    ));
                }
                if stack.contains(&canonical) {
                    return Err(err(
                        current,
                        *line,
                        format!("style import cycle through '{}'", canonical.display()),
                    ));
                }
                let contents = fs::read_to_string(&canonical)
                    .map_err(|e| err(current, *line, format!("cannot read style import: {e}")))?;
                let imported =
                    parse_stylesheet(&contents).map_err(|e| err(&canonical, e.line, e.message))?;
                stack.push(canonical.clone());
                expand(
                    &imported,
                    &canonical,
                    canonical.parent().unwrap_or(root),
                    root,
                    stack,
                    sheet,
                )?;
                stack.pop();
            }
            StyleEntry::Rule(rule) => {
                let mut declarations = Vec::new();
                for property in &rule.properties {
                    declarations.push(Declaration {
                        name: property.name.clone(),
                        value: validate_value(property, current)?,
                        line: property.line,
                        path: current.into(),
                    });
                }
                sheet.rules.push(Rule {
                    selector: Selector::parse(&rule.selector),
                    declarations,
                });
            }
        }
    }
    Ok(())
}

fn validate_value(property: &Property, path: &Path) -> Result<Value, StyleError> {
    let value = property.value.trim();
    match property.name.as_str() {
        "fill" | "color" | "border-color" => parse_color(value)
            .map(Value::Color)
            .map_err(|message| err(path, property.line, message)),
        "padding" | "min-width" | "min-height" | "width" | "height" | "font-size"
        | "border-width" | "corner-radius" => {
            let number = value
                .parse::<f64>()
                .map_err(|_| err(path, property.line, "expected an unadorned numeric value"))?;
            if !number.is_finite()
                || number < 0.0
                || (property.name == "font-size" && number == 0.0)
            {
                return Err(err(
                    path,
                    property.line,
                    "style size must be finite and nonnegative",
                ));
            }
            Ok(Value::Number(number))
        }
        _ => Err(err(
            path,
            property.line,
            format!("unknown style property '{}'", property.name),
        )),
    }
}

fn parse_color(input: &str) -> Result<String, &'static str> {
    if input == "none" {
        return Ok("none".into());
    }
    if let Some(rgb) = input.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let channels: Vec<_> = rgb.split(',').map(|s| s.trim().parse::<u8>()).collect();
        if let [Ok(r), Ok(g), Ok(b)] = channels.as_slice() {
            return Ok(format!("#{r:02x}{g:02x}{b:02x}"));
        }
    }
    let hex = input
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(input);
    if hex.len() == 7 && hex.starts_with('#') && hex[1..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(hex.to_ascii_lowercase());
    }
    Err("expected rgb(r,g,b), quoted #rrggbb, or none")
}

fn applicable(property: &str, element: &Element<'_>) -> bool {
    match element {
        Element::Node { .. } => true, // all currently known properties are node-applicable
        Element::Continuation { .. } | Element::Note { .. } => matches!(
            property,
            "fill" | "color" | "border-color" | "border-width" | "font-size"
        ),
        Element::Relation { .. } | Element::Lifeline { .. } | Element::Gate { .. } => {
            matches!(property, "color" | "width" | "font-size")
        }
        Element::Coregion { .. } | Element::Ordering => matches!(property, "color" | "width"),
        Element::Annotation { .. } => matches!(property, "fill" | "color" | "width" | "font-size"),
        Element::Activation { .. } => {
            matches!(
                property,
                "fill" | "border-color" | "border-width" | "width" | "font-size" | "color"
            )
        }
        Element::Fragment { .. } => matches!(
            property,
            "fill" | "border-color" | "border-width" | "color" | "font-size" | "width"
        ),
    }
}
