//! Compact, always-accessible introduction to source-first diagram editing.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum LegalDocument {
    Imprint,
    Privacy,
}
impl LegalDocument {
    fn title(self) -> &'static str {
        match self {
            Self::Imprint => "Imprint",
            Self::Privacy => "Data Privacy Statement",
        }
    }
    fn text(self) -> &'static str {
        match self {
            Self::Imprint => include_str!("help/imprint.txt"),
            Self::Privacy => include_str!("help/privacy.txt"),
        }
    }
}

fn heading(ui: &mut egui::Ui, text: &str) {
    ui.add(
        egui::Label::new(egui::RichText::new(text).font(egui::FontId::new(
            18.0,
            egui::FontFamily::Name("bold".into()),
        )))
        .wrap(),
    );
}
impl Workspace {
    pub(super) fn help_window(&mut self, ctx: &egui::Context) {
        if let Some(document) = self.help_document {
            self.help_document_window(ctx, document);
            return;
        }
        let mut close = false;
        let mut document = None;
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
            ui.horizontal_wrapped(|ui| {
                if ui.button("Imprint").clicked() {
                    document = Some(LegalDocument::Imprint);
                }
                if ui.button("Data Privacy Statement").clicked() {
                    document = Some(LegalDocument::Privacy);
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Documentation on GitHub").clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(
                        "https://github.com/farion/laml/blob/main/docs/getting-started.md",
                    ));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close = ui.button("Close").clicked();
                });
            });
        });
        if close || dismissed {
            self.help_open = false;
        }
        self.help_document = document;
    }

    fn help_document_window(&mut self, ctx: &egui::Context, document: LegalDocument) {
        let mut close = false;
        let dismissed = dialogs::show(
            ctx,
            ("help-document", document.title()),
            document.title(),
            680.0,
            |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                // The shared dialog provides scrolling, selectable text and 16-point
                // padding. Keep the supplied statement as text, without web requests.
                let body = document
                    .text()
                    .split_once('\n')
                    .map_or(document.text(), |(_, body)| body);
                for paragraph in body.trim().split("\n\n") {
                    ui.add(egui::Label::new(paragraph).wrap().selectable(true));
                }
                ui.add_space(8.0);
                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close = ui.button("Close").clicked();
                });
            },
        );
        if close || dismissed {
            self.help_document = None;
        }
    }
}
