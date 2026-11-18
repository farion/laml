//! Explanatory hover text is visually distinct from ordinary control captions.
use super::*;

pub(super) fn show(ui: &mut egui::Ui, text: &str) {
    ui.set_max_width(400.0);
    ui.horizontal_top(|ui| {
        ui.label(
            egui::RichText::new(egui_phosphor::regular::QUESTION)
                .font(egui::FontId::new(
                    16.0,
                    egui::FontFamily::Name("phosphor".into()),
                ))
                .color(appearance::color(ui.ctx(), appearance::MUTED)),
        );
        ui.add(egui::Label::new(egui::RichText::new(text).italics()).wrap());
    });
}
