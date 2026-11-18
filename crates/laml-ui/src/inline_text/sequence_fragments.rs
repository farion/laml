//! Fragment guards are rendered with brackets, but edits target only the source text.
use super::*;
fn field(source: &str, line: usize) -> Option<edit::fields::Field> {
    let fields = edit::item_fields(source, line, None);
    fields
        .iter()
        .find(|f| f.name == "Guard")
        .cloned()
        .or_else(|| {
            fields.into_iter().find(|f| {
                matches!(f.name.as_str(), "Condition" | "Text")
                    && f.kind == edit::fields::Kind::Quoted
            })
        })
}
pub(super) fn hit(preview: &Preview, source: &str, x: f64, y: f64) -> Option<Text> {
    if preview.notation != Notation::Sequence {
        return None;
    }
    let r = preview
        .regions
        .iter()
        .filter(|r| matches!(r.kind.as_str(), "sequence-guard" | "sequence-guard-header"))
        .filter(|r| {
            x >= r.x - 2.0
                && x <= r.x + r.width + 2.0
                && y >= r.y - 2.0
                && y <= r.y + r.height + 2.0
        })
        .min_by(|a, b| (a.width * a.height).total_cmp(&(b.width * b.height)))?;
    Some(Text {
        target: Target::SequenceGuard(r.line),
        value: field(source, r.line).map_or(String::new(), |f| f.value),
        multiline: false,
        line: r.line,
        bounds: (r.x, r.y, r.x + r.width, r.y + r.height),
    })
}
pub(super) fn edits(source: &str, line: usize, value: &str) -> Result<Vec<Edit>, String> {
    if let Some(field) = field(source, line) {
        return edit::fields::edit(source, &field, value);
    }
    let range = crate::source::SourceIndex::new(source)
        .line(line)
        .ok_or("Missing fragment header")?;
    let text = crate::style_edit::code(&source[range.clone()]);
    let tokens = edit::fields::tokens(text);
    let raw = tokens.iter().map(|r| &text[r.clone()]).collect::<Vec<_>>();
    let quoted = crate::relation_edit::quote(value);
    let (at, replacement) = if raw.windows(2).any(|pair| pair == ["loop", "("]) {
        let close = raw
            .iter()
            .rposition(|s| *s == ")")
            .ok_or("Missing loop attributes")?;
        let separator = if close > 0 && matches!(raw[close - 1], "(" | ";") {
            ""
        } else {
            "; "
        };
        (tokens[close].start, format!("{separator}guard={quoted}"))
    } else {
        let brace = raw
            .iter()
            .rposition(|s| *s == "{")
            .ok_or("Missing fragment body")?;
        (tokens[brace].start, format!("{quoted} "))
    };
    Ok(vec![Edit {
        range: range.start + at..range.start + at,
        replacement,
    }])
}
