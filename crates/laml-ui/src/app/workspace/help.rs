//! Compact, always-accessible introduction to source-first diagram editing.
use super::*;

fn heading(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(egui::RichText::new(text)
        .font(egui::FontId::new(18.0, egui::FontFamily::Name("bold".into())))).wrap());
}
impl Workspace {
    pub(super) fn help_window(&mut self, ctx: &egui::Context) {
        let mut close = false;
        let dismissed = dialogs::show(ctx, "editor-help", "LAML Editor help", 540.0, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            heading(ui, "Diagrams written as text");
            ui.label("LAML describes elements, relationships, layout rules and styles in a .laml file, then renders a reproducible diagram. The source is always the document.");
            ui.add_space(16.0);
            heading(ui, "Write or edit visually");
            ui.label("Edit Source on the left, select elements in the preview, and adjust Properties on the right. Contextual + controls add elements. Visual changes update the same source; Undo works for both.");
            ui.add_space(16.0);
            heading(ui, "Layout follows rules");
            ui.label("Dragging changes supported placement rules or event order, not freehand coordinates. Invalid source keeps the last valid preview visible until corrected.");
            ui.add_space(16.0);
            heading(ui, "Save source, export images");
            ui.label("Save keeps the editable .laml file. Export creates SVG, PNG or PDF. The documentation explains syntax, diagram types and examples.");
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Documentation on GitHub").clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(
                        "https://github.com/farion/laml/blob/main/docs/getting-started.md"));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close = ui.button("Close").clicked();
                });
            });
        });
        if close || dismissed { self.help_open = false; }
    }
}
