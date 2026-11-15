//! Context picking shares the exact visible-route/label geometry with left-click.
use super::*;
use egui_phosphor::regular::{BOX_ARROW_UP, PLUG, PLUS, TRASH};
use laml_ui::delete_edit::ContainerMode;

fn action(ui: &mut egui::Ui, enabled: bool, icon: &str, label: &str) -> egui::Response {
    let color = if enabled {
        appearance::color(ui.ctx(), appearance::TEXT)
    } else {
        appearance::color(ui.ctx(), appearance::MUTED)
    };
    let mut text = egui::text::LayoutJob::default();
    text.append(
        &format!("{icon}  "),
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::new(18.0, egui::FontFamily::Name("phosphor".into())),
            color,
            ..Default::default()
        },
    );
    text.append(
        label,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(15.0),
            color,
            ..Default::default()
        },
    );
    // Natural text width: never stretch the popup to the canvas's available width.
    ui.add_enabled(
        enabled,
        egui::Button::new(text).min_size(egui::vec2(0.0, 32.0)),
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Target {
    Node(String),
    Port(String),
    Relation(usize),
}
pub(super) fn hit(preview: &Preview, canvas: egui::Rect, point: egui::Pos2) -> Option<Target> {
    let (left, top, right, _) = preview.bounds;
    let scale = f64::from(canvas.width()) / (right - left);
    let x = left + f64::from(point.x - canvas.left()) / scale;
    let y = top + f64::from(point.y - canvas.top()) / scale;
    laml_ui::selection::hit_port(&preview.nodes, x, y, (7.0 / scale).max(5.0))
        .map(|id| Target::Port(id.into()))
        .or_else(|| {
            laml_ui::selection::hit_node(&preview.nodes, x, y)
                .map(|node| Target::Node(node.id.clone()))
        })
        .or_else(|| {
            laml_ui::relation_selection::hit(preview, x, y, 7.0 / scale).map(Target::Relation)
        })
}
impl Editor {
    pub(super) fn graph_context_menu(
        &mut self,
        _ui: &egui::Ui,
        canvas: egui::Rect,
        response: &egui::Response,
    ) {
        if response.secondary_clicked() {
            self.graph.context = response.interact_pointer_pos().and_then(|point| {
                self.preview
                    .as_ref()
                    .and_then(|p| hit(p, canvas, point))
                    .map(|target| (target, point, self.document.revision()))
            });
        }
        let context = self.graph.context.clone();
        let enabled = self.can_edit_visually()
            && self.graph.gesture.is_none()
            && !self.graph.dialog_open()
            && self.deletion.is_none()
            && context
                .as_ref()
                .is_none_or(|(_, _, revision)| *revision == self.document.revision());
        response.context_menu(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(16.0, 6.0);
            ui.spacing_mut().interact_size.y = 32.0;
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.set_min_width(0.0);
            if action(ui, enabled, PLUS, "New node…").clicked() {
                self.open_node_dialog();
                ui.close();
            }
            ui.add(egui::Separator::default().spacing(8.0));
            if let Some((target, point, _)) = &context {
                if let Target::Node(id) = target {
                    let node = self
                        .preview
                        .as_ref()
                        .and_then(|p| p.nodes.iter().find(|n| &n.id == id).map(|n| (p, n)));
                    let side = node.map(|(p, n)| {
                        connection_geometry::nearest_side(
                            connection_geometry::node_rect(p, canvas, n),
                            *point,
                        )
                    });
                    let ports = self.preview.as_ref().is_some_and(|p| {
                        p.diagram.nodes.iter().any(|n| {
                            &n.id == id
                                && matches!(
                                    n.kind,
                                    laml::syntax::NodeKind::Generic | laml::syntax::NodeKind::Class
                                )
                        })
                    });
                    if action(ui, enabled && ports, PLUG, "Add port…").clicked() {
                        self.open_port_dialog(id, side.unwrap_or("right"));
                        ui.close();
                    }
                }
                let label = match target {
                    Target::Node(_) => "Delete this node…",
                    Target::Port(_) => "Delete this port…",
                    Target::Relation(_) => "Delete this arrow…",
                };
                if action(ui, enabled, TRASH, label).clicked() {
                    match target {
                        Target::Node(id) => self.graph_delete(vec![id.clone()]),
                        Target::Port(id) => self.propose_delete_targets(
                            vec![],
                            vec![],
                            vec![id.clone()],
                            ContainerMode::WithContents,
                        ),
                        Target::Relation(line) => self.propose_delete_targets(
                            vec![],
                            vec![*line],
                            vec![],
                            ContainerMode::WithContents,
                        ),
                    }
                    ui.close();
                }
                ui.add(egui::Separator::default().spacing(8.0));
            }
            if action(
                ui,
                enabled && self.has_selection(),
                TRASH,
                "Delete selection…",
            )
            .clicked()
            {
                self.propose_delete(ContainerMode::WithContents);
                ui.close();
            }
            if self.selected_container()
                && action(
                    ui,
                    enabled,
                    BOX_ARROW_UP,
                    "Remove container, keep contents…",
                )
                .clicked()
            {
                self.propose_delete(ContainerMode::KeepContents);
                ui.close();
            }
        });
    }
}
