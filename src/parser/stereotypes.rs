//! Visible, uninterpreted stereotypes; reserved UML binding needs semantic validation.
use super::{ParseError, error};
use crate::lex::valid_identifier;
use crate::syntax::DiagramType;

pub(super) fn suffix(
    source: &str,
    line: usize,
    diagram_type: DiagramType,
) -> Result<(&str, Vec<String>), ParseError> {
    let mut head = source.trim_end();
    let mut reversed = Vec::new();
    while head.ends_with(">>") {
        let Some((prefix, name)) = head.rsplit_once(" <<") else {
            return Err(error(line, "stereotype needs space before <<name>>"));
        };
        let name = name
            .strip_suffix(">>")
            .ok_or_else(|| error(line, "unclosed stereotype"))?;
        if !valid_identifier(name) {
            return Err(error(line, "invalid stereotype name"));
        }
        if diagram_type != DiagramType::Generic && name == "bind" {
            return Err(error(
                line,
                "UML <<bind>> needs template binding validation; not implemented",
            ));
        }
        reversed.push(name.to_string());
        head = prefix.trim_end();
    }
    reversed.reverse();
    Ok((head, reversed))
}
