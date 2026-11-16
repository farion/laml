//! No document is created until the user explicitly creates or opens one.
use super::*;

pub(super) fn action_button(ui: &mut egui::Ui, glyph: &str, label: &str) -> egui::Response {
    let label = crate::i18n::tr(label);
    let color = ui.visuals().text_color();
    let mut text = egui::text::LayoutJob::default();
    text.append(
        glyph,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::new(20.0, egui::FontFamily::Name("phosphor".into())),
            color,
            ..Default::default()
        },
    );
    text.append(
        &format!("  {label}"),
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(16.0),
            color,
            ..Default::default()
        },
    );
    ui.add_sized([250.0, 44.0], egui::Button::new(text))
}

impl Workspace {
    pub(super) fn empty_view(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(appearance::panel_frame(appearance::color(
                ui.ctx(),
                appearance::HEADER,
            )))
            .show(ui, |ui| {
                let bounds = ui.available_rect_before_wrap();
                ui.add_space(((ui.available_height() - 300.0) * 0.5).max(24.0));
                ui.vertical_centered(|ui| {
                    branding::wordmark(ui, 340.0);
                    ui.add_space(24.0);
                    ui.heading(crate::i18n::tr("No file open"));
                    ui.add_space(20.0);
                    if action_button(ui, egui_phosphor::regular::FILE_PLUS, "Create new file")
                        .clicked()
                    {
                        self.begin_new_diagram();
                    }
                    ui.add_space(10.0);
                    let browser = laml_ui::services::browser_mode();
                    let (glyph, label) = if browser {
                        (egui_phosphor::regular::BOOKS, "Open from library")
                    } else {
                        (FOLDER_OPEN, "Open file")
                    };
                    if action_button(ui, glyph, label).clicked() {
                        self.open_existing(ui.ctx());
                    }
                });
                let footer = egui::Rect::from_min_max(
                    egui::pos2(bounds.left(), bounds.bottom() - 60.0),
                    egui::pos2(bounds.right(), bounds.bottom() - 16.0),
                );
                let mut footer_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("start-page-footer")
                        .max_rect(footer)
                        .layout(egui::Layout::top_down(egui::Align::Center)),
                );
                footer_ui.weak(crate::i18n::tr("Proudly made with Rust"));
                footer_ui.hyperlink_to(
                    "https://github.com/farion/laml",
                    "https://github.com/farion/laml",
                );
            });
    }

    pub(super) fn open_existing(&mut self, ctx: &egui::Context) {
        #[cfg(target_arch = "wasm32")]
        if laml_ui::services::browser_mode() {
            self.library.open = true;
            return;
        }
        self.open_dialog(ctx);
    }

    pub(super) fn open_dialog(&mut self, ctx: &egui::Context) {
        #[cfg(target_arch = "wasm32")]
        if laml_ui::services::browser_mode() {
            crate::browser_storage::import_laml();
            return;
        }
        if let Some(editor) = self.tabs.get_mut(self.active) {
            editor.open_tab(ctx);
        } else if self.file_picker.is_none() {
            self.file_picker = Some(host_dialogs::Picker::open());
        }
    }
}
