//! Immediate contextual palette: local semantic checks, never layout or network work.
use super::*;
const COMMON: &[&str] = &[
    "Note",
    "Activation",
    "Optional fragment",
    "Alternative",
    "Loop",
    "Interaction reference",
];
pub(super) struct Cache {
    dock: Dock,
    expanded: bool,
    choices: Vec<insertion_options::Choice>,
}
fn group(recipe: edit::catalog::Recipe) -> usize {
    if recipe.section == "layout" {
        3
    } else if recipe.body.contains("{\n")
        && !matches!(
            recipe.title,
            "Inline decomposition" | "Coregion" | "Interaction reference with gates"
        )
    {
        1
    } else if recipe.body.starts_with("at[")
        || recipe.body.starts_with("between[")
        || recipe.body.starts_with("order[")
    {
        2
    } else {
        0
    }
}
impl Editor {
    pub(super) fn sequence_add_menu(&mut self, ui: &mut egui::Ui, point: &Point) {
        let recipes = recipes();
        let expanded = self.notation_ui.sequence.expanded;
        if !self
            .notation_ui
            .sequence
            .menu
            .as_ref()
            .is_some_and(|c| c.dock == point.dock && c.expanded == expanded)
        {
            let indices = recipes
                .iter()
                .enumerate()
                .filter(|(_, r)| expanded || COMMON.contains(&r.title))
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            let queries = indices
                .iter()
                .map(|&i| insertion_options::query(self, recipes[i], Some(point)))
                .collect::<Vec<_>>();
            let mut choices =
                edit::sequence_options::build_with(self.document.source(), &queries, false, |_| {
                    true
                });
            for choice in &mut choices {
                choice.index = indices[choice.index];
            }
            self.notation_ui.sequence.menu = Some(Cache {
                dock: point.dock.clone(),
                expanded,
                choices,
            });
        }
        let choices = self
            .notation_ui
            .sequence
            .menu
            .as_ref()
            .unwrap()
            .choices
            .clone();
        for title in COMMON {
            if let Some(choice) = choices.iter().find(|c| recipes[c.index].title == *title) {
                self.sequence_menu_choice(ui, point, recipes[choice.index], choice);
            }
        }
        let other = choices
            .iter()
            .filter(|c| !COMMON.contains(&recipes[c.index].title))
            .collect::<Vec<_>>();
        if !self.notation_ui.sequence.expanded {
            if recipes.iter().any(|r| !COMMON.contains(&r.title)) {
                ui.add_space(2.0);
                ui.separator();
                ui.add_space(2.0);
                if actions::menu_button(ui, "More…") {
                    self.notation_ui.sequence.expanded = true;
                    ui.ctx().request_repaint();
                }
            }
            return;
        }
        if !other.is_empty() {
            ui.add_space(2.0);
            ui.separator();
            ui.add_space(2.0);
        }
        for choice in other.iter().filter(|c| group(recipes[c.index]) == 0) {
            self.sequence_menu_choice(ui, point, recipes[choice.index], choice);
        }
        for (index, title) in [(1, "Fragments"), (2, "Annotations"), (3, "Layout")] {
            let items = other
                .iter()
                .filter(|c| group(recipes[c.index]) == index)
                .collect::<Vec<_>>();
            if items.is_empty() {
                continue;
            }
            ui.menu_button(actions::menu_text(ui, title), |ui| {
                ui.spacing_mut().button_padding = egui::vec2(16.0, 6.0);
                for choice in items {
                    self.sequence_menu_choice(ui, point, recipes[choice.index], choice);
                }
            });
        }
    }
    fn sequence_menu_choice(
        &mut self,
        ui: &mut egui::Ui,
        point: &Point,
        recipe: edit::catalog::Recipe,
        choice: &insertion_options::Choice,
    ) {
        if actions::menu_button(ui, recipe.title) {
            self.notation_ui.sequence.insert = None;
            if let Some(edits) = &choice.edits {
                self.sequence_apply(Proposal {
                    edits: edits.clone(),
                    line: None,
                    label: "Add sequence construct",
                });
            } else {
                self.sequence_begin_insert(recipe, Some(point.clone()));
            }
            ui.close();
        }
    }
}
