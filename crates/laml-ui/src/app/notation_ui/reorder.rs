//! Source-order actions live in a floating canvas toolbar, not the inspector.
use super::*;
use egui_phosphor::regular::{ARROW_DOWN, ARROW_UP};

impl Editor {
    pub(in crate::app) fn notation_reorder_overlay(&mut self, ui: &mut egui::Ui, viewport: egui::Rect, navigation: bool) -> bool {
        if matches!(self.notation(), Notation::Bpmn | Notation::Sequence) { return false; }
        let selected = self.notation_ui.selected.clone().filter(|s| s.2 == self.document.revision());
        if !self.specialized() && selected.is_none() { return false; }
        let record = selected.as_ref().and_then(|s| edit::record(self.document.source(), s.0).ok());
        let proposal = |down| {
            let Some(record) = &record else { return Err("Select an element to change its order".into()); };
            let edits = if self.notation() == Notation::Timing {
                edit::timing::adjacent_lane(self.document.source(), record.line, down)
            } else { edit::reorder(self.document.source(), record.line, down) };
            structure::checked(self.document.source(), edits)
        };
        let up = proposal(false);
        let down = proposal(true);
        let enabled = self.can_edit_visually() && !navigation && !dialogs::blocked(ui.ctx());
        let mut choose = None;
        let overlay_rect = egui::Rect::from_center_size(
            viewport.center_top() + egui::vec2(0.0, 34.0), egui::vec2(76.0, 46.0));
        let mut overlay = ui.new_child(egui::UiBuilder::new()
            .id_salt(("notation-reorder", self.id)).max_rect(overlay_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)));
        overlay.set_clip_rect(viewport.intersect(ui.clip_rect()));
        appearance::floating_frame(ui.ctx()).show(&mut overlay, |ui| {
            ui.set_width(54.0);
            ui.spacing_mut().interact_size = egui::Vec2::splat(24.0);
            ui.spacing_mut().button_padding = egui::vec2(4.0, 4.0);
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.horizontal(|ui| {
                for (later, label, icon, edits) in [(false, "Move earlier", ARROW_UP, &up), (true, "Move later", ARROW_DOWN, &down)] {
                    let item = toolbar::Item { label, icon: Some(icon), enabled: enabled && edits.is_ok(), separator_before: false };
                    let response = toolbar::button(ui, &item, false);
                    if response.clicked() { choose = Some(later); }
                    if let Err(error) = edits { response.on_disabled_hover_text(error); }
                }
            });
        });
        let owned = ui.input(|i| i.pointer.hover_pos()).is_some_and(|p| overlay.min_rect().contains(p));
        if let Some(later) = choose && let Some((_, id, revision)) = selected && let Some(record) = record
            && revision == self.document.revision() {
            let edits = if later { down } else { up };
            if let Ok(edits) = edits {
                match self.reordered_line(&record, id.as_deref(), later, &edits) {
                    Ok(line) => {
                        let id = id.map(|id| if id.starts_with("line:") { format!("line:{line}") } else { id });
                        self.submit_command(Command { edits, label: "Reorder construct".into(), expectation: worker::Expectation::Notation { line: Some(line), id } });
                    }
                    Err(error) => self.command_error = Some(error),
                }
            }
        }
        owned
    }
    fn reordered_line(&self, record: &edit::Record, id: Option<&str>, down: bool, edits: &[laml_ui::document::Edit]) -> Result<usize, String> {
        let candidate = edit::apply(self.document.source(), edits)?;
        if self.notation() == Notation::Timing {
            let old = laml::timing::parse(self.document.source()).map_err(|e| e.message)?;
            let owner = id.or_else(|| old.lifelines.iter().find(|p| p.line == record.line).map(|p| p.id.as_str()));
            let doc = laml::timing::parse(&candidate).map_err(|e| e.message)?;
            return doc.lifelines.iter().find(|p| Some(p.id.as_str()) == owner).map(|p| p.line).ok_or_else(|| "The reordered lifeline could not be selected".into());
        }
        let patch = edits.first().ok_or("No reordering change")?;
        // Reorder swaps whole sibling blocks. The moved block is either the
        // beginning or end of that replacement, even for identical messages.
        let at = patch.range.start + if down { patch.replacement.len() - record.block.len() } else { 0 };
        Ok(candidate[..at].bytes().filter(|b| *b == b'\n').count() + 1)
    }
}
