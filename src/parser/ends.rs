//! Optional textual adornments at association ends.
use super::{ParseError, error};
use crate::lex::{quoted_string, split_top_level, valid_identifier, valid_reference};
use crate::syntax::{DiagramType, EndAnnotation};

pub(super) fn parse_tokens(
    tokens: &[&str],
    line: usize,
    diagram_type: DiagramType,
) -> Result<EndAnnotation, ParseError> {
    let mut result = EndAnnotation::default();
    for token in tokens {
        if let Some(body) = token
            .strip_prefix("qualifier(")
            .and_then(|rest| rest.strip_suffix(')'))
        {
            if diagram_type == DiagramType::Object || result.qualifier.is_some() {
                return Err(error(line, "duplicate or unsupported qualifier"));
            }
            let fields: Vec<_> = split_top_level(body, ',')
                .into_iter()
                .map(|entry| {
                    let (name, ty) = entry
                        .trim()
                        .split_once(':')
                        .ok_or_else(|| error(line, "qualifier attribute requires name:Type"))?;
                    if !valid_identifier(name.trim()) || !valid_reference(ty.trim()) {
                        return Err(error(line, "invalid qualifier attribute"));
                    }
                    Ok(format!("{}: {}", name.trim(), ty.trim()))
                })
                .collect::<Result<_, ParseError>>()?;
            if fields.is_empty() {
                return Err(error(line, "empty qualifier"));
            }
            let mut names = std::collections::HashSet::new();
            if fields
                .iter()
                .any(|field| !names.insert(field.split(':').next().unwrap_or_default()))
            {
                return Err(error(line, "duplicate qualifier attribute"));
            }
            result.qualifier = Some(fields);
        } else {
            let adornment = parse(token, line, diagram_type)?;
            if (result.multiplicity.is_some() && adornment.multiplicity.is_some())
                || (result.role.is_some() && adornment.role.is_some())
            {
                return Err(error(line, "duplicate association end annotation"));
            }
            result.multiplicity = result.multiplicity.or(adornment.multiplicity);
            result.role = result.role.or(adornment.role);
        }
    }
    Ok(result)
}

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
    Ok(EndAnnotation {
        multiplicity,
        role,
        qualifier: None,
    })
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
