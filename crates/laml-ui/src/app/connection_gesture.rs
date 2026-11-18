//! Revision-bound graph gestures. Hit testing and painting are kept separate.
use super::*;
use connection_geometry::{self as geometry, Dock};
use laml::syntax::EndPosition;
#[cfg(test)]
mod tests;

#[derive(Clone)]
pub(super) enum Kind {
    New { from: Dock, operator: String },
    Reconnect { line: usize, end: EndPosition },
}
#[derive(Clone)]
pub(super) struct Gesture {
    pub revision: u64,
    pub canvas: egui::Rect,
    pub kind: Kind,
    pub start: egui::Pos2,
    pub end: egui::Pos2,
    pub start_direction: egui::Vec2,
    pub end_direction: egui::Vec2,
    pub route: laml::route::Route,
}

impl Editor {
    pub(super) fn connection_input(
        &mut self,
        ui: &egui::Ui,
        canvas: egui::Rect,
        response: &egui::Response,
        navigation: bool,
    ) -> bool {
        if self.specialized() {
            return false;
        }
        let was_active = self.graph.gesture.is_some();
        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let stale = self
            .graph
            .gesture
            .as_ref()
            .is_some_and(|g| g.revision != self.document.revision() || g.canvas != canvas);
        if navigation || escape || stale || !self.can_edit_visually() || self.graph.dialog_open() {
            self.end_connection(ui.ctx());
            return was_active || self.graph.dialog_open();
        }
        if let Some(shift) = ui.input(|i| {
            i.events.iter().find_map(|event| match event {
                egui::Event::PointerButton {
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers,
                    ..
                } => Some(modifiers.shift),
                _ => None,
            })
        }) {
            self.graph.connection_press_shift = shift;
        }
        if self.graph.gesture.is_none()
            && !self.creation.connect
            && response.drag_started_by(egui::PointerButton::Primary)
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            let preview = self.preview.as_ref().unwrap();
            let endpoint = geometry::endpoint_at(preview, canvas, origin)
                .filter(|(line, _)| {
                    !self.responsive.touch || self.selection.relations.contains(line)
                })
                .and_then(|(line, end)| {
                    preview
                        .routes
                        .iter()
                        .find(|r| r.source_line == line)
                        .map(|r| (r, end))
                });
            let gesture = if let Some((route, end)) = endpoint {
                let source =
                    laml_ui::relation_selection::endpoint(route, EndPosition::Source).unwrap();
                let target =
                    laml_ui::relation_selection::endpoint(route, EndPosition::Target).unwrap();
                let gesture = Gesture {
                    revision: self.document.revision(),
                    canvas,
                    kind: Kind::Reconnect {
                        line: route.source_line,
                        end,
                    },
                    start: geometry::screen(preview, canvas, source.x, source.y),
                    end: geometry::screen(preview, canvas, target.x, target.y),
                    start_direction: route
                        .points
                        .get(1)
                        .map_or(egui::Vec2::ZERO, |p| {
                            geometry::screen(preview, canvas, p.x, p.y)
                                - geometry::screen(preview, canvas, source.x, source.y)
                        })
                        .normalized(),
                    end_direction: route
                        .points
                        .get(route.points.len().saturating_sub(2))
                        .map_or(egui::Vec2::ZERO, |p| {
                            geometry::screen(preview, canvas, p.x, p.y)
                                - geometry::screen(preview, canvas, target.x, target.y)
                        })
                        .normalized(),
                    route: route.clone(),
                };
                let line = route.source_line;
                match self.request_gesture_base(line) {
                    Ok(()) => {
                        self.relation_drag = Some((line, end, self.document.revision()));
                        Some(gesture)
                    }
                    Err(e) => {
                        self.graph.report_errors = true;
                        self.command_error = Some(e);
                        None
                    }
                }
            } else if let Some(from) = geometry::source_at(preview, canvas, origin).filter(|dock| {
                !self.responsive.touch || self.selection.ids.last() == Some(&dock.owner)
            }) {
                let from = geometry::attachment(from, self.graph.connection_press_shift);
                let operator = self.graph.operator.clone();
                let draft_operator = self
                    .notation()
                    .relation(&operator)
                    .map_or(operator.as_str(), |r| r.operator);
                let draft_operator = if matches!(draft_operator, "==" | "=>") {
                    "--"
                } else {
                    draft_operator
                };
                let parsed = laml::parser::parse(&format!(
                    "semantics {{\n A\n B\n A {draft_operator} B\n}}\n"
                ));
                parsed.ok().map(|parsed| Gesture {
                    revision: self.document.revision(),
                    canvas,
                    start: from.point,
                    end: origin,
                    start_direction: from.side.map_or(egui::Vec2::ZERO, geometry::direction),
                    end_direction: egui::Vec2::ZERO,
                    route: laml_ui::gesture_preview::draft_route(preview, &parsed.relations[0]),
                    kind: Kind::New { from, operator },
                })
            } else {
                None
            };
            if let Some(gesture) = gesture {
                self.graph.gesture = Some(gesture);
                self.graph.report_errors = true;
                self.command_error = None;
                self.placement.cancel();
                self.creation.drag = None;
            }
        }
        let Some(gesture) = self.graph.gesture.clone() else {
            return false;
        };
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            let automatic = ui.input(|i| {
                i.events
                    .iter()
                    .find_map(|event| match event {
                        egui::Event::PointerButton {
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            modifiers,
                            ..
                        } => Some(modifiers.shift),
                        _ => None,
                    })
                    .unwrap_or(i.modifiers.shift)
            });
            let target = ui
                .input(|i| i.pointer.interact_pos())
                .filter(|p| ui.clip_rect().contains(*p))
                .and_then(|p| {
                    geometry::target_at(self.preview.as_ref().unwrap(), canvas, p, automatic)
                });
            self.end_connection(ui.ctx());
            if let Some(target) = target {
                self.commit_connection(&gesture, &target);
            }
        } else if !ui.input(|i| i.pointer.primary_down()) {
            self.end_connection(ui.ctx());
        }
        true
    }

    pub(super) fn end_connection(&mut self, ctx: &egui::Context) {
        if let Some((uri, _)) = &self.graph.drag_image {
            ctx.forget_image(uri);
        }
        if !self.graph.motion.uri.is_empty() {
            ctx.forget_image(&self.graph.motion.uri);
        }
        self.remote.cancel();
        self.graph.cancel_gesture();
        self.relation_drag = None;
    }
    pub(super) fn commit_connection(&mut self, gesture: &Gesture, target: &Dock) {
        if gesture.revision != self.document.revision() || !self.can_edit_visually() {
            return;
        }
        self.graph.report_errors = true;
        if let Kind::New { from, operator } = &gesture.kind {
            self.connect_notation_default(&from.id, &target.id, operator, from.side, target.side);
            return;
        }
        let result = match &gesture.kind {
            Kind::New { from, operator } => laml_ui::graph_edit::add_relation(
                self.document.source(),
                &from.id,
                &target.id,
                operator,
                from.side,
                target.side,
            ),
            Kind::Reconnect { line, end } => laml_ui::graph_edit::reconnect(
                self.document.source(),
                *line,
                *end,
                &target.id,
                target.side,
            ),
        };
        match result {
            Ok((id, edits)) => self.submit_command(Command {
                edits,
                label: match gesture.kind {
                    Kind::New { .. } => "Add relation".into(),
                    Kind::Reconnect { .. } => "Reconnect relation".into(),
                },
                expectation: worker::Expectation::Relation(id),
            }),
            Err(e) => self.command_error = Some(e),
        }
    }
}
