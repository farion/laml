//! First geometry stage: measure rectangular nodes and satisfy hard rules.
//! Collision resolution is bounded. Complex routes may still fail explicitly.

mod c4_frame;
pub(crate) mod composite;
mod constraints;
mod containers;
mod junctions;
mod lollipop;
mod members;
mod node_notation;
mod nodes;
pub(crate) mod package_outline;
mod port_contours;
mod ports;
mod usecase;
mod visibility;

pub use constraints::GeometryError;
pub(crate) use members::visible_text as member_visible_text;
pub use nodes::{BoxNode, NodeShape};
pub(crate) use visibility::filtered as visible_diagram;

use crate::styles::StyleSheet;
use crate::syntax::{Diagram, NodeDecl, NodeKind};

pub(crate) fn scoped_styles(diagram: &Diagram, node: &NodeDecl, sheet: &StyleSheet) -> StyleSheet {
    let mut parents = Vec::new();
    let mut current = node.parent.as_deref();
    while let Some(id) = current {
        let Some(parent) = diagram.nodes.iter().find(|candidate| candidate.id == id) else {
            break;
        };
        parents.push(parent);
        current = parent.parent.as_deref();
    }
    let mut scoped = sheet.clone();
    for parent in parents.into_iter().rev() {
        scoped = scoped.with_ancestor(&parent.id, &parent.classes);
    }
    scoped
}

pub fn place(diagram: &Diagram, styles: &StyleSheet) -> Result<Vec<BoxNode>, GeometryError> {
    let visible = visible_diagram(diagram, styles)
        .map_err(|(line, message)| GeometryError { line, message })?;
    let diagram = &visible;
    let mut boxes: Vec<_> = diagram
        .nodes
        .iter()
        .map(|node| {
            let scoped = scoped_styles(diagram, node, styles);
            if matches!(
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
            ) {
                junctions::measure(node, &scoped, diagram.settings.direction)
            } else if node.kind == NodeKind::UseCaseActor {
                usecase::actor(node, &scoped)
            } else {
                nodes::measure_node(
                    node,
                    &scoped,
                    diagram.settings.diagram_type,
                    &diagram.port_layout,
                )
            }
            .map_err(|message| GeometryError {
                line: node.line,
                message,
            })
        })
        .collect::<Result<_, _>>()?;
    for (source, drawn) in diagram.nodes.iter().zip(&boxes) {
        if source.classes.iter().any(|class| {
            matches!(
                class.as_str(),
                "c4Person" | "c4System" | "c4Container" | "c4Component" | "c4Boundary"
            )
        }) && source
            .label
            .as_deref()
            .is_some_and(|label| label.split('\n').ne(drawn.lines.iter().map(String::as_str)))
        {
            return Err(GeometryError {
                line: source.line,
                message: "C4 element text would be truncated; increase its size or shorten its description".into(),
            });
        }
        if diagram.settings.diagram_type == crate::syntax::DiagramType::Composite
            && drawn
                .ports
                .iter()
                .any(|port| port.border_color == "none" || port.color == "none")
        {
            return Err(GeometryError {
                line: source.line,
                message: "composite port outline and label must remain visible".into(),
            });
        }
        if (source.kind == NodeKind::ActivityAction
            || diagram.settings.diagram_type == crate::syntax::DiagramType::Composite)
            && source.ports.len() != drawn.ports.len()
        {
            return Err(GeometryError {
                line: source.line,
                message:
                    if diagram.settings.diagram_type == crate::syntax::DiagramType::Composite {
                        "composite ports must remain visible"
                    } else {
                        "activity pins must remain visible"
                    }
                    .into(),
            });
        }
    }
    for node in &mut boxes {
        if matches!(
            diagram.settings.diagram_type,
            crate::syntax::DiagramType::Package | crate::syntax::DiagramType::Profile
        ) && node.kind == NodeKind::Package
            && !diagram
                .nodes
                .iter()
                .any(|child| child.parent.as_deref() == Some(node.id.as_str()))
        {
            node.is_container = false;
            let required =
                14.0 + node.padding * 2.0 + node.font_size * 1.25 * node.lines.len() as f64 + 4.0;
            if node.fixed_height.is_some_and(|height| height < required) {
                return Err(GeometryError {
                    line: node.line,
                    message: "fixed leaf package cannot fit its name in the body".into(),
                });
            }
            node.height = node.height.max(required);
        }
        if (node.kind == NodeKind::Component
            || node.kind == NodeKind::CompositeStructure
            || node.kind == NodeKind::State
            || node.kind == NodeKind::UseCaseSystem
            || node.kind.is_deployment_target())
            && diagram
                .nodes
                .iter()
                .any(|child| child.parent.as_deref() == Some(node.id.as_str()))
        {
            node.is_container = true;
        }
    }
    for relation in &diagram.relations {
        for endpoint in [&relation.from, &relation.to] {
            if !boxes.iter().any(|node| node.has_endpoint(endpoint)) {
                return Err(GeometryError {
                    line: relation.line,
                    message: format!("visible relation targets hidden member '{endpoint}'"),
                });
            }
        }
    }
    for rule in &diagram.port_layout {
        if !boxes
            .iter()
            .any(|node| node.ports.iter().any(|port| port.id == rule.id))
        {
            return Err(GeometryError {
                line: rule.line,
                message: format!("mandatory port layout targets hidden port '{}'", rule.id),
            });
        }
    }
    containers::place(diagram, &mut boxes, styles)?;
    ports::position(diagram, &mut boxes);
    port_contours::validate(&boxes)?;
    Ok(boxes)
}
