//! Direct operand insertion and object-local fragment wrapping.
use super::*;
impl Editor {
    pub(super) fn sequence_add_branch(&mut self, line: usize) {
        match edit::sequence_fragments::branch(self.document.source(), line, "condition") {
            Ok(edits) => self.sequence_apply(Proposal {
                edits,
                line: Some(line),
                label: "Add branch",
            }),
            Err(e) => self.command_error = Some(e),
        }
    }
    pub(super) fn sequence_wrap_menu(&mut self, ui: &mut egui::Ui, line: usize) {
        ui.menu_button(actions::menu_text(ui, "Wrap in fragment"), |ui| {
            ui.spacing_mut().button_padding = egui::vec2(16.0, 6.0);
            for (kind, title) in [
                ("opt", "Optional fragment"),
                ("alt", "Alternative"),
                ("par", "Parallel fragment"),
                ("loop", "Loop"),
                ("critical", "Critical fragment"),
                ("break", "Break fragment"),
                ("neg", "Negative fragment"),
                ("assert", "Assertion fragment"),
                ("strict", "Strict fragment"),
                ("seq", "Weak sequencing fragment"),
                ("ignore", "Ignore fragment"),
                ("consider", "Consider fragment"),
            ] {
                if actions::menu_button(ui, title) {
                    match edit::sequence_fragments::wrap(
                        self.document.source(),
                        line,
                        line,
                        kind,
                        "condition",
                    ) {
                        Ok(edits) => self.sequence_apply(Proposal {
                            edits,
                            line: Some(line),
                            label: "Wrap in fragment",
                        }),
                        Err(e) => self.command_error = Some(e),
                    }
                    ui.close();
                }
            }
        });
    }
}
