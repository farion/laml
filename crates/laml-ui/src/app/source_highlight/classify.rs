//! Context is recovered from tokens, not from a successful compiler/render pass.
use super::{
    Role, fragments,
    lexer::{Kind, Token},
    members, words,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Section {
    #[default]
    Semantics,
    Settings,
    Layout,
    Styling,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Body {
    #[default]
    Elements,
    Members,
    Text,
    State,
}
#[derive(Default)]
pub(super) struct Context {
    section: Section,
    bodies: Vec<Body>,
    bpmn: bool,
}

impl Context {
    pub(super) fn new(source: &str) -> Self {
        // Section order is arbitrary; use the same tolerant scanner for the prepass.
        let mut probe = Self::default();
        for line in source.split_inclusive('\n') {
            probe.line(&super::lexer::lex(line));
        }
        Self {
            bpmn: probe.bpmn,
            ..Default::default()
        }
    }
    pub(super) fn line(&mut self, all: &[Token<'_>]) -> Vec<Role> {
        let mut out: Vec<_> = all.iter().map(base).collect();
        let positions: Vec<_> = all
            .iter()
            .enumerate()
            .filter(|(_, t)| !matches!(t.kind, Kind::Space | Kind::Comment))
            .map(|(i, _)| i)
            .collect();
        let t: Vec<_> = positions.iter().map(|&i| &all[i]).collect();
        if t.is_empty() {
            return out;
        }
        let mut roles: Vec<_> = positions.iter().map(|&i| out[i]).collect();
        if self.section == Section::Settings
            && t[0].text == "type"
            && t.get(1).is_some_and(|t| t.text == ":")
        {
            self.bpmn = t.get(2).is_some_and(|t| t.text == "bpmn");
        }
        let section = match t[0].text {
            "semantics" => Some(Section::Semantics),
            "settings" => Some(Section::Settings),
            "layout" => Some(Section::Layout),
            "styling" => Some(Section::Styling),
            _ => None,
        };
        if self.bodies.is_empty()
            && let Some(section) = section
        {
            self.section = section;
            roles[0] = Role::Section;
        } else {
            let body = self.bodies.last().copied().unwrap_or_default();
            match (self.section, body) {
                (Section::Semantics, Body::Text) if t[0].text != "}" => {
                    for (i, token) in t.iter().enumerate() {
                        if token.kind != Kind::Space {
                            roles[i] = Role::Text;
                        }
                    }
                    // Free-form notes may contain braces and comment-looking text.
                    for role in &mut out {
                        *role = Role::Text;
                    }
                    return out;
                }
                (Section::Semantics, Body::Members) if !element_line(&t) => {
                    members::classify(&t, &mut roles)
                }
                (Section::Semantics, _) => semantics(&t, &mut roles, body),
                (Section::Settings, _) => properties(&t, &mut roles, true),
                (Section::Layout, _) => layout(&t, &mut roles),
                (Section::Styling, _) => styling(&t, &mut roles),
            }
        }
        // Only structural tokens can change context: braces in strings/labels cannot.
        let opens = t
            .iter()
            .enumerate()
            .filter(|(i, token)| token.text == "{" && roles[*i] != Role::Text)
            .count();
        let closes = t
            .iter()
            .enumerate()
            .filter(|(i, token)| token.text == "}" && roles[*i] != Role::Text)
            .count();
        for _ in 0..closes {
            self.bodies.pop();
        }
        let body = if section.is_some() || self.section != Section::Semantics {
            Body::Elements
        } else if t
            .iter()
            .zip(&roles)
            .any(|(t, r)| t.text == "note" && *r == Role::Keyword)
        {
            Body::Text
        } else if t.iter().zip(&roles).any(|(t, r)| {
            matches!(
                t.text,
                "class" | "interface" | "enum" | "datatype" | "object"
            ) && *r == Role::Keyword
        }) {
            Body::Members
        } else if t
            .iter()
            .zip(&roles)
            .any(|(t, r)| t.text == "state" && *r == Role::Keyword)
        {
            Body::State
        } else if self.bpmn
            || t.iter()
                .zip(&roles)
                .any(|(t, r)| fragments::operator(t.text) && *r == Role::Keyword)
        {
            Body::Elements
        } else if t[0].kind == Kind::Word && roles[0] == Role::Id {
            Body::Members
        } else if t.first().is_some_and(|t| t.text == "{") {
            self.bodies.last().copied().unwrap_or_default()
        } else {
            Body::Elements
        };
        for _ in 0..opens {
            self.bodies.push(body);
        }
        for (i, role) in positions.into_iter().zip(roles) {
            out[i] = role;
        }
        out
    }
}

fn base(t: &Token<'_>) -> Role {
    match t.kind {
        Kind::Space => Role::Plain,
        Kind::Comment => Role::Comment,
        Kind::String => Role::Text,
        Kind::Number => Role::Number,
        Kind::Explicit => Role::ExplicitId,
        Kind::Word => Role::Id,
        Kind::Operator => Role::Operator,
        Kind::Symbol if matches!(t.text, "+" | "-" | "#" | "~" | "!" | "?" | "<" | ">" | "^") => {
            Role::Operator
        }
        Kind::Symbol => Role::Punctuation,
    }
}

fn element_line(t: &[&Token<'_>]) -> bool {
    let word = t.iter().position(|t| t.kind == Kind::Word);
    let declaration = word.is_some_and(|i| {
        words::declaration(t[i].text)
            && !words::modifier(t[i].text)
            && t.get(i + 1)
                .is_some_and(|t| matches!(t.kind, Kind::Word | Kind::Explicit))
    });
    declaration
        || t.first().is_some_and(|t| t.text == "}")
        || t.iter()
            .any(|t| t.kind == Kind::Operator && !matches!(t.text, "--" | "<<" | ">>"))
}

fn semantics(t: &[&Token<'_>], roles: &mut [Role], body: Body) {
    let mut prefix = true;
    for (i, token) in t.iter().enumerate() {
        if matches!(token.kind, Kind::Explicit)
            || matches!(token.text, "+" | "-" | "#" | "~") && prefix
        {
            continue;
        }
        let callable = matches!(token.text, "loop" | "execution" | "object");
        let bracketed = matches!(token.text, "at" | "between" | "order" | "gate");
        let is_declaration = words::declaration(token.text)
            && (callable || t.get(i + 1).is_none_or(|t| t.text != "("))
            && (!bracketed || t.get(i + 1).is_some_and(|t| t.text == "["));
        if prefix && token.kind == Kind::Word && is_declaration {
            roles[i] = Role::Keyword;
            prefix = matches!(
                token.text,
                "abstract" | "static" | "anonymous" | "external" | "association" | "internal"
            );
        } else if token.kind != Kind::Space {
            prefix = false;
        }
    }
    if body == Body::State && matches!(t[0].text, "entry" | "do" | "exit") {
        roles[0] = Role::Keyword;
        for role in &mut roles[1..] {
            *role = Role::Text;
        }
        return;
    }
    inline_attributes(t, roles);
    if fragments::classify(t, roles) {
        return;
    }
    let first_arrow = t
        .iter()
        .position(|t| t.kind == Kind::Operator && !matches!(t.text, "<<" | ">>"));
    let first_colon = (0..t.len()).find(|&i| t[i].text == ":" && !qualified_colon(t, i));
    let named_activation = t.iter().filter(|t| t.text == ":").count() > 1;
    let branches = t
        .iter()
        .any(|t| matches!(t.text, "<X>" | "<+>" | "<O>" | "<event>"));
    let mut depth = 0usize;
    let mut label = false;
    for i in 0..t.len() {
        let token = t[i];
        if label && branches && token.text == ";" {
            label = false;
            continue;
        }
        if label {
            roles[i] = Role::Text;
            continue;
        }
        if matches!(token.text, "(" | "[") {
            depth += 1;
        } else if matches!(token.text, ")" | "]") {
            depth = depth.saturating_sub(1);
        }
        if token.text == ":" && depth == 0 && !qualified_colon(t, i) {
            let prefix_colon = first_colon == Some(i)
                && (first_arrow.is_some_and(|a| i < a)
                    || named_activation && t[i + 1..].iter().any(|t| matches!(t.text, "+" | "-")));
            let property_tail = t[i + 1..].get(1).is_some_and(|t| t.text == "=");
            if !prefix_colon && !property_tail {
                label = true;
            }
        }
        if token.text == "at" && i > 0 && matches!(t[0].text, "state" | "event") {
            roles[i] = Role::Keyword;
        }
    }
}

fn qualified_colon(t: &[&Token<'_>], i: usize) -> bool {
    i > 0 && t[i - 1].text == ":" && t[i - 1].range.end == t[i].range.start
        || t.get(i + 1)
            .is_some_and(|next| next.text == ":" && t[i].range.end == next.range.start)
}

fn inline_attributes(t: &[&Token<'_>], roles: &mut [Role]) {
    let mut key = "";
    let mut value = false;
    for (i, token) in t.iter().enumerate() {
        if token.kind == Kind::Word && t.get(i + 1).is_some_and(|t| t.text == "=") {
            key = token.text;
            roles[i] = Role::Property;
            value = true;
        } else if matches!(token.text, ";" | ")" | "]") {
            value = false;
        } else if value && token.kind == Kind::Word {
            roles[i] = if key == "style-classes" {
                Role::StyleClass
            } else if words::reference_property(key) {
                Role::Id
            } else {
                Role::Value
            };
        }
    }
}

fn properties(t: &[&Token<'_>], roles: &mut [Role], settings: bool) {
    if t[0].kind == Kind::Word && t.get(1).is_some_and(|t| t.text == ":") {
        roles[0] = Role::Property;
        for (i, token) in t.iter().enumerate().skip(2) {
            if token.kind == Kind::Word {
                roles[i] = if settings && words::reference_property(t[0].text) {
                    Role::Id
                } else {
                    Role::Value
                };
            }
        }
    }
    if t[0].text == "import" {
        roles[0] = Role::Keyword;
    }
}

fn layout(t: &[&Token<'_>], roles: &mut [Role]) {
    inline_attributes(t, roles);
    for (i, token) in t.iter().enumerate() {
        if matches!(
            token.text,
            "above" | "below" | "left" | "right" | "of" | "v"
        ) && roles[i] == Role::Id
        {
            roles[i] = Role::Operator;
        }
        if matches!(token.text, "relation" | "port") && t.get(i + 1).is_some_and(|t| t.text == "[")
        {
            roles[i] = Role::Keyword;
        }
    }
    // Timing has section-level properties and ruler declarations.
    properties(t, roles, false);
    if t[0].text == "tick" {
        roles[0] = Role::Keyword;
    }
}

fn styling(t: &[&Token<'_>], roles: &mut [Role]) {
    if t.get(1).is_some_and(|t| t.text == ":") && t[0].kind == Kind::Word {
        properties(t, roles, false);
        return;
    }
    let mut bracket = 0usize;
    for (i, token) in t.iter().enumerate() {
        if token.text == "[" {
            bracket += 1;
        } else if token.text == "]" {
            bracket = bracket.saturating_sub(1);
        }
        if token.kind == Kind::Word {
            if bracket == 0 && i > 0 && t[i - 1].text == "." {
                roles[i] = Role::StyleClass;
            } else if bracket == 0 && words::selector(token.text) {
                roles[i] = Role::Keyword;
            }
        }
    }
    inline_attributes(t, roles);
    if t[0].text == "import" {
        roles[0] = Role::Keyword;
    }
}
