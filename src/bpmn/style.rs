//! Restrict LSS to BPMN-safe properties; do not silently ignore requested styles.
use super::{Document, Kind, Node};
use crate::styles::{Element, StyleSheet, Value};

fn element(node: &Node) -> Element<'_> {
    Element::Node {
        id: &node.id,
        classes: &[],
    }
}
pub(super) fn value<'a>(sheet: &'a StyleSheet, node: &Node, property: &str) -> Option<&'a Value> {
    sheet.get(&element(node), property).map(|d| &d.value)
}
pub(super) fn icon<'a>(sheet: &'a StyleSheet, node: &'a Node) -> Option<&'a str> {
    if let Some(Value::Keyword(name)) = value(sheet, node, "icon") {
        Some(name)
    } else {
        node.icon.as_deref()
    }
}
pub fn validate_styles(doc: &Document, sheet: &StyleSheet) -> Result<(), String> {
    for rule in &sheet.rules {
        for declaration in &rule.declarations {
            if !matches!(
                declaration.name.as_str(),
                "fill" | "border-color" | "icon" | "icon-set"
            ) {
                return Err(format!(
                    "BPMN styling property '{}' is not yet supported",
                    declaration.name
                ));
            }
        }
    }
    for node in &doc.nodes {
        if value(sheet, node, "icon").is_some()
            && !matches!(
                node.kind,
                Kind::Task
                    | Kind::UserTask
                    | Kind::ServiceTask
                    | Kind::SendTask
                    | Kind::ReceiveTask
                    | Kind::ScriptTask
                    | Kind::ManualTask
                    | Kind::Collapsed
                    | Kind::Subprocess
            )
        {
            return Err(format!(
                "BPMN marker {} cannot be replaced by a free icon",
                node.id
            ));
        }
    }
    Ok(())
}
