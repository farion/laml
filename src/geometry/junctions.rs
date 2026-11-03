//! Small unlabeled junctions are not ordinary diamond-shaped text nodes.
use super::{BoxNode, NodeShape};
use crate::styles::{Element, StyleSheet, Value};
use crate::syntax::{FlowDirection, NodeDecl, NodeKind};
use crate::text::measure as text_width;

pub(super) fn measure(
    node: &NodeDecl,
    sheet: &StyleSheet,
    direction: FlowDirection,
) -> Result<BoxNode, String> {
    let element = Element::Node {
        id: &node.id,
        classes: &node.classes,
    };
    if matches!(
        node.kind,
        NodeKind::StateInitial
            | NodeKind::ActivityInitial
            | NodeKind::ActivityFork
            | NodeKind::ActivityJoin
    ) && sheet.get(&element, "fill").is_some()
    {
        return Err("initial or fork/join control node must remain visibly filled".into());
    }
    if [
        "shape",
        "width",
        "height",
        "min-width",
        "min-height",
        "icon",
    ]
    .iter()
    .any(|key| sheet.get(&element, key).is_some())
    {
        return Err(if matches!(
            node.kind,
            NodeKind::StateInitial
                | NodeKind::StateFinal
                | NodeKind::StateChoice
                | NodeKind::ActivityInitial
                | NodeKind::ActivityFinal
                | NodeKind::ActivityFlowFinal
                | NodeKind::ActivityDecision
                | NodeKind::ActivityMerge
                | NodeKind::ActivityFork
                | NodeKind::ActivityJoin
        ) {
            "state/activity control node is a fixed UML primitive, not a resizable node"
        } else {
            "junction diamond is a fixed UML/GNE primitive, not a resizable node"
        }
        .into());
    }
    let palette = super::node_notation::palette(node.kind, sheet, &element);
    let fill = super::node_notation::color(
        sheet,
        &element,
        "fill",
        palette.as_ref().map_or(
            if sheet.theme.dark() {
                "#181f2a"
            } else {
                "#ffffff"
            },
            |paint| paint.fill.as_str(),
        ),
    );
    let color = super::node_notation::text_paint(sheet, &element, palette.as_ref());
    let border_color = super::node_notation::border_paint(sheet, &element, palette.as_ref());
    let border_width = match sheet.get(&element, "border-width").map(|d| &d.value) {
        Some(Value::Number(value)) => *value,
        _ => 1.0,
    };
    if matches!(
        node.kind,
        NodeKind::StateInitial
            | NodeKind::StateFinal
            | NodeKind::StateChoice
            | NodeKind::ActivityInitial
            | NodeKind::ActivityFinal
            | NodeKind::ActivityFlowFinal
            | NodeKind::ActivityDecision
            | NodeKind::ActivityMerge
            | NodeKind::ActivityFork
            | NodeKind::ActivityJoin
    ) && (border_color == "none" || border_width == 0.0)
    {
        return Err("UML control node requires a visible outline and center".into());
    }
    debug_assert!(matches!(
        node.kind,
        NodeKind::Association
            | NodeKind::Junction
            | NodeKind::Lollipop
            | NodeKind::Socket
            | NodeKind::StateInitial
            | NodeKind::StateFinal
            | NodeKind::StateChoice
            | NodeKind::ActivityInitial
            | NodeKind::ActivityFinal
            | NodeKind::ActivityFlowFinal
            | NodeKind::ActivityDecision
            | NodeKind::ActivityMerge
            | NodeKind::ActivityFork
            | NodeKind::ActivityJoin
    ));
    let label = node.label.clone().unwrap_or_default();
    let vertical_bar = matches!(node.kind, NodeKind::ActivityFork | NodeKind::ActivityJoin)
        && matches!(
            direction,
            FlowDirection::LeftToRight | FlowDirection::RightToLeft
        );
    let base = match node.kind {
        NodeKind::Lollipop | NodeKind::Socket => 20.0,
        NodeKind::StateInitial | NodeKind::ActivityInitial => 18.0,
        NodeKind::ActivityFork | NodeKind::ActivityJoin if vertical_bar => 12.0,
        NodeKind::ActivityFork | NodeKind::ActivityJoin => 72.0,
        _ => 24.0,
    };
    let width = if label.is_empty() {
        base
    } else {
        base + 12.0 + text_width(&label, 14.0)?
    };
    let height = if matches!(node.kind, NodeKind::ActivityFork | NodeKind::ActivityJoin) {
        if vertical_bar { 72.0 } else { 12.0 }
    } else if label.is_empty() {
        base
    } else {
        base.max(14.0 * 1.25 + 8.0)
    };
    Ok(BoxNode {
        id: node.id.clone(),
        label,
        template: None,
        lines: Vec::new(),
        icons: Vec::new(),
        body: Vec::new(),
        body_styles: Vec::new(),
        ports: Vec::new(),
        port_start: 0.0,
        top_band: 0.0,
        bottom_band: 0.0,
        abstract_node: false,
        kind: node.kind,
        is_container: false,
        deployment: false,
        shape: if matches!(
            node.kind,
            NodeKind::Lollipop
                | NodeKind::Socket
                | NodeKind::StateInitial
                | NodeKind::StateFinal
                | NodeKind::ActivityInitial
                | NodeKind::ActivityFinal
                | NodeKind::ActivityFlowFinal
        ) {
            NodeShape::Ellipse
        } else if matches!(node.kind, NodeKind::ActivityFork | NodeKind::ActivityJoin) {
            NodeShape::Rectangle
        } else {
            NodeShape::Diamond
        },
        x: 0.0,
        y: 0.0,
        width,
        height,
        fixed_width: None,
        fixed_height: None,
        font_size: 14.0,
        padding: 0.0,
        fill: if matches!(
            node.kind,
            NodeKind::StateInitial
                | NodeKind::ActivityInitial
                | NodeKind::ActivityFork
                | NodeKind::ActivityJoin
        ) {
            border_color.clone()
        } else {
            fill
        },
        color,
        border_color,
        border_width,
        accent_color: String::new(),
        divider_color: String::new(),
        radius: 0.0,
        note_card: false,
        line: node.line,
    })
}
