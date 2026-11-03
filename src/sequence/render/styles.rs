//! Reuse LSS node/relation selectors for sequence heads and messages.

use crate::sequence::AnnotationKind;
use crate::styles::{Element, StyleSheet, Value};

pub(super) fn escape(text: &str) -> String {
    let mut output = String::new();
    for ch in text.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(ch),
        }
    }
    output
}

fn color(styles: &StyleSheet, element: &Element<'_>, key: &str, default: &str) -> String {
    if let Some(Value::Color(value)) = styles.get(element, key).map(|decl| &decl.value) {
        return value.clone();
    }
    let base = match styles.get(element, "base-color").map(|decl| &decl.value) {
        Some(Value::Color(value)) => Some(value.as_str()),
        _ => None,
    };
    if styles.theme.modern() || base.is_some() || styles.theme.dark() {
        let role = if matches!(element, Element::Note { .. } | Element::Annotation { .. }) {
            crate::syntax::NodeKind::Note
        } else {
            crate::syntax::NodeKind::Generic
        };
        let paint = styles
            .theme
            .colors(base.unwrap_or(styles.theme.default_base(role)));
        return match key {
            "fill" if default != "none" => paint.fill,
            "border-color" if matches!(element, Element::Note { .. }) && styles.theme.modern() => {
                paint.divider
            }
            "border-color" => paint.border,
            "color"
                if matches!(
                    element,
                    Element::Node { .. }
                        | Element::Note { .. }
                        | Element::Continuation { .. }
                        | Element::Activation { .. }
                        | Element::Annotation { .. }
                ) =>
            {
                paint.text
            }
            "color" => styles.theme.edge().into(),
            _ => default.into(),
        };
    }
    default.into()
}

fn size(styles: &StyleSheet, element: &Element<'_>, key: &str, default: f64) -> f64 {
    match styles.get(element, key).map(|decl| &decl.value) {
        Some(Value::Number(value)) => *value,
        _ => default,
    }
}

pub(super) struct StrokePaint {
    pub color: String,
    pub width: f64,
}

pub(super) struct AnnotationPaint {
    pub color: String,
    pub fill: String,
    pub width: f64,
    pub font_size: f64,
}

pub(super) fn annotation_kind(kind: AnnotationKind) -> &'static str {
    match kind {
        AnnotationKind::TimeObservation => "time-observation",
        AnnotationKind::TimeConstraint => "time-constraint",
        AnnotationKind::DurationObservation => "duration-observation",
        AnnotationKind::DurationConstraint => "duration-constraint",
        AnnotationKind::State => "state",
        AnnotationKind::Invariant => "invariant",
        AnnotationKind::Action => "action",
    }
}

pub(super) fn annotation(styles: &StyleSheet, kind: &str) -> AnnotationPaint {
    let element = Element::Annotation { kind };
    AnnotationPaint {
        color: color(styles, &element, "color", "#20252b"),
        fill: color(styles, &element, "fill", "white"),
        width: size(styles, &element, "width", 1.0),
        font_size: size(styles, &element, "font-size", 14.0),
    }
}

pub(super) fn ordering(styles: &StyleSheet) -> StrokePaint {
    let element = Element::Ordering;
    StrokePaint {
        color: color(styles, &element, "color", "#454d55"),
        width: size(styles, &element, "width", 1.2),
    }
}

pub(super) fn coregion(styles: &StyleSheet, id: &str, classes: &[String]) -> StrokePaint {
    let element = Element::Coregion {
        id,
        parent_classes: classes,
    };
    StrokePaint {
        color: color(styles, &element, "color", "#20252b"),
        width: size(styles, &element, "width", 2.0),
    }
}

pub(super) fn lifeline(styles: &StyleSheet, id: &str, classes: &[String]) -> StrokePaint {
    let element = Element::Lifeline {
        id,
        parent_classes: classes,
    };
    StrokePaint {
        color: color(styles, &element, "color", "#66717d"),
        width: size(styles, &element, "width", 1.0),
    }
}

