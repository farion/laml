//! Parser-side validation of LSS selectors.

use super::{ParseError, error};
use crate::lex::{valid_identifier, valid_reference};

pub(super) fn validate_selector(selector: &str, line: usize) -> Result<(), ParseError> {
    if let Some((parent, child)) = selector.split_once(' ') {
        let valid_parent = parent.strip_prefix("node.").is_some_and(valid_identifier)
            || parent
                .strip_prefix("node[")
                .and_then(|id| id.strip_suffix(']'))
                .is_some_and(valid_reference);
        let descendant_node = child == "node"
            || child
                .strip_prefix("node[")
                .and_then(|id| id.strip_suffix(']'))
                .is_some_and(valid_identifier);
        let named_activation = parent
            .strip_prefix("node[")
            .and_then(|id| id.strip_suffix(']'))
            .is_some_and(valid_identifier)
            && child
                .strip_prefix("activation[")
                .and_then(|id| id.strip_suffix(']'))
                .is_some_and(valid_identifier);
        if (valid_parent
            && (matches!(child, "lifeline" | "activation" | "coregion")
                || descendant_node
                || validate_selector(child, line).is_ok()))
            || named_activation
        {
            return Ok(());
        }
        return Err(error(line, "invalid contextual style selector"));
    }
    let valid = if selector == "*" {
        true
    } else if let Some(kind) = selector
        .strip_prefix("compartment[")
        .and_then(|s| s.strip_suffix(']'))
    {
        matches!(kind, "attributes" | "operations" | "literals")
    } else if let Some(kind) = selector
        .strip_prefix("member[kind=")
        .and_then(|s| s.strip_suffix(']'))
    {
        matches!(kind, "field" | "method" | "slot" | "literal" | "text")
    } else if let Some(id) = selector
        .strip_prefix("member[")
        .or_else(|| selector.strip_prefix("port["))
        .and_then(|id| id.strip_suffix(']'))
    {
        valid_reference(id)
    } else if let Some(class) = selector
        .strip_prefix("node.")
        .or_else(|| selector.strip_prefix("relation."))
        .or_else(|| selector.strip_prefix("member."))
        .or_else(|| selector.strip_prefix("port."))
        .or_else(|| selector.strip_prefix('.'))
    {
        valid_identifier(class)
    } else if let Some(id) = selector
        .strip_prefix("node[")
        .or_else(|| selector.strip_prefix("relation["))
        .or_else(|| selector.strip_prefix("lifeline["))
        .or_else(|| selector.strip_prefix("activation["))
        .or_else(|| selector.strip_prefix("fragment["))
        .or_else(|| selector.strip_prefix("note["))
        .or_else(|| selector.strip_prefix("coregion["))
        .or_else(|| selector.strip_prefix("continuation["))
        .or_else(|| selector.strip_prefix("gate["))
    {
        id.strip_suffix(']').is_some_and(valid_reference)
    } else if let Some(kind) = selector
        .strip_prefix("annotation[")
        .and_then(|s| s.strip_suffix(']'))
    {
        matches!(
            kind,
            "time-observation"
                | "time-constraint"
                | "duration-observation"
                | "duration-constraint"
                | "state"
                | "invariant"
                | "action"
        )
    } else {
        matches!(
            selector,
            "node"
                | "relation"
                | "member"
                | "port"
                | "lifeline"
                | "activation"
                | "fragment"
                | "note"
                | "coregion"
                | "continuation"
                | "annotation"
                | "ordering"
                | "gate"
        )
    };
    if valid {
        Ok(())
    } else {
        Err(error(line, "invalid style selector"))
    }
}
