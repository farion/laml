use super::BoxNode;
use crate::syntax::NodeKind;

impl BoxNode {
    pub fn contains_package_content(&self, x: f64, y: f64, width: f64, height: f64) -> bool {
        self.is_container
            && x >= self.x + 1.0
            && x + width <= self.x + self.width - 1.0
            && y >= self.y
                + if self.c4_boundary {
                    self.padding + 30.0
                } else if !matches!(self.kind, NodeKind::Package | NodeKind::Profile) {
                    self.port_start
                } else {
                    self.padding + self.font_size * 1.25 * self.lines.len() as f64 + 4.0
                }
            && y + height <= self.y + self.height - 1.0
    }
}
