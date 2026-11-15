//! Diagram-space node hit testing and ordered multi-selection.
pub fn hit_port(nodes: &[laml::geometry::BoxNode], x: f64, y: f64, tolerance: f64) -> Option<&str> {
    nodes
        .iter()
        .flat_map(|n| &n.ports)
        .filter_map(|p| {
            let distance = (p.x - x).hypot(p.y - y);
            (distance <= tolerance).then_some((p.id.as_str(), distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

use laml::geometry::{BoxNode, NodeShape};

pub fn hit_node(nodes: &[BoxNode], x: f64, y: f64) -> Option<&BoxNode> {
    nodes
        .iter()
        .filter(|node| {
            let nx = (x - node.x) / node.width;
            let ny = (y - node.y) / node.height;
            if !(0.0..=1.0).contains(&nx) || !(0.0..=1.0).contains(&ny) {
                return false;
            }
            match node.shape {
                NodeShape::Ellipse => (nx - 0.5).powi(2) + (ny - 0.5).powi(2) <= 0.25,
                NodeShape::Diamond => (nx - 0.5).abs() + (ny - 0.5).abs() <= 0.5,
                _ => true,
            }
        })
        .min_by(|a, b| (a.width * a.height).total_cmp(&(b.width * b.height)))
}

#[derive(Default)]
pub struct Selection {
    pub ids: Vec<String>,
    pub relations: Vec<usize>,
    pub ports: Vec<String>,
}

impl Selection {
    pub fn click(&mut self, id: Option<&str>, toggle: bool) {
        if !toggle {
            self.ids.clear();
            self.relations.clear();
            self.ports.clear();
        }
        if let Some(id) = id {
            if let Some(index) = self.ids.iter().position(|selected| selected == id) {
                self.ids.remove(index);
            } else {
                self.ids.push(id.into());
            }
        }
    }
    pub fn click_relation(&mut self, line: usize, toggle: bool) {
        if !toggle {
            self.ids.clear();
            self.relations.clear();
            self.ports.clear();
        }
        if let Some(index) = self.relations.iter().position(|selected| *selected == line) {
            self.relations.remove(index);
        } else {
            self.relations.push(line);
        }
    }
    pub fn click_port(&mut self, id: &str, toggle: bool) {
        if !toggle {
            self.ids.clear();
            self.relations.clear();
            self.ports.clear();
        }
        if let Some(index) = self.ports.iter().position(|selected| selected == id) {
            self.ports.remove(index);
        } else {
            self.ports.push(id.into());
        }
    }
    pub fn retain_diagram(&mut self, diagram: &laml::syntax::Diagram) {
        self.ids
            .retain(|id| diagram.nodes.iter().any(|n| &n.id == id));
        self.relations.retain(|line| {
            diagram.relations.iter().any(|r| r.line == *line)
                || diagram.attachments.iter().any(|a| a.line == *line)
        });
        self.ports
            .retain(|id| crate::port_edit::find(diagram, id).is_some());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn last_selected_is_reference_and_toggle_deselects() {
        let mut selection = Selection::default();
        selection.click(Some("A"), false);
        selection.click(Some("B"), true);
        assert_eq!(selection.ids, ["A", "B"]);
        selection.click(Some("A"), true);
        assert_eq!(selection.ids, ["B"]);
        selection.click(None, false);
        assert!(selection.ids.is_empty());
    }
    #[test]
    fn compiler_boxes_are_hit_tested_with_real_shape() {
        let source = "semantics {\n A\n}\nstyling {\n node[A] {\n shape: ellipse\n }\n}\n";
        let preview = crate::preview::build(source, std::path::Path::new("test.laml")).unwrap();
        let node = &preview.nodes[0];
        assert!(hit_node(&preview.nodes, node.x, node.y).is_none());
        assert_eq!(
            hit_node(
                &preview.nodes,
                node.x + node.width / 2.0,
                node.y + node.height / 2.0
            )
            .unwrap()
            .id,
            "A"
        );
    }
}
