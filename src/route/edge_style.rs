//! Relation color, width, and font style lookup.
use crate::styles::{Element, StyleSheet, Value};
use crate::syntax::RelationDecl;

pub(super) fn style(relation: &RelationDecl, sheet: &StyleSheet) -> (String, f64, f64) {
    let element = Element::Relation {
        name: relation.name.as_deref(),
        classes: &relation.classes,
    };
    let number = |key, default| match sheet.get(&element, key).map(|d| &d.value) {
        Some(Value::Number(n)) => *n,
        _ => default,
    };
    let color = match sheet.get(&element, "color").map(|d| &d.value) {
        Some(Value::Color(c)) => c.clone(),
        _ => "#395373".into(),
    };
    (color, number("width", 2.0), number("font-size", 14.0))
}
