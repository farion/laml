//! UTF-8 byte-based placeholder navigation; no hidden document coordinates.
use std::ops::Range;
#[derive(Clone, Debug)]
pub struct Expansion {
    pub text: String,
    pub fields: Vec<Range<usize>>,
}
pub fn expand(template: &str, indent: &str, eol: &str) -> Expansion {
    let mut out = Expansion {
        text: String::new(),
        fields: vec![],
    };
    let mut at = 0;
    while at < template.len() {
        let rest = &template[at..];
        if rest.starts_with('{')
            && let Some(end) = rest.find('}')
        {
            let field = &rest[1..end];
            let (name, quoted) = field
                .strip_suffix(":q")
                .map_or((field, false), |n| (n, true));
            if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                if quoted {
                    out.text.push('"');
                }
                let start = out.text.len();
                out.text.push_str(default_value(name));
                out.fields.push(start..out.text.len());
                if quoted {
                    out.text.push('"');
                }
                at += end + 1;
                continue;
            }
        }
        let c = rest.chars().next().unwrap();
        if c == '\n' {
            out.text.push_str(eol);
            out.text.push_str(indent);
        } else {
            out.text.push(c);
        }
        at += c.len_utf8();
    }
    out
}
fn default_value(name: &str) -> &str {
    match name {
        "content" => "",
        "id" | "child" => "NewElement",
        "from" | "participant" | "first" => "Source",
        "to" | "other" | "second" => "Target",
        "label" | "text" => "Text",
        "condition" | "otherwise" => "condition",
        "minimum" | "time" | "number" => "0",
        "positive" => "14",
        "maximum" => "1",
        "gap" => "40",
        "scale" => "1",
        "color" => "808080",
        "property" => "color",
        "value" => "#808080",
        "selector" => "node",
        "type" | "result" => "Type",
        _ => name,
    }
}
#[derive(Clone, Debug)]
pub struct Session {
    pub fields: Vec<Range<usize>>,
    pub active: usize,
    pub revision: u64,
    pub end: usize,
}
impl Session {
    pub fn new(expansion: &Expansion, at: usize, revision: u64) -> Option<Self> {
        (!expansion.fields.is_empty()).then(|| Self {
            fields: expansion
                .fields
                .iter()
                .map(|r| at + r.start..at + r.end)
                .collect(),
            active: 0,
            revision,
            end: at + expansion.text.len(),
        })
    }
    pub fn selection(&self) -> Range<usize> {
        self.fields[self.active].clone()
    }
    pub fn step(&mut self, backward: bool) -> Option<Range<usize>> {
        if backward {
            self.active = self.active.saturating_sub(1);
        } else {
            self.active += 1;
        }
        self.fields.get(self.active).cloned()
    }
    /// Track a single text edit confined to the active field. Any unrelated edit
    /// invalidates the session (including undo, paste outside, or tab switches).
    pub fn update(&mut self, before: &str, after: &str, revision: u64) -> bool {
        let mut start = before
            .bytes()
            .zip(after.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        while !before.is_char_boundary(start) || !after.is_char_boundary(start) {
            start -= 1;
        }
        let max_suffix = before.len().min(after.len()) - start;
        let mut suffix = before
            .bytes()
            .rev()
            .zip(after.bytes().rev())
            .take(max_suffix)
            .take_while(|(a, b)| a == b)
            .count();
        while !before.is_char_boundary(before.len() - suffix)
            || !after.is_char_boundary(after.len() - suffix)
        {
            suffix -= 1;
        }
        let end = before.len() - suffix;
        let field = &self.fields[self.active];
        if start < field.start || end > field.end {
            return false;
        }
        let delta = after.len() as isize - before.len() as isize;
        for (i, r) in self.fields.iter_mut().enumerate() {
            if i == self.active {
                r.end = r.end.saturating_add_signed(delta);
            } else if r.start >= end {
                r.start = r.start.saturating_add_signed(delta);
                r.end = r.end.saturating_add_signed(delta);
            }
        }
        self.end = self.end.saturating_add_signed(delta);
        self.revision = revision;
        true
    }
}
