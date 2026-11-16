//! Explicit, named document creation with a concrete notation.
use super::*;
use laml_ui::notation::Notation;

pub(super) struct Picker {
    family: &'static str,
    notation: Notation,
    name: String,
    include_example: bool,
}
impl Picker {
    fn new(name: String) -> Self {
        Self {
            family: "Generic",
            notation: Notation::Gne,
            name,
            include_example: true,
        }
    }
}
fn filename(name: &str) -> Result<String, &'static str> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Enter a file name.");
    }
    if name == "."
        || name == ".."
        || name.contains(['/', '\\'])
        || name.chars().any(char::is_control)
    {
        return Err("Enter a file name, not a path.");
    }
    Ok(if name.to_ascii_lowercase().ends_with(".laml") {
        name.into()
    } else {
        format!("{name}.laml")
    })
}
impl Workspace {
    pub(super) fn default_filename(&self) -> String {
        let mut number = self.next_untitled;
        loop {
            let name = if number == 1 {
                "untitled.laml".into()
            } else {
                format!("untitled-{number}.laml")
            };
            let used = self
                .tabs
                .iter()
                .any(|tab| tab.path.file_name().is_some_and(|p| p == name.as_str()));
            #[cfg(target_arch = "wasm32")]
            let used = used || (laml_ui::services::browser_mode() && self.library_has_name(&name));
            if !used {
                return name;
            }
            number += 1;
        }
    }
    pub(super) fn begin_new_diagram(&mut self) {
        self.new_diagram = Some(Picker::new(self.default_filename()));
    }
    pub(super) fn new_diagram_window(&mut self, ctx: &egui::Context) {
        let Some(mut picker) = self.new_diagram.take() else {
            return;
        };
        let mut create = false;
        let mut cancel = false;
        let dismissed = dialogs::show(ctx, "new-diagram", "New file", 420.0, |ui| {
            ui.label(crate::i18n::tr("File name"));
            ui.text_edit_singleline(&mut picker.name);
            let name = filename(&picker.name);
            if let Err(error) = name {
                ui.colored_label(ui.visuals().error_fg_color, crate::i18n::message(error));
            }
            ui.add_space(12.0);
            ui.label(crate::i18n::tr("Family"));
            ui.horizontal(|ui| {
                for family in ["Generic", "UML", "BPMN", "C4"] {
                    if ui
                        .selectable_value(&mut picker.family, family, crate::i18n::tr(family))
                        .changed()
                    {
                        picker.notation = Notation::ALL
                            .into_iter()
                            .find(|n| n.family() == family)
                            .unwrap();
                    }
                }
            });
            ui.add_space(12.0);
            ui.label(crate::i18n::tr("Diagram type"));
            egui::ComboBox::from_id_salt("new-diagram-type")
                .width(300.0)
                .selected_text(crate::i18n::tr(picker.notation.title()))
                .show_ui(ui, |ui| {
                    for notation in Notation::ALL
                        .into_iter()
                        .filter(|n| n.family() == picker.family)
                    {
                        ui.selectable_value(
                            &mut picker.notation,
                            notation,
                            crate::i18n::tr(notation.title()),
                        );
                    }
                });
            ui.add_space(12.0);
            ui.checkbox(
                &mut picker.include_example,
                crate::i18n::tr("Include example diagram"),
            );
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                create = ui
                    .add_enabled(name.is_ok(), egui::Button::new(crate::i18n::tr("Create")))
                    .clicked();
                cancel = ui.button(crate::i18n::tr("Cancel")).clicked();
            });
        });
        if create {
            if let Ok(name) = filename(&picker.name) {
                if picker.include_example {
                    self.new_typed_tab(picker.notation, name);
                } else {
                    self.new_typed_tab_with_example(picker.notation, name, false);
                }
            }
        } else if !cancel && !dismissed {
            self.new_diagram = Some(picker);
        }
    }
    pub(super) fn new_typed_tab(&mut self, notation: Notation, name: String) {
        self.new_typed_tab_with_example(notation, name, true);
    }
    fn new_typed_tab_with_example(
        &mut self,
        notation: Notation,
        name: String,
        include_example: bool,
    ) {
        let source = notation.new_file(self.editor_theme == egui::Theme::Dark, include_example);
        let mut editor = Editor::with_services(source, None, self.backend.clone());
        editor.path = PathBuf::from(name);
        self.next_untitled += 1;
        self.add_tab(editor);
        #[cfg(target_arch = "wasm32")]
        self.library_tick();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_dialog_includes_an_example_by_default() {
        assert!(Picker::new("Example.laml".into()).include_example);
    }
    #[test]
    fn names_are_required_and_receive_the_source_extension() {
        assert!(filename("  ").is_err());
        assert!(filename("folder/test.laml").is_err());
        assert_eq!(filename(" Architecture ").unwrap(), "Architecture.laml");
        assert_eq!(filename("Architecture.LAML").unwrap(), "Architecture.LAML");
    }
}
