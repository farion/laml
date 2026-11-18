//! Project an inline decomposition for existing source transactions, then patch
//! only its body and qualify newly authored root layout references.
use super::*;
use std::ops::Range;
pub struct Scope {
    pub source: String,
    pub offset: isize,
    prefix: String,
    body: Range<usize>,
    layout: Vec<Record>,
}
impl Scope {
    pub fn of(source: &str, owner: &str) -> Result<Option<Self>, String> {
        let Some((prefix, _)) = owner.rsplit_once('.') else {
            return Ok(None);
        };
        let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
        fn find(
            doc: &laml::sequence::SequenceDiagram,
            prefix: &str,
            offset: isize,
        ) -> Option<usize> {
            let (head, tail) = prefix
                .split_once('.')
                .map_or((prefix, None), |(a, b)| (a, Some(b)));
            let child = doc.decompositions.iter().find(|d| d.parent == head)?;
            let line = child.line.saturating_add_signed(offset);
            if let Some(tail) = tail {
                find(&child.diagram, tail, line as isize - 5)
            } else {
                Some(line)
            }
        }
        let line = find(&doc, prefix, 0).ok_or("The inline decomposition no longer exists")?;
        let r = record(source, line)?;
        let index = SourceIndex::new(source);
        let start = index
            .line(line + 1)
            .ok_or("Missing decomposition body")?
            .start;
        let end_brace = r.block.start
            + source[r.block.clone()]
                .rfind('}')
                .ok_or("Missing decomposition end")?;
        let end = source[..end_brace].rfind('\n').map_or(0, |i| i + 1);
        let body = start..end;
        let nl = crate::element_edit::newline(source);
        let mut projected = format!(
            "settings {{{nl} type: sequence{nl} semantic-check: {}{nl}}}{nl}semantics {{{nl}{}}}{nl}",
            if doc.semantic_check_off { "off" } else { "on" },
            &source[body.clone()]
        );
        let layout = records(source)
            .into_iter()
            .filter(|r| {
                r.section == "layout"
                    && r.heading
                        .split_whitespace()
                        .next()
                        .is_some_and(|w| w.starts_with(&format!("{prefix}.")))
            })
            .collect::<Vec<_>>();
        if !layout.is_empty() {
            projected.push_str(&format!("layout {{{nl}"));
            for r in &layout {
                let raw = &source[r.range.clone()];
                let mut local = raw.to_string();
                let mut spans = super::references::spans(code(raw), "layout", false);
                spans.sort_by_key(|r| std::cmp::Reverse(r.start));
                for span in spans {
                    if let Some(value) = local[span.clone()].strip_prefix(&format!("{prefix}.")) {
                        let value = value.to_owned();
                        local.replace_range(span, &value);
                    }
                }
                projected.push_str(local.trim_end_matches(['\r', '\n']));
                projected.push_str(nl);
            }
            projected.push_str(&format!("}}{nl}"));
        }
        Ok(Some(Self {
            source: projected,
            offset: line as isize - 5,
            prefix: prefix.into(),
            body,
            layout,
        }))
    }
    pub fn line(&self, line: usize) -> usize {
        if line == 0 {
            0
        } else {
            line.saturating_add_signed(-self.offset)
        }
    }
    pub fn absolute(&self, line: usize) -> usize {
        if line == 0 {
            0
        } else {
            line.saturating_add_signed(self.offset)
        }
    }
    pub fn owner(&self, owner: &str) -> Result<String, String> {
        owner
            .strip_prefix(&format!("{}.", self.prefix))
            .filter(|id| !id.contains('.'))
            .map(str::to_owned)
            .ok_or_else(|| "Keep this operation inside the same inline decomposition".into())
    }
    pub fn dock(
        &self,
        dock: &super::sequence_controls::Dock,
    ) -> Result<super::sequence_controls::Dock, String> {
        use super::sequence_controls::{Boundary, Dock};
        Ok(Dock {
            owner: self.owner(&dock.owner)?,
            before: self.line(dock.before),
            boundary: dock.boundary.as_ref().map(|b| match *b {
                Boundary::Start(s) => Boundary::Start(self.line(s)),
                Boundary::End { start, end } => Boundary::End {
                    start: self.line(start),
                    end: end.map(|e| self.line(e)),
                },
            }),
        })
    }
    pub fn commit(&self, original: &str, edits: &[Edit]) -> Result<Vec<Edit>, String> {
        let candidate = apply(&self.source, edits)?;
        let mut depth = 1;
        let mut start = 0;
        let mut end = None;
        for (i, raw) in candidate.split_inclusive('\n').enumerate() {
            if i == 4 {
                start += raw.len();
                continue;
            }
            if i < 4 {
                start += raw.len();
                continue;
            }
            let text = code(raw).trim();
            if text == "}" {
                depth -= 1;
            } else if text.ends_with('{')
                && !fields::tokens(text)
                    .iter()
                    .any(|r| crate::relation_edit::OPERATORS.contains(&&text[r.clone()]))
            {
                depth += 1;
            }
            if depth == 0 {
                end = Some(start);
                break;
            }
            start += raw.len();
        }
        let begin = SourceIndex::new(&candidate)
            .line(6)
            .ok_or("Missing projected body")?
            .start;
        let end = end.ok_or("Missing projected interaction end")?;
        let mut result = vec![Edit {
            range: self.body.clone(),
            replacement: candidate[begin..end].into(),
        }];
        let mut qualify = Vec::new();
        for r in records(&candidate)
            .into_iter()
            .filter(|r| r.section == "layout")
        {
            let raw = &candidate[r.range.clone()];
            let tokens = fields::tokens(code(raw));
            for span in super::references::spans(code(raw), "layout", false) {
                let value = &raw[span.clone()];
                let index = tokens.iter().position(|t| t.start == span.start);
                if (index == Some(1)
                    && ["over", "above", "spans", "covers", "left", "right"].contains(&value))
                    || (index == Some(2) && value == "of")
                {
                    continue;
                }
                qualify.push(Edit {
                    range: r.range.start + span.start..r.range.start + span.end,
                    replacement: format!("{}.{value}", self.prefix),
                });
            }
        }
        let qualified = apply(&candidate, &qualify)?;
        let index = SourceIndex::new(&qualified);
        let layout_line = qualified
            .lines()
            .position(|line| code(line).trim() == "layout {")
            .map(|i| i + 1);
        if let Some(line) = layout_line {
            let begin = index
                .line(line + 1)
                .ok_or("Missing projected layout")?
                .start;
            let end = qualified
                .lines()
                .enumerate()
                .skip(line)
                .find(|(_, line)| code(line).trim() == "}")
                .and_then(|(i, _)| index.line(i + 1))
                .ok_or("Missing projected layout end")?
                .start;
            let layout = &qualified[begin..end];
            let old = self
                .layout
                .iter()
                .map(|r| &original[r.block.clone()])
                .collect::<String>();
            if layout != old {
                result.extend(self.layout.iter().map(|r| Edit {
                    range: r.block.clone(),
                    replacement: String::new(),
                }));
                if !layout.trim().is_empty() {
                    result.extend(insert(
                        original,
                        "layout",
                        None,
                        None,
                        layout.trim_end_matches(['\r', '\n']),
                    )?);
                }
            }
        }
        let result = crate::element_edit::merge_insertions(result);
        laml::sequence::parse(&apply(original, &result)?).map_err(|e| e.message)?;
        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_message_and_order_patch_the_body_and_qualify_layout() {
        let s = "settings {\n type: sequence\n}\nsemantics {\n Root\n decompose Root \"Inner\" {\n  A\n  B\n }\n}\n";
        let scope = Scope::of(s, "Root.A").unwrap().unwrap();
        let edits =
            super::super::sequence_participants::reorder(&scope.source, "B", Some("A")).unwrap();
        let result = apply(s, &scope.commit(s, &edits).unwrap()).unwrap();
        assert!(result.contains("Root.B < Root.A"));
        assert!(result.contains(" decompose Root \"Inner\""));
    }
}
