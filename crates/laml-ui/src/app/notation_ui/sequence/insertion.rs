//! Shared direct-insertion menu and cancellable, canvas-based reference picking.
use super::*;
use std::collections::BTreeMap;

pub(super) struct Picker {
    query: insertion_options::Query,
}
impl Editor {
    pub(super) fn sequence_begin_insert(
        &mut self,
        recipe: edit::catalog::Recipe,
        point: Option<Point>,
    ) {
        self.notation_ui.sequence.insert = None;
        self.notation_ui.sequence.picker = Some(Picker {
            query: insertion_options::query(self, recipe, point.as_ref()),
        });
    }
    pub(super) fn sequence_pick_reference(
        &mut self,
        ui: &mut egui::Ui,
        scene: &Scene,
        pointer: Option<egui::Pos2>,
        response: &egui::Response,
    ) -> bool {
        let Some(picker) = &self.notation_ui.sequence.picker else {
            return false;
        };
        let query = picker.query.clone();
        let prompt = |action: String| {
            format!(
                "{} — {action} · Esc: {}",
                crate::i18n::tr(&query.title),
                crate::i18n::tr("Cancel")
            )
        };
        let valid = match self.sequence_valid_options(ui.ctx(), vec![query.clone()], true) {
            Ok(Some(choices)) => choices,
            Ok(None) => {
                reference_picker::hint(
                    ui,
                    prompt(crate::i18n::tr("Checking reference points…").into()),
                );
                ui.ctx().set_cursor_icon(egui::CursorIcon::Progress);
                return true;
            }
            Err(_) => {
                reference_picker::hint(
                    ui,
                    prompt(crate::i18n::tr("Could not check available actions.").into()),
                );
                return true;
            }
        };
        let Some(choice) = valid.first() else {
            reference_picker::hint(
                ui,
                prompt(crate::i18n::tr("No valid reference points.").into()),
            );
            return true;
        };
        if let Some(edits) = &choice.edits {
            self.notation_ui.sequence.picker = None;
            self.sequence_apply(Proposal {
                edits: edits.clone(),
                line: None,
                label: "Add sequence construct",
            });
            return true;
        }
        let key = &query.keys[0].0;
        let scope = query
            .owner
            .as_ref()
            .and_then(|owner| owner.rsplit_once('.').map(|(s, _)| s));
        let candidate = reference_picker::show(ui, scene, key, scope, &choice.next, pointer);
        reference_picker::hint(
            ui,
            prompt(format!(
                "{} {}",
                crate::i18n::tr("Select"),
                crate::i18n::tr(key)
            )),
        );
        if ui.input(|i| i.pointer.primary_clicked())
            && response.contains_pointer()
            && let Some(value) = candidate
        {
            let picker = self.notation_ui.sequence.picker.as_mut().unwrap();
            let (key, _) = picker.query.keys.remove(0);
            picker.query.values.insert(key, value);
        }
        true
    }
}
pub(super) fn defaults(editor: &Editor, recipe: edit::catalog::Recipe) -> BTreeMap<String, String> {
    let mut values = editor.construct_defaults(recipe);
    let label = match recipe.title {
        "Time observation" => "t",
        "Duration observation" => "d",
        "Time constraint" | "Duration constraint" => "{0..1}",
        "State annotation" => "State",
        "Invariant" => "{condition}",
        "Action annotation" => "Action",
        _ => recipe.title,
    };
    values.insert("label".into(), label.into());
    values
}
