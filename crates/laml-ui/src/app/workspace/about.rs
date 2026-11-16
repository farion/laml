//! Application information and locally bundled legal documents.
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

impl Workspace {
    pub(super) fn about_window(&mut self, ctx: &egui::Context) {
        if let Some(document) = self.about_document {
            self.about_document_window(ctx, document);
            return;
        }
        let mut close = false;
        let mut document = None;
        let dismissed = dialogs::show(ctx, "editor-about", "About LAML Editor", 540.0, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.heading(format!("LAML Editor {}", env!("CARGO_PKG_VERSION")));
            ui.add_space(8.0);
            ui.hyperlink("https://github.com/farion/laml");
            ui.add_space(8.0);
            ui.label(crate::i18n::tr("LAML License MIT"));
            ui.label(crate::i18n::tr("Editor License GPL-3.0-or-later"));
            ui.add_space(8.0);
            ui.label(crate::i18n::tr(
                "Maintainer: Frieder Reinhold<reinhold@trigon-media.com>",
            ));
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button(crate::i18n::tr("Imprint")).clicked() {
                    document = Some(LegalDocument::Imprint);
                }
                if ui
                    .button(crate::i18n::tr("Data Privacy Statement"))
                    .clicked()
                {
                    document = Some(LegalDocument::Privacy);
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close = ui.button(crate::i18n::tr("Close")).clicked();
                });
            });
        });
        if close || dismissed {
            self.about_open = false;
        }
        self.about_document = document;
    }

    fn about_document_window(&mut self, ctx: &egui::Context, document: LegalDocument) {
        let mut close = false;
        let dismissed = dialogs::show(
            ctx,
            ("about-document", document.title()),
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
                    ui.add(
                        egui::Label::new(crate::i18n::tr(paragraph))
                            .wrap()
                            .selectable(true),
                    );
                }
                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = ui.button(crate::i18n::tr("Close")).clicked();
                    });
                });
            },
        );
        if close || dismissed {
            self.about_document = None;
        }
    }
}
