//! Instant, source-only authoring assistance shared by native and browser editors.
mod catalog;
mod context;
pub mod pairs;
pub mod snippet;
mod symbols;
#[cfg(test)]
mod tests;
use crate::notation::Notation;
use std::{collections::BTreeSet, ops::Range};
#[derive(Clone, Debug)]
pub struct Entry {
    pub label: String,
    pub detail: String,
    /// Uses the compiler recipe's {name} / {name:q} placeholder convention.
    pub template: String,
}
impl Entry {
    pub(super) fn new(label: &str, detail: &str, template: &str) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            template: template.into(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Completion {
    pub range: Range<usize>,
    pub entries: Vec<Entry>,
}
pub fn complete(source: &str, caret: usize, explicit: bool) -> Completion {
    let mut out = Completion {
        range: caret..caret,
        entries: vec![],
    };
    let ctx = context::context(source, caret);
    if ctx.opaque {
        return out;
    }
    let ty = Notation::of(source);
    let mut range = context::token(source, caret);
    if let Some(at) = ctx.quote_start {
        range.start = at + 1;
    }
    let mut prefix = &source[range.start..caret];
    // Operators form their own token, distinct from a declaration's hyphenated ID.
    if ctx.section == "semantics"
        && ctx.attributes.is_none()
        && (prefix.is_empty() || prefix.chars().all(|c| matches!(c, '-' | '.')))
    {
        let start = source[..caret]
            .char_indices()
            .rev()
            .take_while(|(_, c)| "<>-|.*o=~".contains(*c))
            .last()
            .map_or(caret, |(i, _)| i);
        if start < caret {
            range = start..caret;
            prefix = &source[start..caret];
        }
    }
    let before = source[..range.start]
        .rsplit('\n')
        .next()
        .unwrap_or("")
        .trim();
    let triggered = ctx.quoted
        || source[..caret].trim_end().ends_with([':', '='])
        || ctx.attributes.is_some() && source[..caret].trim_end().ends_with(['(', ';']);
    if !explicit && prefix.is_empty() && !triggered {
        return out;
    }
    let mut entries = if let Some((key, _)) = before
        .rsplit([';', '('])
        .next()
        .unwrap_or("")
        .rsplit_once('=')
    {
        let key = key.trim().rsplit([';', '(', ':', ' ']).next().unwrap_or("");
        catalog::values(ty, key, &ctx, source)
    } else if (ctx.section == "settings" || ctx.section == "styling") && before.contains(':') {
        catalog::values(
            ty,
            before.split(':').next().unwrap_or("").trim(),
            &ctx,
            source,
        )
    } else if ctx.section == "settings" {
        catalog::settings(ty)
    } else if ctx.section == "styling" {
        catalog::styles(&ctx)
    } else if let Some(at) = ctx.attributes {
        catalog::attributes(ty, &source[..at], source)
    } else if ctx.section == "layout" {
        catalog::layout(ty, before, source)
    } else if ctx.section == "semantics" {
        if !before.is_empty()
            && ty
                .relation_types()
                .iter()
                .any(|r| before.contains(r.operator))
        {
            symbols::references(source)
        } else if !before.is_empty() && !prefix.chars().next().is_some_and(char::is_alphanumeric) {
            ty.relation_types()
                .into_iter()
                .filter(|r| r.prefix.is_empty() && r.stereotype.is_empty())
                .map(|r| Entry::new(r.operator, r.title, r.operator))
                .collect()
        } else if before.is_empty() {
            let mut items = catalog::constructs(ty, &ctx, source);
            if !ctx
                .heading
                .split_whitespace()
                .any(|w| matches!(w, "class" | "interface" | "enum" | "datatype" | "object"))
            {
                items.extend(symbols::references(source));
            }
            items
        } else {
            let mut items = symbols::references(source);
            items.extend(
                ty.relation_types()
                    .into_iter()
                    .filter(|r| r.prefix.is_empty() && r.stereotype.is_empty())
                    .map(|r| Entry::new(r.operator, r.title, r.operator)),
            );
            items
        }
    } else {
        ["semantics", "layout", "styling", "settings"]
            .into_iter()
            .filter(|s| {
                !context::code(source)
                    .lines()
                    .any(|l| l.trim().starts_with(&format!("{s} {{")))
            })
            .map(|s| Entry::new(s, "Section", &format!("{s} {{\n    {{content}}\n}}")))
            .collect()
    };
    // Suppress duplicate scalar properties in the current block or attribute list,
    // including declarations following the caret.
    let block_start = ctx.attributes.map_or(ctx.block_start, |p| p + 1);
    let tail = &source[block_start..];
    let clean_tail = context::code(tail);
    let block_end = clean_tail
        .find(if ctx.attributes.is_some() { ')' } else { '}' })
        .unwrap_or(tail.len());
    let mut used = context::code(&tail[..block_end]);
    if range.start >= block_start && range.end <= block_start + block_end {
        used.replace_range(
            range.start - block_start..range.end - block_start,
            &" ".repeat(range.len()),
        );
    }
    let mut seen = BTreeSet::new();
    entries.retain(|e| {
        let matches = e.label.to_lowercase().starts_with(&prefix.to_lowercase())
            || e.template
                .split_whitespace()
                .next()
                .is_some_and(|s| s.to_lowercase().starts_with(&prefix.to_lowercase()));
        let key = e.label.trim_end_matches([':', '=']);
        let duplicate = e.label.ends_with([':', '='])
            && used.split(['\n', ';', '(']).any(|p| {
                p.trim_start()
                    .strip_prefix(key)
                    .is_some_and(|t| t.trim_start().starts_with([':', '=']))
            });
        matches && !duplicate && seen.insert(e.label.clone())
    });
    if ctx.quoted {
        for entry in &mut entries {
            entry.template = entry.template.trim_matches('"').to_owned();
            if !source[range.end..].starts_with('"') {
                entry.template.push('"');
            }
        }
    }
    entries.sort_by_key(|e| e.label.to_lowercase());
    out.range = range;
    out.entries = entries;
    out
}
