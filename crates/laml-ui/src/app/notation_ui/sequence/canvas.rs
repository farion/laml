//! Dedicated sequence canvas: never falls through into generic gestures.
use super::*;
impl Editor {
    pub(in crate::app) fn sequence_canvas(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        response: &egui::Response,
        navigation: bool,
    ) {
        let revision = self.document.revision();
        if self.notation_ui.sequence.revision != Some(revision)
            || !self.can_edit_visually()
            || ui.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.notation_ui.sequence = State {
                revision: Some(revision),
                ..Default::default()
            };
        }
        self.notation_ui.drag = None;
        let Some(preview) = &self.preview else {
            return;
        };
        let scene = Scene::new(preview, rect);
        // Selection is persistent, not a hover control: paint it during navigation too.
        if self.notation_ui.sequence.gesture.is_none()
            && let Some((line, id, _)) = &self.notation_ui.selected
            && let Some(message) = scene.regions.iter().find(|r| {
                r.kind == "message"
                    && !r.path.is_empty()
                    && r.line == *line
                    && id.as_ref().is_none_or(|id| id == &r.id)
            })
        {
            painting::selected_message(ui, &scene, message);
        }
        if navigation || !self.can_edit_visually() || dialogs::blocked(ui.ctx()) {
            return;
        }
        if scene.heads.is_empty() {
            let at = ui.clip_rect().center();
            let button = ui.put(
                egui::Rect::from_center_size(at, egui::vec2(190.0, 32.0)),
                egui::Button::new(crate::i18n::tr("Add participant…")),
            );
            if button.clicked() {
                self.sequence_add_default(None, None);
            }
        }
        let pointer = ui
            .input(|i| i.pointer.hover_pos())
            .filter(|p| ui.clip_rect().contains(*p));
        let hit = pointer.and_then(|p| {
            if !self.responsive.touch {
                return scene.hit(p);
            }
            let anchor = self
                .notation_ui
                .sequence
                .touch_anchor
                .map(|(x, y)| scene.point(x, y));
            let anchored_hit = anchor.and_then(|p| scene.hit(p));
            scene.hit_filtered(p, |area| {
                matches!(
                    area.hit,
                    Hit::Participant(_) | Hit::Activation(_) | Hit::Event(_) | Hit::Destruction(_)
                ) || anchor.is_some_and(|p| {
                    painting::control_visible(&scene, area, p, anchored_hit.as_ref())
                })
            })
        });
        if ui.input(|i| i.pointer.secondary_pressed()) && response.contains_pointer() {
            self.notation_ui.sequence.press = None;
            self.notation_ui.sequence.gesture = None;
            self.notation_ui.sequence.context = hit.clone();
            if let Some(hit) = &hit {
                self.sequence_select_hit(hit);
            }
        }
        if self.sequence_pick_reference(ui, &scene, pointer, response) {
            return;
        }
        self.sequence_context(ui, response);
        if self.notation_ui.sequence.insert.is_some() {
            self.sequence_insert_menu(ui, &scene);
            return;
        }
        if self.notation_ui.sequence.gesture.is_some() || self.notation_ui.sequence.press.is_some()
        {
            self.sequence_gesture(ui, &scene);
        }
        if self.notation_ui.sequence.gesture.is_some()
            || self.notation_ui.sequence.insert.is_some()
            || self.notation_ui.sequence.alternatives.is_some()
            || self.notation_ui.sequence.picker.is_some()
        {
            return;
        }
        if ui.input(|i| i.pointer.primary_pressed()) && response.contains_pointer() {
            if self.responsive.touch {
                let scale = f64::from(rect.width()) / (scene.bounds.2 - scene.bounds.0);
                self.notation_ui.sequence.touch_anchor =
                    pointer.filter(|_| hit.is_some()).map(|p| {
                        (
                            scene.bounds.0 + f64::from(p.x - rect.left()) / scale,
                            scene.bounds.1 + f64::from(p.y - rect.top()) / scale,
                        )
                    });
            }
            if let Some(hit) = hit.clone() {
                self.sequence_select_hit(&hit);
                self.notation_ui.sequence.press = pointer.map(|p| (hit, p));
            } else {
                self.notation_ui.selected = None;
                self.notation_ui.sequence.clear_selection();
            }
        }
        if ui.input(|i| i.pointer.primary_released())
            && let Some((pressed, origin)) = self.notation_ui.sequence.press.take()
            && pointer.is_some_and(|p| p.distance(origin) < 5.0)
        {
            match pressed {
                Hit::ParticipantPlus { neighbor, left } => {
                    self.sequence_add_default(Some((neighbor, left)), None)
                }
                Hit::Insert(point) => {
                    self.notation_ui.sequence.expanded = false;
                    self.notation_ui.sequence.insert = Some(point);
                }
                Hit::Destroy { owner, before } => {
                    match controls::destroy(self.document.source(), &owner, before) {
                        Ok(edits) => self.sequence_apply(Proposal {
                            edits,
                            line: None,
                            label: "Destroy participant",
                        }),
                        Err(e) => self.command_error = Some(e),
                    }
                }
                _ => {}
            }
        }
        let controls = if self.responsive.touch {
            self.notation_ui
                .sequence
                .touch_anchor
                .map(|(x, y)| scene.point(x, y))
        } else {
            pointer
        };
        if let Some(point) = controls {
            let control_hit = scene.hit(point);
            painting::hover(ui, &scene, point, control_hit.as_ref());
        }
        self.sequence_insert_menu(ui, &scene);
    }
    fn sequence_select_hit(&mut self, hit: &Hit) {
        let selected = match hit {
            Hit::Participant(r) | Hit::Event(r) | Hit::Destruction(r) => {
                Some((r.line, r.id.clone()))
            }
            Hit::Endpoint { message, .. } => Some((message.line, message.id.clone())),
            Hit::Activation(bar) | Hit::Resize(bar, _) => {
                Some((bar.region.line, format!("line:{}", bar.region.line)))
            }
            _ => None,
        };
        if let Some((line, id)) = selected {
            self.notation_select(line, Some(id));
            self.notation_ui.sequence.selection = Some(hit.clone());
        }
    }
}
