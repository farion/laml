//! Sequence fragment guards are display text, including named/nested fragments.
use super::{
    Role,
    lexer::{Kind, Token},
};

pub(super) fn operator(text: &str) -> bool {
    matches!(
        text,
        "alt"
            | "opt"
            | "loop"
            | "par"
            | "break"
            | "critical"
            | "neg"
            | "assert"
            | "strict"
            | "seq"
            | "ignore"
            | "consider"
            | "ref"
            | "else"
            | "and"
    )
}

pub(super) fn classify(t: &[&Token<'_>], roles: &mut [Role]) -> bool {
    let start = if t.get(1).is_some_and(|t| t.text == ":") {
        2
    } else {
        0
    };
    if !t.get(start).is_some_and(|t| operator(t.text)) {
        return false;
    }
    if t.last().is_none_or(|t| t.text != "{") && !matches!(t[start].text, "else" | "and") {
        return false;
    }
    if start == 2 && t[0].kind == Kind::Word {
        roles[0] = Role::Id;
    }
    let mut prefix = true;
    let mut depth = 0usize;
    for i in start..t.len() {
        if prefix && operator(t[i].text) {
            roles[i] = Role::Keyword;
            continue;
        }
        if t[i].text == "(" {
            depth += 1;
        }
        if t[i].text == ")" {
            depth = depth.saturating_sub(1);
            continue;
        }
        if t[i].text == "{" && i + 1 == t.len() {
            continue;
        }
        prefix = false;
        if depth == 0 {
            roles[i] = Role::Text;
        } else if t[i].kind == Kind::Word && roles[i] == Role::Id {
            roles[i] = Role::Text;
        }
    }
    true
}
