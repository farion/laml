//! Usable default interior space for containers without child nodes.
use super::*;

pub(super) fn size(diagram: &Diagram, boxes: &mut [BoxNode], styles: &StyleSheet) {
    for node in boxes {
        let container = matches!(
            node.kind,
            NodeKind::Package
                | NodeKind::Profile
                | NodeKind::CompositeStructure
                | NodeKind::UseCaseSystem
        ) || node.kind.is_deployment_target()
            || node.c4_boundary
            || node.is_container;
        if !container
            || diagram
                .nodes
                .iter()
                .any(|n| n.parent.as_deref() == Some(&node.id))
        {
            continue;
        }
        let source = diagram.nodes.iter().find(|n| n.id == node.id).unwrap();
        let scoped = scoped_styles(diagram, source, styles);
        let element = crate::styles::Element::Node {
            id: &source.id,
            classes: &source.classes,
        };
        if node.fixed_width.is_none() && scoped.get(&element, "min-width").is_none() {
            node.width = node.width.max(200.0);
        }
        if node.fixed_height.is_none() && scoped.get(&element, "min-height").is_none() {
            let heading = node.padding * 2.0 + node.font_size * 1.25 * node.lines.len() as f64;
            node.height = node.height.max(heading + 100.0);
        }
    }
}
