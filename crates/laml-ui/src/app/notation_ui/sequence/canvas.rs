use super::*;
use laml::authoring::Region;

pub(super) fn point(rect: egui::Rect, bounds: (f64, f64, f64, f64), x: f64, y: f64) -> egui::Pos2 {
    let scale = f64::from(rect.width()) / (bounds.2 - bounds.0);
    rect.min
        + egui::vec2(
            ((x - bounds.0) * scale) as f32,
            ((y - bounds.1) * scale) as f32,
        )
}
fn anchor_at(regions: &[Region], x: f64, y: f64, tolerance: f64) -> Option<Anchor> {
    let heads: Vec<_> = regions
        .iter()
        .filter(|r| r.kind == "participant" && !r.id.contains('.'))
        .collect();
    if let Some(head) = heads.iter().find(|r| {
        x >= r.x - tolerance
            && x <= r.x + r.width + tolerance * 2.0
            && y >= r.y - tolerance
            && y <= r.y + r.height
    }) {
        let right = heads
            .iter()
            .filter(|r| r.x > head.x)
            .min_by(|a, b| a.x.total_cmp(&b.x))
            .map(|r| r.id.clone());
        return Some(Anchor {
            owner: head.id.clone(),
            before: 0,
            x: head.x + head.width,
            y: head.y + head.height / 2.0,
            kind: Kind::Head,
            end_line: None,
            right,
            activation_line: None,
        });
    }
    let end = regions.iter().find(|r| {
        r.kind == "lifeline-end"
            && !r.id.contains('.')
            && (x - r.x - 5.0).abs() < tolerance
            && (y - r.y - 5.0).abs() < tolerance
    });
    if let Some(end) = end {
        return Some(Anchor {
            owner: end.id.clone(),
            before: 0,
            x: end.x + 5.0,
            y: end.y + 5.0,
            kind: Kind::End,
            end_line: None,
            right: None,
            activation_line: None,
        });
    }
    let line = regions
        .iter()
        .filter(|r| {
            r.kind == "lifeline"
                && !r.id.contains('.')
                && (x - r.x).abs() < tolerance
                && y >= r.y
                && y <= r.y + r.height
        })
        .min_by(|a, b| (x - a.x).abs().total_cmp(&(x - b.x).abs()));
    let execution = regions
        .iter()
        .filter(|r| {
            r.kind == "sequence-execution"
                && !r.id.contains('.')
                && x >= r.x - tolerance
                && x <= r.x + r.width + tolerance
                && y >= r.y
                && y <= r.y + r.height
        })
        .min_by(|a, b| {
            (x - a.x - a.width / 2.0)
                .abs()
                .total_cmp(&(x - b.x - b.width / 2.0).abs())
        });
    let owner = execution.or(line)?;
    let slot = regions
        .iter()
        .filter(|r| r.kind == "sequence-slot" && r.y >= owner.y && r.y <= owner.y + owner.height)
        .min_by(|a, b| (a.y - y).abs().total_cmp(&(b.y - y).abs()))?;
    Some(Anchor {
        owner: owner.id.clone(),
        before: slot.line,
        x: if execution.is_some() {
            owner.x + owner.width
        } else {
            owner.x
        },
        y: slot.y,
        kind: if execution.is_some() {
            Kind::Execution
        } else {
            Kind::Line
        },
        end_line: None,
        right: None,
        activation_line: None,
    })
}
impl Editor {
    pub(in crate::app) fn sequence_canvas(&mut self, ui: &mut egui::Ui, rect: egui::Rect) -> bool {
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
        if !self.can_edit_visually() {
            return false;
        }
        if dialogs::blocked(ui.ctx()) {
            return true;
        }
        let Some(preview) = &self.preview else {
            return false;
        };
        let bounds = preview.bounds;
        let regions = preview.regions.clone();
        let scale = f64::from(rect.width()) / (bounds.2 - bounds.0);
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let local = |p: egui::Pos2| {
            (
                bounds.0 + f64::from(p.x - rect.left()) / scale,
                bounds.1 + f64::from(p.y - rect.top()) / scale,
            )
        };
        if self.sequence_participants(ui, rect) {
            return true;
        }
        if self.sequence_edge_drag(ui, rect) {
            return true;
        }
        if self.sequence_destructions(ui, rect) {
            return true;
        }
        if self.sequence_connections(ui, rect) {
            return true;
        }
        if self.sequence_body_drag(ui, rect) {
            return true;
        }
        if self.notation_ui.sequence.dialog.is_some() {
            return true;
        }
        if !self.notation_ui.sequence.menu_open {
            let retained = self.notation_ui.sequence.anchor.as_ref().is_some_and(|a| {
                pointer.is_some_and(|p| {
                    let center = point(rect, bounds, a.x, a.y) + egui::vec2(15.0, 0.0);
                    p.distance(center) < 22.0
                })
            });
            if !retained {
                self.notation_ui.sequence.anchor = pointer
                    .filter(|p| ui.clip_rect().contains(*p))
                    .and_then(|p| {
                        let (x, y) = local(p);
                        anchor_at(&regions, x, y, 18.0 / scale)
                    })
                    .or_else(|| {
                        let (line, id, selected_revision) = self.notation_ui.selected.as_ref()?;
                        if *selected_revision != revision {
                            return None;
                        }
                        let selected = regions.iter().find(|r| {
                            r.line == *line
                                && id.as_ref().is_none_or(|id| &r.id == id)
                                && matches!(
                                    r.kind.as_str(),
                                    "participant" | "execution" | "lifeline"
                                )
                        })?;
                        anchor_at(
                            &regions,
                            selected.x + selected.width / 2.0,
                            selected.y + selected.height / 2.0,
                            18.0 / scale,
                        )
                    });
            }
        }
        let Some(mut anchor) = self.notation_ui.sequence.anchor.clone() else {
            return false;
        };
        if anchor.kind == Kind::Execution {
            // The renderer's execution endpoint and event regions share final geometry.
            let bar = regions.iter().find(|r| {
                r.kind == "sequence-execution"
                    && r.id == anchor.owner
                    && (r.x + r.width - anchor.x).abs() < 0.01
                    && anchor.y >= r.y
                    && anchor.y <= r.y + r.height
            });
            if let Some(bar) = bar {
                let doc = laml::sequence::parse(self.document.source()).ok();
                anchor.end_line = regions.iter().find(|r| r.kind == "event" && (r.y+12.0-bar.y-bar.height).abs() < 0.1
                    && doc.as_ref().is_some_and(|doc| doc.events.iter().any(|e| matches!(e, laml::sequence::Event::Deactivate { participant, line, .. } if participant == &anchor.owner && *line == r.line)))).map(|r| r.line);
            }
        }
        let center = point(rect, bounds, anchor.x, anchor.y) + egui::vec2(15.0, 0.0);
        if anchor.kind != Kind::Head {
            let stroke = egui::Stroke::new(
                1.0,
                appearance::color(ui.ctx(), appearance::MUTED).gamma_multiply(0.6),
            );
            ui.painter()
                .hline(rect.left()..=rect.right(), center.y, stroke);
        }
        let response = ui
            .push_id(("sequence-plus", self.id), |ui| {
                ui.put(
                    egui::Rect::from_center_size(center, egui::Vec2::splat(24.0)),
                    egui::Button::new(egui::RichText::new(egui_phosphor::regular::PLUS).font(
                        egui::FontId::new(16.0, egui::FontFamily::Name("phosphor".into())),
                    ))
                    .corner_radius(12),
                )
            })
            .inner;
        let owned = response.hovered()
            || response.is_pointer_button_down_on()
            || self.notation_ui.sequence.menu_open;
        let popup = egui::Popup::menu(&response);
        self.notation_ui.sequence.menu_open = popup.is_open();
        let mut selected = None;
        popup.show(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(16.0, 6.0);
            let mut action = |title: &str, action: Action| {
                // Target-dependent message validity is checked after choosing the target.
                let valid = if matches!(action, Action::Sync | Action::SyncActivate | Action::Async)
                {
                    Ok(())
                } else if action == Action::Reply {
                    self.sequence_reply_available(&anchor)
                } else if matches!(action, Action::Note | Action::Participant) {
                    Ok(())
                } else {
                    self.sequence_candidate(&anchor, action, "", "", "")
                        .map(|_| ())
                };
                let response = ui.add_enabled(valid.is_ok(), egui::Button::new(title));
                if response.clicked() {
                    selected = Some(action);
                    ui.close();
                }
                if let Err(error) = valid {
                    response.on_disabled_hover_text(error);
                }
            };
            if anchor.kind == Kind::Head {
                action("Add participant…", Action::Participant);
            } else {
                action("Synchronous call →", Action::Sync);
                action("Call with activation →", Action::SyncActivate);
                action("Asynchronous message →", Action::Async);
                action("Reply →", Action::Reply);
                action(
                    if anchor.kind == Kind::Execution {
                        "Start nested activation"
                    } else {
                        "Start activation"
                    },
                    Action::Activate,
                );
                if anchor.kind == Kind::Execution {
                    action("End activation here", Action::EndActivation);
                }
                action("Add note…", Action::Note);
                if anchor.kind == Kind::End {
                    action("Destroy participant ×", Action::Destroy);
                }
            }
        });
        if let Some(action) = selected {
            self.sequence_action(anchor, action);
        }
        owned || self.notation_ui.sequence.menu_open || selected.is_some()
    }
    fn sequence_reply_available(&self, anchor: &Anchor) -> Result<(), String> {
        let doc = laml::sequence::parse(self.document.source()).map_err(|e| e.message)?;
        for target in &doc.participants {
            if self
                .sequence_candidate(anchor, Action::Reply, &target.id, "", "Response")
                .is_ok()
            {
                return Ok(());
            }
        }
        Err("No unambiguous open call can be answered by this participant here".into())
    }
}
