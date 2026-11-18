//! Graph-only connection affordances, moving arrows and connection handles.
use super::*;
use connection_geometry as geometry;
use connection_gesture::Kind;
impl Editor {
    pub(super) fn connection_overlay(
        &mut self,
        ui: &egui::Ui,
        canvas: egui::Rect,
        navigation: bool,
    ) {
        let Some(preview) = &self.preview else {
            return;
        };
        let pointer = ui
            .input(|i| i.pointer.hover_pos())
            .filter(|p| ui.clip_rect().contains(*p));
        if let Some(gesture) = &self.graph.gesture {
            let Some(pointer) = pointer else {
                return;
            };
            let target =
                geometry::target_at(preview, canvas, pointer, ui.input(|i| i.modifiers.shift));
            for node in &preview.nodes {
                for handle in geometry::handles(preview, canvas, node, false) {
                    let hot = target
                        .as_ref()
                        .is_some_and(|t| t.id == handle.dock.id && t.side == handle.dock.side);
                    ui.painter().circle_filled(
                        handle.point,
                        handle.radius * if hot { 1.15 } else { 1.0 },
                        if hot {
                            egui::Color32::LIGHT_GREEN
                        } else {
                            appearance::color(ui.ctx(), appearance::PANEL)
                        },
                    );
                    ui.painter().circle_stroke(
                        handle.point,
                        handle.radius * if hot { 1.15 } else { 1.0 },
                        egui::Stroke::new(1.3, appearance::color(ui.ctx(), appearance::SELECTED)),
                    );
                }
                for port in &node.ports {
                    let point = geometry::screen(preview, canvas, port.x, port.y);
                    let hot = target.as_ref().is_some_and(|t| t.id == port.id);
                    ui.painter().circle_stroke(
                        point,
                        geometry::radius(preview, canvas, node) * if hot { 0.85 } else { 0.72 },
                        egui::Stroke::new(
                            2.0,
                            if hot {
                                egui::Color32::LIGHT_GREEN
                            } else {
                                appearance::color(ui.ctx(), appearance::SELECTED)
                            },
                        ),
                    );
                }
                if target
                    .as_ref()
                    .is_some_and(|t| t.owner == node.id && t.id == node.id && t.side.is_none())
                {
                    ui.painter().rect_stroke(
                        geometry::node_rect(preview, canvas, node).expand(3.0),
                        4,
                        egui::Stroke::new(1.5, egui::Color32::LIGHT_GREEN),
                        egui::StrokeKind::Outside,
                    );
                }
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            let moving_source = matches!(
                gesture.kind,
                Kind::Reconnect {
                    end: laml::syntax::EndPosition::Source,
                    ..
                }
            );
            let fixed = if moving_source {
                gesture.end
            } else {
                gesture.start
            };
            let moving = target.as_ref().map_or(pointer, |dock| {
                if dock.id == dock.owner && dock.side.is_none() {
                    let toward = if let Kind::New { from, .. } = &gesture.kind
                        && from.owner == dock.owner
                    {
                        let node = preview.nodes.iter().find(|n| n.id == dock.owner).unwrap();
                        let center = geometry::node_rect(preview, canvas, node).center();
                        center
                            + if from.side == Some("bottom") {
                                egui::vec2(1.0, 0.0)
                            } else {
                                egui::vec2(0.0, 1.0)
                            }
                    } else {
                        fixed
                    };
                    geometry::automatic_anchor(preview, canvas, &dock.id, toward)
                } else {
                    dock.point
                }
            });
            let (mut start, end) = if moving_source {
                (moving, gesture.end)
            } else {
                (gesture.start, moving)
            };
            let mut source_direction = gesture.start_direction;
            let mut target_direction = gesture.end_direction;
            if let Kind::New { from, .. } = &gesture.kind {
                if from.id == from.owner && from.side.is_none() {
                    let toward = if target.as_ref().is_some_and(|t| t.owner == from.owner) {
                        let node = preview.nodes.iter().find(|n| n.id == from.owner).unwrap();
                        geometry::node_rect(preview, canvas, node).center() + egui::vec2(1.0, 0.0)
                    } else {
                        moving
                    };
                    start = geometry::automatic_anchor(preview, canvas, &from.id, toward);
                }
                source_direction = geometry::dock_direction(preview, canvas, from, start);
            }
            let moving_direction = target.as_ref().map_or(egui::Vec2::ZERO, |dock| {
                geometry::dock_direction(preview, canvas, dock, moving)
            });
            if moving_source {
                source_direction = moving_direction;
            } else {
                target_direction = moving_direction;
            }
            connection_preview::paint(
                ui,
                preview,
                connection_preview::Rendering {
                    presentation: self.compiler.presentation,
                    source: self.document.source(),
                    path: &self.path,
                    service: &mut self.remote,
                },
                self.id,
                gesture,
                connection_preview::Frame {
                    canvas,
                    viewport: ui.clip_rect(),
                    start,
                    end,
                    source_direction,
                    target_direction,
                    target,
                },
                &mut self.graph.motion,
            );
            return;
        }
        if navigation
            || !self.can_edit_visually()
            || self.graph.dialog_open()
            || self.placement.drag.is_some()
            || self.creation.connect
        {
            return;
        }
        let hovered = self
            .graph_control_node(preview, canvas, pointer)
            .map(|n| n.id.clone());
        self.graph.hover_node = hovered.clone();
        if let Some(node) = hovered.and_then(|id| preview.nodes.iter().find(|n| n.id == id)) {
            for handle in geometry::handles(preview, canvas, node, true) {
                let hot = pointer.is_some_and(|p| p.distance(handle.point) <= handle.radius);
                ui.painter().circle_filled(
                    handle.point,
                    handle.radius,
                    if hot {
                        appearance::color(ui.ctx(), appearance::ACCENT)
                    } else {
                        appearance::color(ui.ctx(), appearance::PANEL)
                    },
                );
                ui.painter().circle_stroke(
                    handle.point,
                    handle.radius,
                    egui::Stroke::new(1.0, appearance::color(ui.ctx(), appearance::SELECTED)),
                );
                if hot {
                    ui.painter().circle_stroke(
                        handle.point,
                        handle.radius + 4.0,
                        egui::Stroke::new(2.0, appearance::color(ui.ctx(), appearance::ACCENT)),
                    );
                }
                if hot {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                    egui::Tooltip::always_open(
                        ui.ctx().clone(),
                        ui.layer_id(),
                        egui::Id::unique(("connection-tip", self.id)),
                        handle.point,
                    )
                    .show(|ui| {
                        let title = crate::i18n::tr(if handle.dock.side.is_none() {
                            "Drag a connection · automatic source side"
                        } else {
                            "Drag a connection · fixed sides"
                        });
                        hover_help::show(
                            ui,
                            &format!(
                                "{title}\n{}\n{}",
                                crate::i18n::tr("Shift when starting: automatic source side"),
                                crate::i18n::tr("Shift when releasing: automatic target side")
                            ),
                        );
                    });
                }
            }
        }
    }
}
