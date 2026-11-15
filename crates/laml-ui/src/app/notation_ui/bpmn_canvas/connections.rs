//! Endpoint grips and snapping targets for BPMN flows.
use super::*;
impl Editor {
    pub(super) fn bpmn_connections(&mut self, ui: &mut egui::Ui, rect: egui::Rect) -> bool {
        if self.notation_ui.bpmn.reconnect.is_some() || self.notation_ui.bpmn.connect.is_some() {
            self.bpmn_connection_drag(ui, rect);
            return true;
        }
        let Some(preview) = &self.preview else {
            return false;
        };
        let bounds = preview.bounds;
        let pointer = ui
            .input(|i| i.pointer.hover_pos())
            .filter(|p| ui.clip_rect().contains(*p));
        let mut owned = false;
        let mut start = None;
        let mut select = None;
        for r in preview
            .regions
            .iter()
            .filter(|r| r.kind == "flow" && !r.path.is_empty())
        {
            let Some(index) =
                r.id.strip_prefix("flow:")
                    .and_then(|s| s.parse::<usize>().ok())
            else {
                continue;
            };
            let selected = self.notation_ui.selected.as_ref().is_some_and(|s| {
                s.1.as_deref() == Some(r.id.as_str()) && s.2 == self.document.revision()
            });
            if !selected && !pointer.is_some_and(|p| near(r, p, rect, bounds)) {
                continue;
            }
            for (from, at) in [(true, r.path[0]), (false, *r.path.last().unwrap())] {
                let center = point(rect, bounds, at.0, at.1);
                let response = ui.interact(
                    egui::Rect::from_center_size(center, egui::Vec2::splat(22.0)),
                    ui.scope_id().with(("bpmn-end", self.id, index, from)),
                    egui::Sense::click_and_drag(),
                );
                ui.painter().circle_filled(
                    center,
                    5.5,
                    appearance::color(ui.ctx(), appearance::PANEL),
                );
                ui.painter().circle_stroke(
                    center,
                    5.5,
                    egui::Stroke::new(1.5, appearance::color(ui.ctx(), appearance::ACCENT)),
                );
                if response.hovered() || response.is_pointer_button_down_on() {
                    owned = true;
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
                if response.is_pointer_button_down_on() || response.clicked() {
                    select = Some((r.line, r.id.clone()));
                }
                if response.drag_started() {
                    start = Some((index, from));
                    owned = true;
                }
            }
        }
        if let Some((line, id)) = select {
            self.notation_select(line, Some(id));
        }
        if let Some(start) = start {
            self.notation_ui.bpmn.reconnect = Some(start);
            self.notation_ui.drag = None;
            ui.ctx().request_repaint();
        }
        owned
    }
    fn bpmn_connection_drag(&mut self, ui: &egui::Ui, rect: egui::Rect) {
        self.notation_ui.drag = None;
        let Some(preview) = &self.preview else {
            return;
        };
        let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) else {
            return;
        };
        let Ok(doc) = laml::bpmn::parse(self.document.source()) else {
            return;
        };
        let bounds = preview.bounds;
        let reconnect = self.notation_ui.bpmn.reconnect;
        let from = self.notation_ui.bpmn.connect.as_deref();
        let target = ui
            .clip_rect()
            .contains(pointer)
            .then(|| {
                preview
                    .regions
                    .iter()
                    .filter(|r| r.kind == "node" && near(r, pointer, rect, bounds))
                    .min_by(|a, b| (a.width * a.height).total_cmp(&(b.width * b.height)))
            })
            .flatten();
        let candidate = target.map(|r| {
            if let Some((index, moving_source)) = reconnect {
                edit::bpmn_inline::reconnect(self.document.source(), index, moving_source, &r.id)
            } else {
                edit::bpmn_inline::connect(
                    self.document.source(),
                    from.unwrap_or(""),
                    &r.id,
                    &self.graph.operator,
                )
            }
        });
        let valid = candidate.as_ref().is_some_and(|r| r.is_ok());
        let color = appearance::color(ui.ctx(), appearance::ACCENT);
        for r in preview.regions.iter().filter(|r| r.kind == "node") {
            let eligible = if let Some((index, moving_source)) = reconnect {
                edit::bpmn_inline::reconnect(self.document.source(), index, moving_source, &r.id)
                    .is_ok()
            } else {
                edit::bpmn_inline::connect(
                    self.document.source(),
                    from.unwrap_or(""),
                    &r.id,
                    &self.graph.operator,
                )
                .is_ok()
            };
            if !eligible && target.is_none_or(|t| t.id != r.id) {
                continue;
            }
            let hot = target.is_some_and(|t| t.id == r.id);
            let area = egui::Rect::from_min_max(
                point(rect, bounds, r.x, r.y),
                point(rect, bounds, r.x + r.width, r.y + r.height),
            );
            let tint = if eligible { color } else { egui::Color32::RED };
            ui.painter().rect_filled(
                area,
                3.0,
                tint.gamma_multiply(if hot { 0.22 } else { 0.06 }),
            );
            ui.painter().rect_stroke(
                area.expand(2.0),
                3.0,
                egui::Stroke::new(if hot { 2.5 } else { 1.0 }, tint),
                egui::StrokeKind::Outside,
            );
        }
        let path = reconnect.and_then(|(index, _)| {
            preview
                .regions
                .iter()
                .find(|r| r.id == format!("flow:{index}"))
        });
        let fixed = if let Some((_, moving_source)) = reconnect {
            path.and_then(|r| {
                if moving_source {
                    r.path.last()
                } else {
                    r.path.first()
                }
            })
            .map(|p| point(rect, bounds, p.0, p.1))
        } else {
            preview
                .regions
                .iter()
                .find(|r| Some(r.id.as_str()) == from && r.kind == "node")
                .map(|r| point(rect, bounds, r.x + r.width, r.y + r.height / 2.0))
        };
        let snapped = target
            .map(|r| {
                let area = egui::Rect::from_min_max(
                    point(rect, bounds, r.x, r.y),
                    point(rect, bounds, r.x + r.width, r.y + r.height),
                );
                let candidates = [
                    egui::pos2(area.left(), area.center().y),
                    egui::pos2(area.right(), area.center().y),
                    egui::pos2(area.center().x, area.top()),
                    egui::pos2(area.center().x, area.bottom()),
                ];
                *candidates
                    .iter()
                    .min_by(|a, b| a.distance(pointer).total_cmp(&b.distance(pointer)))
                    .unwrap()
            })
            .unwrap_or(pointer);
        if let Some(fixed) = fixed {
            let (a, b) = if reconnect.is_some_and(|(_, from)| from) {
                (snapped, fixed)
            } else {
                (fixed, snapped)
            };
            let stroke = egui::Stroke::new(1.5, color);
            let kind = reconnect
                .and_then(|(i, _)| doc.edges.get(i))
                .map(|e| e.kind);
            if matches!(
                kind,
                Some(
                    laml::bpmn::EdgeKind::Message
                        | laml::bpmn::EdgeKind::Data
                        | laml::bpmn::EdgeKind::Association
                )
            ) {
                ui.painter()
                    .add(egui::Shape::dashed_line(&[a, b], stroke, 5.0, 4.0));
            } else {
                ui.painter().line_segment([a, b], stroke);
            }
            if kind != Some(laml::bpmn::EdgeKind::Association) {
                let d = (b - a).normalized();
                let n = egui::vec2(-d.y, d.x);
                ui.painter()
                    .line_segment([b, b - d * 9.0 + n * 4.0], stroke);
                ui.painter()
                    .line_segment([b, b - d * 9.0 - n * 4.0], stroke);
            }
            ui.painter().circle_filled(snapped, 4.5, color);
        }
        ui.ctx().set_cursor_icon(if target.is_some() && !valid {
            egui::CursorIcon::NotAllowed
        } else {
            egui::CursorIcon::Crosshair
        });
        if ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary))
            && (reconnect.is_some() || target.is_some())
        {
            // A new connection asks for an alternative rather than silently changing its type.
            let destination = target.map(|r| r.id.clone());
            let origin = from.map(str::to_owned);
            self.notation_ui.bpmn.reconnect = None;
            self.notation_ui.bpmn.connect = None;
            match candidate {
                Some(Ok(edits)) if edits.is_empty() => {}
                Some(Ok(edits)) => {
                    let ends = if let Some((index, moving_source)) = reconnect {
                        doc.edges.get(index).map(|edge| {
                            (
                                if moving_source {
                                    destination.clone().unwrap_or_else(|| edge.from.clone())
                                } else {
                                    edge.from.clone()
                                },
                                if moving_source {
                                    edge.to.clone()
                                } else {
                                    destination.clone().unwrap_or_else(|| edge.to.clone())
                                },
                            )
                        })
                    } else {
                        origin.clone().zip(destination.clone())
                    };
                    let selected = edit::apply(self.document.source(), &edits)
                        .ok()
                        .and_then(|source| laml::bpmn::parse(&source).ok())
                        .and_then(|doc| {
                            let (a, b) = ends?;
                            doc.edges
                                .iter()
                                .enumerate()
                                .rev()
                                .find(|(_, edge)| edge.from == a && edge.to == b)
                                .map(|(i, edge)| (edge.line, format!("flow:{i}")))
                        });
                    self.submit_command(Command {
                        edits,
                        label: "Connect BPMN flow".into(),
                        expectation: worker::Expectation::Notation {
                            line: selected.as_ref().map(|s| s.0),
                            id: selected.map(|s| s.1),
                        },
                    });
                }
                Some(Err(_)) if reconnect.is_none() => {
                    if let (Some(from), Some(to)) = (origin, destination) {
                        self.notation_connect(&from, &to);
                    }
                }
                Some(Err(error)) => self.command_error = Some(error),
                _ => {}
            }
        }
    }
}
