//! Source-derived container interior shared by editor insertion and drop feedback.
use super::BoxNode;
use crate::syntax::NodeKind;

impl BoxNode {
    /// Interior below the rendered heading, excluding frame strokes and cube faces.
    /// Coordinates are in the compiler's diagram coordinate system.
    pub fn container_body(&self) -> Option<(f64, f64, f64, f64)> {
        if !self.is_container && !matches!(self.kind, NodeKind::Package | NodeKind::Profile) {
            return None;
        }
        let step = self.font_size * 1.25 * self.lines.len() as f64;
        let header = if self.c4_boundary {
            self.padding + 30.0
        } else if matches!(self.kind, NodeKind::Package | NodeKind::Profile) {
            let (_, tab) = super::package_outline::tab(self).ok()?;
            if self.is_container {
                tab
            } else {
                tab + self.padding + step + 4.0
            }
        } else if self.kind == NodeKind::CompositeStructure {
            super::composite::header_height(self) + 30.0
        } else {
            self.port_start.max(self.top_band + self.padding + step)
        };
        let inset = self.border_width.max(1.0);
        let x = self.x + inset;
        let y = self.y + header + inset;
        let right = self.x + self.width
            - inset
            - if self.kind.is_deployment_target() {
                14.0
            } else {
                0.0
            };
        let bottom = self.y + self.height - inset;
        (right > x && bottom > y).then_some((x, y, right, bottom))
    }
}
