//! Empty-container insertion appears while hovering the owning container.
use super::*;

pub(super) fn centers(preview: &Preview, canvas: egui::Rect) -> Vec<(String, egui::Pos2)> {
    preview
        .nodes
        .iter()
        .filter(|n| {
            (n.is_container
                || n.c4_boundary
                || matches!(
                    n.kind,
                    laml::syntax::NodeKind::Package
                        | laml::syntax::NodeKind::Profile
                        | laml::syntax::NodeKind::CompositeStructure
                        | laml::syntax::NodeKind::UseCaseSystem
                )
                || n.kind.is_deployment_target())
                && !preview
                    .diagram
                    .nodes
                    .iter()
                    .any(|c| c.parent.as_deref() == Some(&n.id))
        })
        .filter_map(|n| {
            let (x, y, right, bottom) = n.container_body()?;
            Some((
                n.id.clone(),
                connection_geometry::screen(preview, canvas, (x + right) / 2.0, (y + bottom) / 2.0),
            ))
        })
        .collect()
}
pub(super) fn hover_centers(
    preview: &Preview,
    canvas: egui::Rect,
    pointer: Option<egui::Pos2>,
) -> Vec<(String, egui::Pos2)> {
    let hovered = pointer.and_then(|p| connection_geometry::hit_node(preview, canvas, p));
    centers(preview, canvas)
        .into_iter()
        .filter(|(id, _)| hovered.is_some_and(|n| n.id == *id))
        .collect()
}
impl Editor {
    pub(super) fn empty_container_pluses(&mut self, ui: &mut egui::Ui, canvas: egui::Rect) {
        let pointer = ui
            .input(|i| i.pointer.hover_pos())
            .filter(|p| ui.clip_rect().contains(*p));
        let preview = self.preview.as_ref().unwrap();
        let positions = if self.responsive.touch {
            centers(preview, canvas)
                .into_iter()
                .filter(|(id, _)| self.selection.ids.last() == Some(id))
                .collect()
        } else {
            hover_centers(preview, canvas, pointer)
        };
        for (from, at) in positions {
            let r = ui.interact(
                egui::Rect::from_center_size(at, egui::Vec2::splat(24.0)),
                ui.make_persistent_id(("empty-container-plus", &from)),
                egui::Sense::click(),
            );
            icon(ui, at, r.hovered());
            if r.clicked() {
                let choices = graph_insert::items(self.document.source())
                    .into_iter()
                    .filter(|item| {
                        graph_insert::plan_inside(self.document.source(), &from, *item).is_ok()
                    })
                    .collect();
                self.graph.insertion = Some(Menu {
                    revision: self.document.revision(),
                    from,
                    side: Placement::Right,
                    at,
                    expanded: false,
                    inside: true,
                    choices,
                });
            }
        }
    }
}
