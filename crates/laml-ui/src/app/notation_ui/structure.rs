//! Context-sensitive actions explain compiler rejections before mutation.
use super::*;
fn action(ui: &mut egui::Ui, title: &str, proposal: &Result<Vec<laml_ui::document::Edit>, String>) -> bool {
    let response = ui.add_enabled(proposal.is_ok(), egui::Button::new(title));
    let clicked = response.clicked();
    if let Err(reason) = proposal { response.on_disabled_hover_text(reason); }
    clicked
}
pub(super) fn checked(source: &str, proposal: Result<Vec<laml_ui::document::Edit>, String>) -> Result<Vec<laml_ui::document::Edit>, String> {
    proposal.and_then(|edits| { let candidate = edit::apply(source, &edits)?; laml_ui::notation::inspect(&candidate)?; Ok(edits) })
}
impl Editor {
    pub(super) fn notation_structure(&mut self, ui: &mut egui::Ui, record: &edit::Record, id: Option<&str>) {
        inspector_ui::content(ui, |ui| self.notation_structure_contents(ui, record, id));
    }
    fn notation_structure_contents(&mut self, ui: &mut egui::Ui, record: &edit::Record, id: Option<&str>) {
        let line = record.line;
        ui.horizontal_wrapped(|ui| {
            if ui.button("Delete…").clicked() { self.notation_delete(line, id, false); }
            if record.opens {
                let plan = edit::delete(self.document.source(), line, id, true);
                let proposal = checked(self.document.source(), plan.as_ref().map(|p| p.edits.clone()).map_err(Clone::clone));
                if action(ui, "Unpack…", &proposal) && let Ok(plan) = plan { self.notation_ui.deletion = Some((self.document.revision(), plan)); }
            }
        });
        if ui.button("Add construct…").clicked() { self.open_construct_dialog(); }
        ui.label("Destination");
        destination(ui, self.document.source(), &record.section, &mut self.notation_ui.move_parent, &mut self.notation_ui.move_before, Some(line));
        let proposal = if self.notation() == Notation::Bpmn && id.is_some_and(|id| !id.starts_with("flow:")) && self.notation_ui.move_parent.is_some() {
            edit::bpmn::relocate(self.document.source(), id.unwrap(), self.notation_ui.move_parent.unwrap())
        } else { edit::move_record(self.document.source(), line, self.notation_ui.move_parent, self.notation_ui.move_before) };
        let proposal = checked(self.document.source(), proposal);
        if action(ui, "Move to destination", &proposal) { self.notation_submit("Move construct", proposal.unwrap(), None); }
    }
}
