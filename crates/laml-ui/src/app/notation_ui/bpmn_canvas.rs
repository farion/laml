//! Inline BPMN controls use the compiler's final node, container and flow regions.
use super::*;
use edit::bpmn_inline::Action;
use laml::authoring::Region;
mod connections;

#[derive(Default)]
pub(super) struct State {
    revision: Option<u64>,
    anchor: Option<Region>,
    menu_open: bool,
    form: Option<Form>,
    reconnect: Option<(usize, bool)>,
    connect: Option<String>,
}
struct Form {
    action: Action,
    ty: String,
    id: String,
    label: String,
    flow_label: String,
    revision: u64,
}
fn point(rect: egui::Rect, bounds: (f64, f64, f64, f64), x: f64, y: f64) -> egui::Pos2 {
    let scale = f64::from(rect.width()) / (bounds.2 - bounds.0);
    rect.min
        + egui::vec2(
            ((x - bounds.0) * scale) as f32,
            ((y - bounds.1) * scale) as f32,
        )
}
fn plus_at(r: &Region) -> (f64, f64) {
    if !r.path.is_empty() {
        let length: f64 = r
            .path
            .windows(2)
            .map(|p| (p[1].0 - p[0].0).hypot(p[1].1 - p[0].1))
            .sum();
        let mut rest = length / 2.0;
        for pair in r.path.windows(2) {
            let len = (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1);
            if rest <= len && len > 0.0 {
                let t = rest / len;
                return (
                    pair[0].0 + t * (pair[1].0 - pair[0].0),
                    pair[0].1 + t * (pair[1].1 - pair[0].1),
                );
            }
            rest -= len;
        }
    }
    if matches!(r.kind.as_str(), "pool" | "lane") {
        (r.x + r.width - 18.0, r.y + 18.0)
    } else {
        (r.x + r.width, r.y + r.height / 2.0)
    }
}
fn near(r: &Region, p: egui::Pos2, rect: egui::Rect, bounds: (f64, f64, f64, f64)) -> bool {
    if r.path.is_empty() {
        return egui::Rect::from_min_max(
            point(rect, bounds, r.x, r.y),
            point(rect, bounds, r.x + r.width, r.y + r.height),
        )
        .expand(8.0)
        .contains(p);
    }
    r.path.windows(2).any(|v| {
        let a = point(rect, bounds, v[0].0, v[0].1);
        let b = point(rect, bounds, v[1].0, v[1].1);
        let d = b - a;
        let t = if d.length_sq() < 0.01 {
            0.0
        } else {
            ((p - a).dot(d) / d.length_sq()).clamp(0.0, 1.0)
        };
        p.distance(a + t * d) < 10.0
    })
}
impl Editor {
    pub(in crate::app) fn bpmn_canvas(&mut self, ui: &mut egui::Ui, rect: egui::Rect) -> bool {
        let revision = self.document.revision();
        if self.notation_ui.bpmn.revision != Some(revision)
            || !self.can_edit_visually()
            || ui.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.notation_ui.bpmn = State {
                revision: Some(revision),
                ..Default::default()
            };
        }
        if !self.can_edit_visually() {
            return false;
        }
        if dialogs::blocked(ui.ctx()) || self.notation_ui.bpmn.form.is_some() {
            return true;
        }
        if self.bpmn_connections(ui, rect) {
            return true;
        }
        let Some(preview) = &self.preview else {
            return false;
        };
        let bounds = preview.bounds;
        let regions = &preview.regions;
        let pointer = ui
            .input(|i| i.pointer.hover_pos())
            .filter(|p| ui.clip_rect().contains(*p));
        if !self.notation_ui.bpmn.menu_open {
            let retained = self.notation_ui.bpmn.anchor.as_ref().is_some_and(|r| {
                let (x, y) = plus_at(r);
                let at = point(rect, bounds, x, y)
                    + if r.kind == "node" {
                        egui::vec2(15.0, 0.0)
                    } else {
                        egui::Vec2::ZERO
                    };
                pointer.is_some_and(|p| p.distance(at) < 22.0)
            });
            if !retained {
                self.notation_ui.bpmn.anchor = pointer
                    .and_then(|p| {
                        regions
                            .iter()
                            .filter(|r| near(r, p, rect, bounds))
                            .min_by(|a, b| {
                                let rank = |r: &Region| match r.kind.as_str() {
                                    "flow" => 0,
                                    "node" => 1,
                                    "lane" => 2,
                                    _ => 3,
                                };
                                rank(a).cmp(&rank(b)).then_with(|| {
                                    (a.width * a.height).total_cmp(&(b.width * b.height))
                                })
                            })
                            .cloned()
                    })
                    .or_else(|| {
                        let (line, id, rev) = self.notation_ui.selected.as_ref()?;
                        if *rev != revision {
                            return None;
                        }
                        regions
                            .iter()
                            .find(|r| r.line == *line && id.as_ref().is_some_and(|id| id == &r.id))
                            .cloned()
                    });
            }
        }
        let Some(anchor) = self.notation_ui.bpmn.anchor.clone() else {
            return false;
        };
        let (x, y) = plus_at(&anchor);
        let center = point(rect, bounds, x, y)
            + if anchor.kind == "node" {
                egui::vec2(15.0, 0.0)
            } else {
                egui::Vec2::ZERO
            };
        let response = ui
            .push_id(("bpmn-plus", self.id), |ui| {
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
            || self.notation_ui.bpmn.menu_open;
        let popup = egui::Popup::menu(&response);
        self.notation_ui.bpmn.menu_open = popup.is_open();
        let Ok(doc) = laml::bpmn::parse(self.document.source()) else {
            return owned;
        };
        let mut chosen = None;
        let mut connect = false;
        popup.show(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(16.0, 6.0);
            let mut action = |title: &str, action: Action| {
                let valid = self
                    .bpmn_initial_type(&action)
                    .ok_or("No supported element fits this context".to_owned());
                let response =
                    ui.add_enabled(valid.is_ok(), egui::Button::new(crate::i18n::tr(title)));
                if response.clicked() {
                    chosen = Some(action);
                    ui.close();
                }
                if let Err(error) = valid {
                    response.on_disabled_hover_text(crate::i18n::message(&(error)));
                }
            };
            if let Some(index) = anchor.id.strip_prefix("flow:").and_then(|s| s.parse().ok()) {
                action("Insert element…", Action::Insert(index));
            } else if matches!(anchor.kind.as_str(), "pool" | "lane") {
                action("Add element here…", Action::Content(anchor.line));
                if anchor.kind == "pool" {
                    action("Add lane…", Action::Lane(anchor.line));
                }
            } else if let Some(node) = doc.nodes.iter().find(|n| n.id == anchor.id) {
                let title = if matches!(
                    node.kind,
                    laml::bpmn::Kind::Exclusive
                        | laml::bpmn::Kind::Parallel
                        | laml::bpmn::Kind::Inclusive
                        | laml::bpmn::Kind::EventBased
                ) {
                    "Add branch…"
                } else {
                    "Add successor…"
                };
                action(title, Action::Successor(anchor.id.clone()));
                if node.kind == laml::bpmn::Kind::Subprocess {
                    action("Add element inside…", Action::Content(anchor.line));
                }
                if matches!(
                    node.kind,
                    laml::bpmn::Kind::Task
                        | laml::bpmn::Kind::UserTask
                        | laml::bpmn::Kind::ServiceTask
                        | laml::bpmn::Kind::SendTask
                        | laml::bpmn::Kind::ReceiveTask
                        | laml::bpmn::Kind::ScriptTask
                        | laml::bpmn::Kind::ManualTask
                        | laml::bpmn::Kind::Subprocess
                        | laml::bpmn::Kind::Collapsed
                ) {
                    ui.menu_button(crate::i18n::tr("Attach boundary event"), |ui| {
                        for (marker, title) in [
                            ("timer", "Timer…"),
                            ("timer+", "Non-interrupting timer…"),
                            ("message", "Message…"),
                            ("message+", "Non-interrupting message…"),
                            ("error", "Error…"),
                        ] {
                            if ui.button(crate::i18n::tr(title)).clicked() {
                                chosen = Some(Action::Boundary(anchor.id.clone(), marker.into()));
                                ui.close();
                            }
                        }
                    });
                }
                if ui
                    .button(crate::i18n::tr("Connect to existing element →"))
                    .clicked()
                {
                    connect = true;
                    ui.close();
                }
            }
        });
        if let Some(action) = chosen {
            let ty = self
                .bpmn_initial_type(&action)
                .unwrap_or_else(|| "task".into());
            self.notation_ui.bpmn.form = Some(Form {
                action,
                ty,
                id: self.bpmn_fresh_id(),
                label: String::new(),
                flow_label: String::new(),
                revision,
            });
            self.notation_ui.bpmn.menu_open = false;
            self.notation_ui.drag = None;
            return true;
        }
        if connect {
            self.notation_ui.bpmn.connect = Some(anchor.id);
            self.notation_ui.bpmn.menu_open = false;
            self.notation_ui.drag = None;
            return true;
        }
        owned || self.notation_ui.bpmn.menu_open
    }
    fn bpmn_initial_type(&self, action: &Action) -> Option<String> {
        let fresh = self.bpmn_fresh_id();
        edit::bpmn::TYPES
            .iter()
            .map(|(key, _)| *key)
            .chain(std::iter::once("subprocess"))
            .find(|ty| {
                edit::bpmn_inline::create(
                    self.document.source(),
                    action,
                    ty,
                    &fresh,
                    "New element",
                    "",
                )
                .is_ok()
            })
            .map(str::to_owned)
    }
    fn bpmn_fresh_id(&self) -> String {
        (1..)
            .map(|i| format!("Element{i}"))
            .find(|id| !self.document.source().contains(id))
            .unwrap()
    }
    pub(in crate::app) fn bpmn_inline_window(&mut self, ctx: &egui::Context) {
        let Some(mut form) = self.notation_ui.bpmn.form.take() else {
            return;
        };
        let mut submit = false;
        let mut cancel = false;
        let title = match &form.action {
            Action::Insert(_) => "Insert BPMN element",
            Action::Successor(_) => "Add BPMN successor",
            Action::Content(_) => "Add BPMN content",
            Action::Lane(_) => "Add lane",
            Action::Boundary(_, _) => "Attach boundary event",
        };
        let dismissed = dialogs::show(ctx, ("bpmn-inline", self.id), title, 400.0, |ui| {
            let lane = matches!(form.action, Action::Lane(_));
            if !lane {
                ui.label(if matches!(form.action, Action::Boundary(_, _)) {
                    crate::i18n::tr("Outgoing target type")
                } else {
                    crate::i18n::tr("Type")
                });
                let mut types = edit::bpmn::TYPES.to_vec();
                types.push(("subprocess", "Expanded subprocess"));
                let fresh = self.bpmn_fresh_id();
                egui::ComboBox::from_id_salt("bpmn-inline-type")
                    .width(280.0)
                    .selected_text(
                        types
                            .iter()
                            .find(|(key, _)| *key == form.ty)
                            .map_or(form.ty.as_str(), |(_, title)| crate::i18n::tr(title)),
                    )
                    .show_ui(ui, |ui| {
                        for (ty, title) in types {
                            if edit::bpmn_inline::create(
                                self.document.source(),
                                &form.action,
                                ty,
                                &fresh,
                                "New element",
                                "",
                            )
                            .is_ok()
                            {
                                ui.selectable_value(
                                    &mut form.ty,
                                    ty.into(),
                                    crate::i18n::tr(title),
                                );
                            }
                        }
                    });
            }
            ui.label(crate::i18n::tr("ID"));
            self.source_ui
                .extra_drafts
                .push(ui.text_edit_singleline(&mut form.id).id);
            ui.label(crate::i18n::tr("Label"));
            self.source_ui
                .extra_drafts
                .push(ui.text_edit_singleline(&mut form.label).id);
            if matches!(form.action, Action::Successor(_)) {
                ui.label(crate::i18n::tr("Flow label"));
                self.source_ui
                    .extra_drafts
                    .push(ui.text_edit_singleline(&mut form.flow_label).id);
            }
            if matches!(form.action, Action::Boundary(_, _)) {
                ui.label(crate::i18n::format(
                    "Creates outgoing target: {0}Target",
                    &[("0", form.id.to_string())],
                ));
            }
            let candidate = edit::bpmn_inline::create(
                self.document.source(),
                &form.action,
                &form.ty,
                &form.id,
                &form.label,
                &form.flow_label,
            );
            if let Err(error) = &candidate {
                ui.label(crate::i18n::message(error));
            }
            if form.revision != self.document.revision() {
                ui.label(crate::i18n::tr("Document changed. Reopen this action."));
            }
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                submit = ui
                    .add_enabled(
                        candidate.is_ok()
                            && form.revision == self.document.revision()
                            && self.can_edit_visually(),
                        egui::Button::new(crate::i18n::tr("Add")),
                    )
                    .clicked();
                cancel = ui.button(crate::i18n::tr("Cancel")).clicked();
            });
        });
        if submit {
            match edit::bpmn_inline::create(
                self.document.source(),
                &form.action,
                &form.ty,
                &form.id,
                &form.label,
                &form.flow_label,
            ) {
                Ok(edits) => {
                    let selected = edit::apply(self.document.source(), &edits)
                        .ok()
                        .and_then(|source| laml::bpmn::parse(&source).ok())
                        .and_then(|doc| {
                            let old = laml::bpmn::parse(self.document.source()).ok()?;
                            doc.nodes
                                .into_iter()
                                .find(|n| {
                                    n.id.rsplit('.').next() == Some(form.id.as_str())
                                        && !old.nodes.iter().any(|o| o.id == n.id)
                                })
                                .map(|n| (n.line, n.id))
                        });
                    self.submit_command(Command {
                        edits,
                        label: title.into(),
                        expectation: worker::Expectation::Notation {
                            line: selected.as_ref().map(|s| s.0),
                            id: selected.map(|s| s.1),
                        },
                    });
                }
                Err(e) => self.command_error = Some(e),
            }
        } else if !cancel && !dismissed {
            self.notation_ui.bpmn.form = Some(form);
        }
    }
}
