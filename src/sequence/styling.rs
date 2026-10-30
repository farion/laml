//! Extract sequence LSS without changing source line numbers or import policy.

use crate::lex::without_comment;
use crate::parser::{ParseError, parse_stylesheet};
use crate::syntax::StyleEntry;

pub(super) fn extract(source: &str) -> Result<(String, Vec<StyleEntry>), ParseError> {
    let mut body = String::new();
    let mut stylesheet = String::new();
    let mut found = false;
    let mut depth = 0_usize;
    let mut outer_depth = 0_usize;
    let mut start = 0;
    for (index, raw) in source.lines().enumerate() {
        let text = without_comment(raw).trim();
        if depth == 0 && outer_depth == 0 && text == "styling {" {
            if found {
                return Err(ParseError {
                    line: index + 1,
                    message: "duplicate styling section".into(),
                });
            }
            found = true;
            start = index + 1;
            stylesheet = "\n".repeat(index);
            depth = 1;
            stylesheet.push_str(raw);
            stylesheet.push('\n');
            body.push('\n');
            continue;
        }
        if depth != 0 {
            stylesheet.push_str(raw);
            stylesheet.push('\n');
            body.push('\n');
            if text.ends_with('{') {
                depth += 1;
            }
            if text == "}" {
                depth -= 1;
            }
        } else {
            body.push_str(raw);
            body.push('\n');
            if text.ends_with('{') {
                outer_depth += 1;
            }
            if text == "}" {
                outer_depth = outer_depth.saturating_sub(1);
            }
        }
    }
    if depth != 0 {
        return Err(ParseError {
            line: start,
            message: "unclosed styling section".into(),
        });
    }
    let entries = if found {
        parse_stylesheet(&stylesheet)?.into_iter().collect()
    } else {
        Vec::new()
    };
    Ok((body, entries))
}

/// Detect the explicit type setting, never a comment or a label elsewhere.
pub fn is_sequence_source(source: &str) -> bool {
    let mut settings = false;
    for line in source.lines() {
        let text = without_comment(line).trim();
        match text {
            "settings {" => settings = true,
            "}" if settings => settings = false,
            _ if settings && text == "type: sequence" => return true,
            _ => {}
        }
    }
    false
}
