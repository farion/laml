//! Optional textual adornments at association ends.
use super::{ParseError, error};
use crate::lex::{quoted_string, valid_identifier};
use crate::syntax::{DiagramType, EndAnnotation};

pub(super) fn parse(
    text: &str,
    line: usize,
    diagram_type: DiagramType,
) -> Result<EndAnnotation, ParseError> {
    let (multiplicity, role) = if let Some(after_quote) = text.strip_prefix('"') {
        let quote = after_quote
            .find('"')
            .ok_or_else(|| error(line, "unclosed end multiplicity"))?
            + 1;
        let multiplicity = quoted_string(&text[..=quote]).map_err(|msg| error(line, msg))?;
        let tail = &text[quote + 1..];
        let role = if tail.is_empty() {
            None
        } else {
            Some(
                tail.strip_prefix('/')
                    .ok_or_else(|| error(line, "expected /role after multiplicity"))?,
            )
        };
        (Some(multiplicity), role)
    } else {
        (
            None,
            Some(
                text.strip_prefix('/')
                    .ok_or_else(|| error(line, "expected quoted multiplicity or /role"))?,
            ),
        )
    };
    if diagram_type == DiagramType::Object {
        return Err(error(
            line,
            "object links cannot have association end adornments",
        ));
    }
    if diagram_type == DiagramType::Class
        && multiplicity
            .as_deref()
            .is_some_and(|value| !valid_multiplicity(value))
    {
        return Err(error(line, "invalid UML multiplicity"));
    }
    let role = role
        .map(|value| {
            let unmarked = value.trim_start_matches(['+', '-', '#', '~']);
            if !valid_identifier(unmarked) {
                Err(error(line, "invalid association-end role"))
            } else {
                Ok(value.to_string())
            }
        })
        .transpose()?;
    Ok(EndAnnotation { multiplicity, role })
}

fn valid_multiplicity(text: &str) -> bool {
    if text == "*" {
        return true;
    }
    if let Some((low, high)) = text.split_once("..") {
        let Ok(low) = low.parse::<u64>() else {
            return false;
        };
        if high == "*" {
            return true;
        }
        return high.parse::<u64>().is_ok_and(|high| low <= high);
    }
    text.parse::<u64>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recognizes_standard_multiplicities() {
        for value in ["0", "1", "*", "0..*", "1..3"] {
            assert!(valid_multiplicity(value), "{value}");
        }
        for value in ["", "3..1", "1..x", "-1", "1,3", "NaN"] {
            assert!(!valid_multiplicity(value), "{value}");
        }
    }
}
