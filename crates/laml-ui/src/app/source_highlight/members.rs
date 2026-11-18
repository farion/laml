//! UML member names, parameter names, types and literal defaults stay separate.
use super::{
    Role,
    lexer::{Kind, Token},
    words,
};

pub(super) fn classify(tokens: &[&Token<'_>], roles: &mut [Role]) {
    // Literal and punctuation roles from the lexer are retained throughout.
    for (i, t) in tokens.iter().enumerate() {
        if t.kind == Kind::Word {
            roles[i] = if words::modifier(t.text) {
                Role::Keyword
            } else {
                Role::Type
            };
        }
    }
    let start = tokens
        .iter()
        .position(|t| t.kind == Kind::Word && !words::modifier(t.text));
    let Some(start) = start else {
        return;
    };
    let mut depth = 0usize;
    let end = tokens
        .iter()
        .position(|t| {
            if t.text == "(" {
                depth += 1;
            } else if t.text == ")" {
                depth = depth.saturating_sub(1);
            }
            t.text == "=" && depth == 0
        })
        .unwrap_or(tokens.len());
    let open = tokens[..end].iter().position(|t| t.text == "(");
    if let Some(open) = open {
        if open > start && tokens[open - 1].kind == Kind::Word {
            roles[open - 1] = Role::Member;
        }
        let close = tokens.iter().rposition(|t| t.text == ")").unwrap_or(end);
        let mut segment = open + 1;
        let mut depth = 0usize;
        for i in open + 1..=close {
            let text = tokens.get(i).map_or(")", |t| t.text);
            if i == close || text == "," && depth == 0 {
                field(tokens, roles, segment, i);
                segment = i + 1;
            } else if matches!(text, "<" | "(" | "[") {
                depth += 1;
            } else if matches!(text, ">" | ")" | "]" | ">>") {
                depth = depth.saturating_sub(if text == ">>" { 2 } else { 1 });
            }
        }
    } else {
        field(tokens, roles, start, tokens.len());
    }
    // Defaults may be expressions, but LAML does not prescribe an expression language.
    if end < tokens.len() {
        for (i, t) in tokens.iter().enumerate().skip(end + 1) {
            if t.kind == Kind::Word {
                roles[i] = Role::Value;
            }
        }
    }
}

fn field(tokens: &[&Token<'_>], roles: &mut [Role], start: usize, end: usize) {
    let equal = (start..end).find(|&i| tokens[i].text == "=").unwrap_or(end);
    let colon = (start..equal).find(|&i| tokens[i].text == ":");
    let names: Vec<_> = (start..colon.unwrap_or(equal))
        .filter(|&i| tokens[i].kind == Kind::Word && !words::modifier(tokens[i].text))
        .collect();
    if let Some(&name) = if colon.is_some() {
        names.first()
    } else {
        names.last()
    } {
        roles[name] = Role::Member;
    }
    for i in equal.saturating_add(1)..end {
        if tokens[i].kind == Kind::Word {
            roles[i] = Role::Value;
        }
    }
}
