//! Lossless, UTF-8-safe tokens. Recovery at newline keeps incomplete input local.
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Space,
    Word,
    Explicit,
    Number,
    String,
    Comment,
    Symbol,
    Operator,
}
#[derive(Debug)]
pub(super) struct Token<'a> {
    pub text: &'a str,
    pub range: Range<usize>,
    pub kind: Kind,
}

pub(super) fn lex(line: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < line.len() {
        let start = i;
        let ch = line[i..].chars().next().unwrap();
        let kind = if ch.is_whitespace() {
            i += ch.len_utf8();
            while i < line.len() && line[i..].starts_with(char::is_whitespace) {
                i += line[i..].chars().next().unwrap().len_utf8();
            }
            Kind::Space
        } else if line[i..].starts_with("//")
            && (i == 0 || line[..i].ends_with(char::is_whitespace))
        {
            i = line.trim_end_matches(['\r', '\n']).len();
            Kind::Comment
        } else if ch == '"' {
            i += 1;
            let mut escaped = false;
            while i < line.len() {
                let c = line[i..].chars().next().unwrap();
                if matches!(c, '\r' | '\n') {
                    break;
                }
                i += c.len_utf8();
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    break;
                }
            }
            Kind::String
        } else if let Some(op) = [
            "<-->", "--|>", "..|>", "<|--", "<|..", "--^>", "<--", "-->", "<->", "->>", "<<-",
            "*--", "--*", "o--", "--o", "..>", "<..", "->", "<-", "--", "..", "==", "=>", "<=",
            "<<", ">>", "<X>", "<+>", "<O>", "<event>", "~~>",
        ]
        .into_iter()
        .find(|op| line[i..].starts_with(op))
        {
            i += op.len();
            Kind::Operator
        } else if ch == '@' || ch.is_alphabetic() || ch == '_' {
            i += ch.len_utf8();
            while i < line.len() {
                let c = line[i..].chars().next().unwrap();
                if c.is_alphanumeric()
                    || c == '_'
                    || (c == '-' && line[i + 1..].starts_with(char::is_alphabetic))
                {
                    i += c.len_utf8();
                } else {
                    break;
                }
            }
            if ch == '@' {
                Kind::Explicit
            } else {
                Kind::Word
            }
        } else if ch.is_ascii_digit() {
            i += 1;
            while i < line.len() && line.as_bytes()[i].is_ascii_digit() {
                i += 1;
            }
            if line[i..].starts_with('.') && line[i + 1..].starts_with(|c: char| c.is_ascii_digit())
            {
                i += 1;
                while i < line.len() && line.as_bytes()[i].is_ascii_digit() {
                    i += 1;
                }
            }
            Kind::Number
        } else {
            i += ch.len_utf8();
            Kind::Symbol
        };
        tokens.push(Token {
            text: &line[start..i],
            range: start..i,
            kind,
        });
    }
    tokens
}
