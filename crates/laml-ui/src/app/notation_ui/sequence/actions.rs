//! Object-local context actions; relation choices are checked before display.
use super::*;
pub(super) fn menu_text(ui: &egui::Ui, title: &str) -> egui::text::LayoutJob {
    let icon = menu_icons::icon(title);
    let mut text = egui::text::LayoutJob::default();
    text.append(
        &format!("{icon}  "),
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::new(16.0, egui::FontFamily::Name("phosphor".into())),
            color: ui.visuals().text_color(),
            ..Default::default()
        },
    );
    text.append(
        crate::i18n::tr(title),
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(15.0),
            color: ui.visuals().text_color(),
            ..Default::default()
        },
    );
    text
}
pub(super) fn menu_button(ui: &mut egui::Ui, title: &str) -> bool {
    ui.add(egui::Button::new(menu_text(ui, title)).wrap_mode(egui::TextWrapMode::Extend))
        .clicked()
}
fn target(hit: &Hit) -> Option<(usize, String)> {
    match hit {
        Hit::Participant(r) | Hit::Event(r) | Hit::Destruction(r) => Some((r.line, r.id.clone())),
        Hit::Endpoint { message, .. } => Some((message.line, message.id.clone())),
        Hit::Activation(bar) | Hit::Resize(bar, _) => {
            Some((bar.region.line, format!("line:{}", bar.region.line)))
        }
        _ => None,
    }
}
impl Editor {
    pub(in crate::app) fn sequence_delete_selected(&mut self, line: usize) -> bool {
        let plan = match self.notation_ui.sequence.selection.as_ref() {
            Some(Hit::Activation(bar) | Hit::Resize(bar, _)) if bar.region.line == line => {
                edit::sequence_lifetime::activation(
                    self.document.source(),
                    &bar.owner,
                    bar.region.line,
                )
            }
            Some(Hit::Destruction(r)) if r.line == line => {
                edit::sequence_lifetime::destruction(self.document.source(), &r.id, r.line)
            }
            _ => return false,
        };
        match plan {
            Ok(plan) => self.notation_ui.deletion = Some((self.document.revision(), plan)),
            Err(e) => self.command_error = Some(e),
        }
        true
    }
    pub(super) fn sequence_context(&mut self, _ui: &mut egui::Ui, response: &egui::Response) {
        let context = self.notation_ui.sequence.context.clone();
        response.context_menu(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(16.0, 6.0);
            if let Some((line, id)) = context.as_ref().and_then(target) {
                let is_message = self.preview.as_ref().is_some_and(|p| {
                    p.regions
                        .iter()
                        .any(|r| r.kind == "message" && r.line == line)
                });
                if is_message {
                    for ty in laml_ui::notation::Notation::Sequence.relation_types() {
                        if let Ok(edits) =
                            controls::message_type(self.document.source(), line, ty.key)
                            && actions::menu_button(ui, ty.title)
                        {
                            self.sequence_apply(Proposal {
                                edits,
                                line: Some(line),
                                label: "Change message type",
                            });
                            ui.close();
                        }
                    }
                    for (create, title) in [(true, "Creation message"), (false, "Deletion message")]
                    {
                        if let Ok(edits) = edit::sequence_lifetime::message_kind(
                            self.document.source(),
                            line,
                            create,
                        ) && actions::menu_button(ui, title)
                        {
                            self.sequence_apply(Proposal {
                                edits,
                                line: Some(line),
                                label: "Change message lifetime",
                            });
                            ui.close();
                        }
                    }
                    ui.add_space(2.0);
                    ui.separator();
                    ui.add_space(2.0);
                }
                if edit::sequence_fragments::branch(self.document.source(), line, "").is_ok()
                    && actions::menu_button(ui, "Add branch")
                {
                    self.sequence_add_branch(line);
                    ui.close();
                }
                if !matches!(
                    context,
                    Some(
                        Hit::Participant(_)
                            | Hit::Activation(_)
                            | Hit::Resize(..)
                            | Hit::Destruction(_)
                    )
                ) {
                    self.sequence_wrap_menu(ui, line);
                }
                if actions::menu_button(ui, "Delete…") {
                    let plan = match context.as_ref() {
                        Some(Hit::Activation(bar) | Hit::Resize(bar, _)) => {
                            Some(edit::sequence_lifetime::activation(
                                self.document.source(),
                                &bar.owner,
                                bar.region.line,
                            ))
                        }
                        Some(Hit::Destruction(r)) => Some(edit::sequence_lifetime::destruction(
                            self.document.source(),
                            &r.id,
                            r.line,
                        )),
                        _ => None,
                    };
                    match plan {
                        Some(Ok(plan)) => {
                            self.notation_ui.deletion = Some((self.document.revision(), plan))
                        }
                        Some(Err(e)) => self.command_error = Some(e),
                        None => self.notation_delete(line, Some(&id), false),
                    }
                    ui.close();
                }
            }
            if actions::menu_button(ui, "Elements…") {
                self.notation_ui.navigator = true;
                ui.close();
            }
        });
    }
}