pub(super) struct BoxPaint {
    pub fill: String,
    pub border: String,
    pub color: String,
    pub width: f64,
    pub border_width: f64,
    pub font_size: f64,
}

pub(super) fn activation(
    styles: &StyleSheet,
    id: &str,
    local_id: Option<&str>,
    classes: &[String],
) -> BoxPaint {
    let element = Element::Activation {
        id,
        local_id,
        parent_classes: classes,
    };
    BoxPaint {
        fill: color(styles, &element, "fill", "white"),
        border: color(styles, &element, "border-color", "#20252b"),
        color: color(styles, &element, "color", "#20252b"),
        width: size(styles, &element, "width", 12.0),
        border_width: size(styles, &element, "border-width", 1.0),
        font_size: size(styles, &element, "font-size", 14.0),
    }
}

pub(super) fn continuation(styles: &StyleSheet, id: &str) -> BoxPaint {
    let element = Element::Continuation { id };
    BoxPaint {
        fill: color(styles, &element, "fill", "white"),
        border: color(styles, &element, "border-color", "#20252b"),
        color: color(styles, &element, "color", "#20252b"),
        width: 0.0,
        border_width: size(styles, &element, "border-width", 1.0),
        font_size: size(styles, &element, "font-size", 14.0),
    }
}

pub(super) fn note_accent(styles: &StyleSheet, id: &str) -> String {
    let element = Element::Note { id };
    let base = match styles.get(&element, "base-color").map(|decl| &decl.value) {
        Some(Value::Color(base)) => base.as_str(),
        _ => styles.theme.default_base(crate::syntax::NodeKind::Note),
    };
    styles.theme.colors(base).border
}

pub(super) fn note(styles: &StyleSheet, id: &str) -> BoxPaint {
    let element = Element::Note { id };
    BoxPaint {
        fill: color(styles, &element, "fill", "white"),
        border: color(styles, &element, "border-color", "#20252b"),
        color: color(styles, &element, "color", "#20252b"),
        width: 0.0,
        border_width: size(styles, &element, "border-width", 1.0),
        font_size: size(styles, &element, "font-size", 14.0),
    }
}

pub(super) fn fragment(styles: &StyleSheet, operator: &str, id: Option<&str>) -> BoxPaint {
    let element = Element::Fragment { operator, id };
    BoxPaint {
        fill: color(styles, &element, "fill", "none"),
        border: color(styles, &element, "border-color", "#20252b"),
        color: color(styles, &element, "color", "#20252b"),
        width: 0.0,
        border_width: size(styles, &element, "border-width", 1.0),
        font_size: size(styles, &element, "font-size", 14.0),
    }
}

pub(super) struct NodePaint {
    pub fill: String,
    pub border: String,
    pub text: String,
    pub font_size: f64,
}

pub(super) fn node(styles: &StyleSheet, id: &str, classes: &[String]) -> NodePaint {
    let element = Element::Node { id, classes };
    NodePaint {
        fill: color(styles, &element, "fill", "white"),
        border: color(styles, &element, "border-color", "#20252b"),
        text: color(styles, &element, "color", "#20252b"),
        font_size: size(styles, &element, "font-size", 14.0),
    }
}

pub(super) struct MessagePaint {
    pub color: String,
    pub width: f64,
    pub font_size: f64,
}

pub(super) fn gate(styles: &StyleSheet, id: &str, message: &MessagePaint) -> MessagePaint {
    let element = Element::Gate { id };
    MessagePaint {
        color: color(styles, &element, "color", &message.color),
        width: size(styles, &element, "width", message.width + 0.4),
        font_size: size(styles, &element, "font-size", message.font_size),
    }
}

pub(super) fn message(styles: &StyleSheet, name: Option<&str>, classes: &[String]) -> MessagePaint {
    let element = Element::Relation { name, classes };
    MessagePaint {
        color: color(styles, &element, "color", "#20252b"),
        width: size(styles, &element, "width", 1.6),
        font_size: size(styles, &element, "font-size", 14.0),
    }
}
