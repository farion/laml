//! A concrete diagram type is chosen only when creating a document.
use super::*;
use laml_ui::notation::Notation;

pub(super) struct Picker {
    family: &'static str,
    notation: Notation,
}
impl Default for Picker {
    fn default() -> Self { Self { family: "Generic", notation: Notation::Gne } }
}
impl Workspace {
    pub(super) fn new_diagram_window(&mut self, ctx: &egui::Context) {
        let Some(mut picker) = self.new_diagram.take() else { return; };
        let mut create = false;
        let mut cancel = false;
        let dismissed = dialogs::show(ctx, "new-diagram", "New diagram", 420.0, |ui| {
            ui.label("Family");
            ui.horizontal(|ui| {
                for family in ["Generic", "UML", "BPMN", "C4"] {
                    if ui.selectable_value(&mut picker.family, family, family).changed() {
                        picker.notation = Notation::ALL.into_iter().find(|n| n.family() == family).unwrap();
                    }
                }
            });
            ui.add_space(12.0);
            ui.label("Diagram type");
            egui::ComboBox::from_id_salt("new-diagram-type").width(300.0)
                .selected_text(picker.notation.title()).show_ui(ui, |ui| {
                    for notation in Notation::ALL.into_iter().filter(|n| n.family() == picker.family) {
                        ui.selectable_value(&mut picker.notation, notation, notation.title());
                    }
                });
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                create = ui.button("Create").clicked();
                cancel = ui.button("Cancel").clicked();
            });
        });
        if create { self.new_typed_tab(picker.notation); }
        else if !cancel && !dismissed { self.new_diagram = Some(picker); }
    }
    fn new_typed_tab(&mut self, notation: Notation) {
        let source = notation.starter(self.editor_theme == egui::Theme::Dark);
        let mut editor = Editor::with_services(source, None, self.backend.clone());
        editor.path = PathBuf::from(if self.next_untitled == 1 { "untitled.laml".into() }
            else { format!("untitled-{}.laml", self.next_untitled) });
        self.next_untitled += 1;
        self.add_tab(editor);
    }
}
