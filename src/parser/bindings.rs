//! UML template binding adornment, distinct from an unchecked free stereotype.
use super::{ParseError, error};
use crate::lex::{split_top_level, valid_identifier, valid_reference};
use crate::syntax::DiagramType;

type BindingArgs = Vec<(String, String)>;

pub(super) fn suffix(
    source: &str,
    line: usize,
    ty: DiagramType,
) -> Result<(&str, Option<BindingArgs>), ParseError> {
    let Some((body, tail)) = source.rsplit_once(" <<bind(") else {
        return Ok((source, None));
    };
    let args = tail
        .strip_suffix(")>>")
        .ok_or_else(|| error(line, "expected <<bind(Formal->Actual)>> at relation end"))?;
    if ty != DiagramType::Class {
        return Err(error(line, "UML template binding requires type: class"));
    }
    if args.trim().is_empty() {
        return Err(error(
            line,
            "template binding needs at least one substitution",
        ));
    }
    let mut bindings = Vec::new();
    for entry in split_top_level(args, ',') {
        let (formal, actual) = entry
            .trim()
            .split_once("->")
            .ok_or_else(|| error(line, "binding argument must be Formal->Actual"))?;
        let (formal, actual) = (formal.trim(), actual.trim());
        if !valid_identifier(formal)
            || !valid_reference(actual)
            || bindings
                .iter()
                .any(|(name, _): &(String, String)| name == formal)
        {
            return Err(error(
                line,
                "invalid or duplicate template binding argument",
            ));
        }
        bindings.push((formal.into(), actual.into()));
    }
    if bindings.is_empty() {
        return Err(error(
            line,
            "template binding needs at least one substitution",
        ));
    }
    Ok((body.trim_end(), Some(bindings)))
}
