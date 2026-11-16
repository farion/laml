//! Destructive library actions use a separate modal, never inline controls.
use super::*;

impl Workspace {
    pub(super) fn library_confirmation(
        &mut self,
        ctx: &egui::Context,
        snapshot: &Snapshot,
        safe: bool,
    ) {
        let Some((kind, id)) = self.library.confirm.clone() else {
            return;
        };
        let (title, message, button) = match kind.as_str() {
            "clear" => (
                "Delete everything?",
                crate::i18n::format(
                    "Permanently delete all {0} documents from this browser, including the trash?",
                    &[("0", format!("{}", snapshot.records.len()))],
                ),
                "Delete everything",
            ),
            "empty" => (
                "Empty trash?",
                crate::i18n::format(
                    "Permanently delete all {0} documents in the trash?",
                    &[(
                        "0",
                        format!("{}", snapshot.records.iter().filter(|r| r.trashed).count()),
                    )],
                ),
                "Empty trash",
            ),
            _ => {
                let name = snapshot
                    .records
                    .iter()
                    .find(|r| r.id == id)
                    .map(|r| r.name.as_str())
                    .unwrap_or("the selected document");
                (
                    "Delete permanently?",
                    crate::i18n::format(
                        "Permanently delete {name}?",
                        &[("name", format!("{}", name))],
                    ),
                    "Delete permanently",
                )
            }
        };
        let mut accepted = false;
        let mut cancelled = false;
        let dismissed = dialogs::show(ctx, "browser-library-confirm", title, 460.0, |ui| {
            ui.horizontal_top(|ui| {
                ui.label(
                    egui::RichText::new(egui_phosphor::regular::WARNING)
                        .font(egui::FontId::new(
                            30.0,
                            egui::FontFamily::Name("phosphor".into()),
                        ))
                        .color(ui.visuals().error_fg_color),
                );
                ui.vertical(|ui| {
                    ui.set_max_width(ui.available_width());
                    ui.add(egui::Label::new(crate::i18n::message(&(&message))).wrap());
                    ui.add_space(8.0);
                    ui.strong(crate::i18n::tr("This cannot be undone."));
                });
            });
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    accepted = ui
                        .add_enabled(
                            safe,
                            egui::Button::new(
                                egui::RichText::new(crate::i18n::tr(&(button)))
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(180, 45, 55)),
                        )
                        .clicked();
                    cancelled = ui.button(crate::i18n::tr("Cancel")).clicked();
                });
            });
        });
        if accepted {
            storage::library_action(&kind, &id, "");
            self.library.confirm = None;
            self.library.rename = None;
        } else if cancelled || dismissed {
            self.library.confirm = None;
        }
    }
}
