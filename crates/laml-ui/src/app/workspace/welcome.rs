//! First-run orientation, shared by native and browser editors.
use super::*;

/// Word-sized runs keep links in the paragraph's normal wrapping flow.
fn paragraph(ui: &mut egui::Ui, parts: &[(&str, Option<&str>, bool)]) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        for &(text, url, bold) in parts {
            let text = crate::i18n::tr(text);
            let family = if bold {
                egui::FontFamily::Name("bold".into())
            } else {
                egui::FontFamily::Proportional
            };
            if let Some(url) = url {
                ui.add(egui::Hyperlink::from_label_and_url(
                    egui::RichText::new(text).font(egui::FontId::new(15.0, family)),
                    url,
                ));
            } else {
                for word in text.split_inclusive(' ') {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(word).font(egui::FontId::new(15.0, family.clone())),
                        )
                        .wrap_mode(egui::TextWrapMode::Extend),
                    );
                }
            }
        }
    });
}

impl Workspace {
    pub(super) fn welcome_window(&mut self, ctx: &egui::Context) {
        let mut understood = false;
        let dismissed = dialogs::show(ctx, "welcome-to-laml", "LAML Editor", 560.0, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.vertical_centered(|ui| branding::wordmark(ui, 260.0));
            ui.add_space(18.0);
            paragraph(
                ui,
                &[
                    ("LAML is a markup language similar to ", None, false),
                    ("PlantUML", Some("https://plantuml.com/"), false),
                    (" or ", None, false),
                    ("Mermaid", Some("https://mermaid.js.org/"), false),
                    (
                        ", with a focus on giving you explicit control over layout. Layout should not be an accidental by-product of rendering.",
                        None,
                        false,
                    ),
                ],
            );
            ui.add_space(18.0);
            paragraph(
                ui,
                &[
                    (
                        "This editor helps you create LAML markup visually. ",
                        None,
                        false,
                    ),
                    ("It is NOT a diagram drawing tool. ", None, true),
                    (
                        "LAML is for people who value the benefits of text-based diagram authoring. If you prefer drawing diagrams directly, consider ",
                        None,
                        false,
                    ),
                    ("draw.io", Some("https://app.diagrams.net/"), false),
                    (", ", None, false),
                    ("yEd", Some("https://www.yworks.com/products/yed"), false),
                    (" or ", None, false),
                    (
                        "OmniGraffle",
                        Some("https://www.omnigroup.com/omnigraffle"),
                        false,
                    ),
                    (" instead.", None, false),
                ],
            );
            ui.add_space(18.0);
            paragraph(ui, &[("Enjoy using the LAML Editor!", None, false)]);
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            // Bound the child layout to button height, not the scroll viewport.
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    understood = ui
                        .add(
                            egui::Button::new(crate::i18n::tr("Continue to LAML Editor"))
                                .fill(ui.visuals().selection.bg_fill),
                        )
                        .clicked();
                });
            });
        });
        if understood || dismissed {
            self.welcome_open = false;
            self.preferences.welcome_seen = true;
            if let Err(error) = self.preferences.save() {
                self.error = Some(crate::i18n::format(
                    "Could not save the welcome preference: {error}. The introduction may appear again next time.",
                    &[("error", error.to_string())],
                ));
            }
        }
    }
}
