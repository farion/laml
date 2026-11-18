//! Notation-specific choices reuse the editor's compiler-syntax catalogs.
use super::{Entry, context::Context, symbols};
use crate::notation::Notation;
fn words(items: &[&str], detail: &str) -> Vec<Entry> {
    items.iter().map(|s| Entry::new(s, detail, s)).collect()
}
fn properties(items: &[&str], separator: char) -> Vec<Entry> {
    items
        .iter()
        .map(|key| {
            Entry::new(
                &format!("{key}{separator}"),
                description(key),
                &format!(
                    "{key}{separator}{}",
                    if separator == ':' { " " } else { "" }
                ),
            )
        })
        .collect()
}
fn description(key: &str) -> &'static str {
    match key {
        "type" => "Diagram notation",
        "theme" => "Document theme; default: modern",
        "direction" => "Automatic layout direction; default: left-to-right",
        "background" => "Canvas background; default: opaque",
        "label" => "Displayed text (quoted)",
        "style-classes" => "Comma-separated style class names",
        "base-color" => "Derive a readable color family from a hex color",
        "align" => "Mandatory alignment",
        "gap" => "Nonnegative spacing in diagram units",
        "of" => "Reference to a classifier",
        "ref" => "Reference to an interaction",
        "font-file" | "font-italic-file" => "Local font file path (quoted)",
        "fill" | "color" | "border-color" => "Hex color or none",
        "relation-label" => "Relation label placement; default: near",
        "allow-diagonal" => "Allow diagonal routes; true or false",
        "font-size" => "Positive font size in diagram units",
        "display" => "block or none",
        "shape" => "Node contour",
        "icon" => "Phosphor icon name",
        "width" | "height" | "padding" | "border-width" | "corner-radius" | "min-width"
        | "min-height" | "length" => "Nonnegative size in diagram units",
        _ => "Notation property",
    }
}
pub(super) fn settings(ty: Notation) -> Vec<Entry> {
    let mut keys = vec![
        "type",
        "theme",
        "background",
        "font-file",
        "font-italic-file",
    ];
    match ty {
        Notation::Sequence => keys.push("semantic-check"),
        Notation::Timing => keys.extend(["interaction", "time-unit"]),
        Notation::Bpmn => keys.extend(["direction", "allow-diagonal"]),
        t if t.c4() && t != Notation::C4Code => {
            keys.extend(["direction", "scope", "allow-diagonal"])
        }
        _ => {
            keys.extend([
                "direction",
                "naming-convention",
                "relation-label",
                "allow-diagonal",
            ]);
            if matches!(ty, Notation::Communication | Notation::InteractionOverview) {
                keys.push("interaction");
            }
        }
    }
    properties(&keys, ':')
}
pub(super) fn styles(ctx: &Context) -> Vec<Entry> {
    if ctx.heading == "styling" {
        return [
            "node",
            "port",
            "relation",
            "member",
            "compartment",
            "lifeline",
            "activation",
            "fragment",
            "note",
            "coregion",
            "continuation",
            "annotation",
            "ordering",
            "gate",
        ]
        .into_iter()
        .map(|s| {
            Entry::new(
                s,
                "Style selector",
                &format!("{s} {{\n    {{property}}: {{value}}\n}}"),
            )
        })
        .collect();
    }
    let selector = ctx.heading.split(['[', '.', ' ', ':']).next().unwrap_or("");
    let keys: &[&str] = match selector {
        "relation" => &[
            "color",
            "base-color",
            "width",
            "font-size",
            "display",
            "interface-notation",
        ],
        "member" => &[
            "color",
            "font-size",
            "display",
            "symbol-rendering",
            "icon-set",
        ],
        "compartment" => &["display"],
        "port" => &["color", "fill", "border-color", "base-color", "display"],
        "lifeline" => &["color", "width", "font-size"],
        "coregion" | "ordering" => &["color", "width"],
        "gate" => &["color", "width", "font-size"],
        "note" | "continuation" => &[
            "fill",
            "color",
            "border-color",
            "base-color",
            "border-width",
            "font-size",
        ],
        "activation" | "fragment" => &[
            "fill",
            "border-color",
            "base-color",
            "border-width",
            "width",
            "font-size",
            "color",
        ],
        "annotation" => &["fill", "color", "base-color", "width", "font-size"],
        _ => &[
            "fill",
            "color",
            "border-color",
            "base-color",
            "border-width",
            "font-size",
            "padding",
            "width",
            "height",
            "min-width",
            "min-height",
            "corner-radius",
            "shape",
            "icon",
            "display",
            "symbol-rendering",
            "icon-set",
            "length",
        ],
    };
    properties(keys, ':')
}
pub(super) fn attributes(ty: Notation, before: &str, _source: &str) -> Vec<Entry> {
    let line = before.rsplit('\n').next().unwrap_or("").trim();
    let first = line
        .trim_start_matches('@')
        .split_whitespace()
        .next()
        .unwrap_or("");
    let keys: &[&str] = if line.starts_with("port ") {
        if matches!(ty, Notation::Component | Notation::Composite) {
            &["provides", "requires", "style-classes"]
        } else {
            &["style-classes"]
        }
    } else if ty == Notation::Sequence {
        if first == "loop" {
            &["min", "max", "guard"]
        } else if line.ends_with(':')
            || line.is_empty()
            || before
                .rsplit('\n')
                .next()
                .is_some_and(|l| l.trim_start().starts_with('@'))
        {
            &["kind", "arrow", "style-classes"]
        } else {
            &["label", "selector", "type", "style-classes"]
        }
    } else if ty == Notation::Timing {
        &["of"]
    } else if ty == Notation::Bpmn {
        &["label", "icon"]
    } else if ty.c4() && ty != Notation::C4Code {
        if line.split_whitespace().any(|w| w == "container") {
            &[
                "label",
                "description",
                "technology",
                "appearance",
                "style-classes",
            ]
        } else if line.split_whitespace().any(|w| w == "component") {
            &["label", "description", "technology", "style-classes"]
        } else {
            &["label", "description", "style-classes"]
        }
    } else if line
        .split_whitespace()
        .any(|s| matches!(s, "object" | "part" | "role"))
    {
        &["label", "of", "style-classes"]
    } else if line.starts_with("interaction-use ") {
        &["ref", "style-classes"]
    } else if ty
        .relation_types()
        .iter()
        .any(|r| line.contains(r.operator))
    {
        &["style-classes"]
    } else {
        &["label", "style-classes"]
    };
    properties(keys, '=')
}
pub(super) fn values(ty: Notation, key: &str, ctx: &Context, source: &str) -> Vec<Entry> {
    let items: &[&str] = match key {
        "type" if ctx.section == "settings" => {
            return Notation::ALL
                .into_iter()
                .filter(|n| *n != Notation::Gne)
                .map(|n| Entry::new(n.key(), "Diagram notation", n.key()))
                .collect();
        }
        "theme" => &[
            "modern",
            "plain",
            "modern-mono",
            "modern-dark",
            "plain-dark",
            "modern-mono-dark",
        ],
        "background" => &["opaque", "transparent"],
        "direction" if ty == Notation::Bpmn => &["left-to-right", "top-to-bottom"],
        "direction" => &[
            "left-to-right",
            "right-to-left",
            "top-to-bottom",
            "bottom-to-top",
        ],
        "semantic-check" => &["on", "off"],
        "allow-diagonal" => &["true", "false"],
        "naming-convention" => &["uppercase-nodes-lowercase-relations"],
        "relation-label" | "label" if ctx.section == "layout" || key == "relation-label" => {
            &["near", "along", "inline"]
        }
        "display" => &["block", "none"],
        "shape" => &["rectangle", "ellipse", "diamond", "note"],
        "interface-notation" => &["box", "lollipop", "socket"],
        "symbol-rendering" => &["text", "icons"],
        "icon-set" => &["phosphor"],
        "appearance" => {
            return ["browser", "database", "bucket", "folder", "terminal"]
                .into_iter()
                .map(|v| Entry::new(v, "Container appearance", &format!("\"{v}\"")))
                .collect();
        }
        "kind" if ty == Notation::Sequence => &["create"],
        "arrow" if ty == Notation::Sequence => &["filled", "open"],
        "align" if ctx.line.contains('?') => &[
            "left", "right", "top", "bottom", "center", "center-x", "center-y",
        ],
        "align"
            if ctx.line.contains("above")
                || ctx.line.contains("below")
                || ctx.line.contains(" ^ ")
                || ctx.line.contains(" v ") =>
        {
            &["left", "right", "center"]
        }
        "align" => &["top", "bottom", "center"],
        "side" | "from" | "to" => &["left", "right", "top", "bottom"],
        "reference" => &["first", "second"],
        "label-orientation" => &["auto", "horizontal", "vertical"],
        "of" | "ref" | "scope" | "participant" | "message" | "owner" | "source" | "target"
        | "requires" | "provides" => {
            return symbols::references(source);
        }
        "style-classes" => return symbols::classes(source),
        "fill" | "color" | "border-color" => &["none"],
        _ => &[],
    };
    let mut out = words(items, description(key));
    if matches!(key, "base-color" | "fill" | "color" | "border-color") {
        out.push(Entry::new("#rrggbb", "Hex RGB color", "#{color}"));
    }
    if matches!(
        key,
        "label"
            | "description"
            | "technology"
            | "selector"
            | "guard"
            | "type"
            | "font-file"
            | "font-italic-file"
    ) && (ctx.section != "settings" || matches!(key, "font-file" | "font-italic-file"))
    {
        out.push(Entry::new("quoted text", description(key), "{text:q}"));
    }
    if matches!(
        key,
        "width"
            | "height"
            | "min-width"
            | "min-height"
            | "padding"
            | "border-width"
            | "corner-radius"
            | "length"
            | "gap"
            | "min"
            | "max"
    ) {
        out.push(Entry::new("number", "Nonnegative number", "{number}"));
    }
    if key == "font-size" {
        out.push(Entry::new(
            "positive number",
            "Positive font size",
            "{positive}",
        ));
    }
    if key == "icon" {
        out.extend(symbols::icons());
    }
    out
}
pub(super) fn constructs(ty: Notation, ctx: &Context, _source: &str) -> Vec<Entry> {
    let mut out = vec![];
    if ty.graph() && (!ty.c4() || ty == Notation::C4Code) {
        // Structural/member bodies are not declaration scopes.
        let heading_kind = ctx
            .heading
            .split_whitespace()
            .find(|w| !matches!(*w, "abstract" | "+" | "-" | "#" | "~"))
            .unwrap_or("");
        if heading_kind == "enum" {
            return vec![Entry::new("literal", "Enumeration literal", "{name}")];
        }
        if heading_kind == "object" {
            return vec![Entry::new("slot", "Object slot", "{name} = {text:q}")];
        }
        if ctx
            .heading
            .split_whitespace()
            .any(|w| matches!(w, "class" | "interface" | "datatype"))
        {
            let mut members = vec![
                Entry::new("field", "UML field", "+{name}: {type}"),
                Entry::new(
                    "method",
                    "UML operation",
                    "+{name}({parameter}: {type}): {result}",
                ),
            ];
            if heading_kind == "class" {
                members.push(Entry::new("port", "Named port", "port {id}"));
            }
            return members;
        }
        for kind in crate::element_edit::ElementKind::available(&ty.starter(false)) {
            let word = kind.keyword().trim();
            if word.is_empty() {
                if kind == crate::element_edit::ElementKind::Rectangle {
                    out.push(Entry::new("Node", "Generic node", "{id}(label={label:q})"));
                }
                continue;
            }
            let block = matches!(
                word,
                "class"
                    | "interface"
                    | "enum"
                    | "datatype"
                    | "object"
                    | "package"
                    | "component"
                    | "structure"
                    | "profile"
                    | "system"
            );
            let template = format!(
                "{word} {{id}}{}",
                if block { " {\n    {content}\n}" } else { "" }
            );
            out.push(Entry::new(word, kind.title(), &template));
        }
    }
    for r in crate::notation_edit::catalog::recipes(ty)
        .into_iter()
        .filter(|r| r.section == "semantics" && r.layout.is_empty())
    {
        if matches!(
            r.title,
            "Attribute"
                | "Operation"
                | "Port"
                | "Required interface port"
                | "Provided interface port"
        ) {
            continue;
        }
        if ty == Notation::Bpmn {
            let heading = ctx.heading.split_whitespace().next().unwrap_or("");
            if r.title == "Lane" && heading != "pool" {
                continue;
            }
            if matches!(r.title, "Pool" | "Wrap root process in pool") && heading != "semantics" {
                continue;
            }
        }
        out.push(Entry::new(r.title, "Construct template", r.body));
    }
    out
}
pub(super) fn layout(ty: Notation, before: &str, _source: &str) -> Vec<Entry> {
    if before.contains(':') {
        let keys: &[&str] = if ty == Notation::Bpmn {
            &["from", "to", "label-orientation"]
        } else if before.starts_with("port[") {
            &["side"]
        } else if before.starts_with("relation[") {
            &["label"]
        } else if before.contains('?') {
            &["align", "reference"]
        } else {
            &["align", "gap"]
        };
        return properties(keys, '=');
    }
    let mut out = words(&["above", "below", "<", ">", "?"], "Layout relationship");
    for r in crate::notation_edit::catalog::recipes(ty)
        .into_iter()
        .filter(|r| r.section == "layout")
    {
        out.push(Entry::new(r.title, "Layout template", r.body));
    }
    out.extend(symbols::references(_source));
    out
}
