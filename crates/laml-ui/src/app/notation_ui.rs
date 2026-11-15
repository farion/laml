//! Diagram-specific authoring controls share the source transaction pipeline.
use super::*;
use laml_ui::{notation::Notation, notation_edit as edit};
mod bpmn;
mod bpmn_canvas;
mod canvas;
mod graph_fields;
mod relations;
mod reorder;
mod sequence;
mod settings;
mod structure;
mod styles;
mod suggestions;
mod windows;

#[derive(Default)]
pub(super) struct State {
    pub selected: Option<(usize, Option<String>, u64)>,
    pub navigator: bool,
    pub create: Option<Creator>,
    pub deletion: Option<(u64, edit::Deletion)>,
    pub drag: Option<(String, String, usize, u64, egui::Pos2)>,
    pub connect: bool,
    sequence: sequence::State,
    bpmn: bpmn_canvas::State,
    relation: Option<relations::Choice>,
    pub move_parent: Option<usize>,
    pub move_before: Option<usize>,
}
pub(super) struct Creator {
    revision: u64,
    recipe: usize,
    values: std::collections::BTreeMap<String, String>,
    parent: Option<usize>,
    before: Option<usize>,
    error: Option<String>,
}
impl Editor {
    pub(super) fn notation(&self) -> Notation {
        self.preview
            .as_ref()
            .map_or_else(|| Notation::of(self.document.source()), |p| p.notation)
    }
    pub(super) fn specialized(&self) -> bool {
        !self.notation().graph()
    }
    pub(super) fn notation_select(&mut self, line: usize, id: Option<String>) {
        self.notation_ui.selected = Some((line, id, self.document.revision()));
        self.selection = Selection::default();
        self.property_drafts.clear();
        self.source_jump = Some(line);
    }
    pub(super) fn notation_submit(
        &mut self,
        label: &str,
        edits: Vec<laml_ui::document::Edit>,
        line: Option<usize>,
    ) {
        let target = line
            .map(|line| laml_ui::relation_edit::remap_line(self.document.source(), line, &edits));
        let id = self
            .notation_ui
            .selected
            .as_ref()
            .and_then(|(_, id, _)| id.clone());
        self.submit_command(Command {
            edits,
            label: label.into(),
            expectation: worker::Expectation::Notation { line: target, id },
        });
    }
    pub(super) fn open_construct_dialog(&mut self) {
        if !self.can_edit_visually() {
            return;
        }
        let recipes = edit::catalog::recipes(self.notation());
        if recipes.is_empty() {
            return;
        }
        let selected = self.notation_ui.selected.as_ref().map(|s| s.0).or_else(|| {
            self.selection.ids.first().and_then(|id| {
                self.preview
                    .as_ref()?
                    .diagram
                    .nodes
                    .iter()
                    .find(|n| &n.id == id)
                    .map(|n| n.line)
            })
        });
        let parent = selected
            .and_then(|line| edit::record(self.document.source(), line).ok())
            .and_then(|r| {
                if r.opens || body_capable(&r) {
                    Some(r.line)
                } else {
                    r.parent
                }
            });
        let recipe = if self.notation() == Notation::InteractionOverview {
            recipes
                .iter()
                .position(|r| r.title == "Insert interaction on flow")
                .unwrap_or(0)
        } else {
            0
        };
        self.notation_ui.create = Some(Creator {
            revision: self.document.revision(),
            recipe,
            values: self.construct_defaults(recipes[recipe]),
            parent,
            before: None,
            error: None,
        });
    }
    fn construct_defaults(
        &self,
        recipe: edit::catalog::Recipe,
    ) -> std::collections::BTreeMap<String, String> {
        let source = self.document.source();
        let ids: Vec<_> = self
            .preview
            .as_ref()
            .map(|p| {
                if p.notation.graph() {
                    p.diagram.nodes.iter().map(|n| n.id.clone()).collect()
                } else {
                    p.regions
                        .iter()
                        .filter(|r| matches!(r.kind.as_str(), "node" | "participant" | "lifeline"))
                        .map(|r| r.id.clone())
                        .collect()
                }
            })
            .unwrap_or_default();
        let edge = self.selection.relations.first().and_then(|line| {
            self.preview
                .as_ref()?
                .diagram
                .relations
                .iter()
                .find(|r| &r.line == line)
        });
        let fresh = (1..)
            .map(|i| format!("Element{i}"))
            .find(|id| !source.contains(id))
            .unwrap();
        edit::catalog::parameters(recipe)
            .into_iter()
            .map(|key| {
                let value = match key.as_str() {
                    "id" => fresh.clone(),
                    "from" | "participant" => edge
                        .map(|r| r.from.clone())
                        .or_else(|| self.selection.ids.first().cloned())
                        .or_else(|| ids.first().cloned())
                        .unwrap_or_else(|| "A".into()),
                    "to" => edge
                        .map(|r| r.to.clone())
                        .or_else(|| self.selection.ids.get(1).cloned())
                        .or_else(|| ids.get(1).or(ids.first()).cloned())
                        .unwrap_or_else(|| "B".into()),
                    "label" => recipe.title.into(),
                    "reply" => "Response".into(),
                    "description" => "Description".into(),
                    "technology" => "Technology".into(),
                    "classifier" => "Class".into(),
                    "other_interaction" => "OtherInteraction".into(),
                    "name" => "member".into(),
                    "time" => "1".into(),
                    "state" => "Active".into(),
                    "minimum" => "1".into(),
                    "maximum" => "3".into(),
                    "number" => "1".into(),
                    "source_side" => "right".into(),
                    "target_side" => "left".into(),
                    "orientation" => "auto".into(),
                    "selector" => "node".into(),
                    "property" => {
                        if self.notation() == Notation::Timing {
                            "border-color".into()
                        } else {
                            "fill".into()
                        }
                    }
                    "parameter" => "T".into(),
                    "source_multiplicity" => "1".into(),
                    "target_multiplicity" => "0..*".into(),
                    "source_role" => "owner".into(),
                    "target_role" => "items".into(),
                    "value" => {
                        if recipe.section == "styling" {
                            "#eeeeee".into()
                        } else {
                            "Value".into()
                        }
                    }
                    "child" => "Child".into(),
                    "other" => "Other".into(),
                    "start" => "Start".into(),
                    "effect" => "effect".into(),
                    "trigger" => "event".into(),
                    "messages" => "message".into(),
                    "message" => "Message".into(),
                    "gap" => "80".into(),
                    "scale" => "100".into(),
                    "condition" => "condition".into(),
                    "otherwise" => "otherwise".into(),
                    "alternative" => "Alternative".into(),
                    _ => String::new(),
                };
                (key, value)
            })
            .collect()
    }
    pub(super) fn notation_inspector(&mut self, ui: &mut egui::Ui) {
        let selected = self
            .notation_ui
            .selected
            .clone()
            .filter(|s| s.2 == self.document.revision());
        if let Some((line, id, _)) = selected {
            let Ok(record) = edit::record(self.document.source(), line) else {
                return;
            };
            inspector_ui::category(
                ui,
                &format!("{} · {}", self.notation().family(), self.notation().title()),
            );
            if self.inspector_tab == inspector_ui::Tab::Style {
                self.notation_style_fields(ui, line, id.as_deref());
                return;
            }
            ui.add_enabled_ui(self.can_edit_visually(), |ui| {
                self.notation_identity(ui, line, id.as_deref());
                let handled = self.notation() == Notation::Bpmn
                    && id
                        .as_deref()
                        .is_some_and(|id| self.bpmn_properties(ui, id, line));
                for (i, field) in edit::item_fields(self.document.source(), line, id.as_deref())
                    .iter()
                    .enumerate()
                {
                    if handled
                        || (field.name == "Name / reference"
                            && id.as_deref().is_some_and(|id| {
                                field.value == id
                                    || id.rsplit('.').next() == Some(field.value.as_str())
                            }))
                    {
                        continue;
                    }
                    let key = format!("notation-{line}-{i}");
                    let mut value = self
                        .property_drafts
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| field.value.clone());
                    let mut commit = false;
                    inspector_ui::row(ui, &field.name, |ui| {
                        if let edit::fields::Kind::Choice(choices) = &field.kind {
                            let old = value.clone();
                            egui::ComboBox::from_id_salt(&key)
                                .width(ui.available_width())
                                .selected_text(notation_choice_title(&field.name, &value))
                                .show_ui(ui, |ui| {
                                    for choice in choices {
                                        ui.selectable_value(
                                            &mut value,
                                            choice.clone(),
                                            notation_choice_title(&field.name, choice),
                                        );
                                    }
                                });
                            commit = value != old;
                        } else {
                            let multiline = matches!(field.kind, edit::fields::Kind::Quoted)
                                || (self.notation() == Notation::Sequence && field.name == "Label");
                            let field_ui = inspector_ui::field(ui, &key, &mut value, multiline);
                            self.source_ui.extra_drafts.push(field_ui.response.id);
                            commit = field_ui.commit;
                        }
                    });
                    self.property_drafts.insert(key, value.clone());
                    if commit && value != field.value {
                        match edit::fields::edit(self.document.source(), field, &value) {
                            Ok(edits) => {
                                self.notation_submit("Change construct property", edits, Some(line))
                            }
                            Err(e) => self.command_error = Some(e),
                        }
                    }
                }
                if !handled {
                    self.notation_optional_fields(ui, line, id.as_deref());
                }
                ui.add_space(12.0);
                inspector_ui::category(ui, "Structure");
                self.notation_structure(ui, &record, id.as_deref());
            });
        } else {
            inspector_ui::category(ui, "Diagram");
            inspector_ui::content(ui, |ui| {
                ui.label(format!(
                    "{} · {}",
                    self.notation().family(),
                    self.notation().title()
                ));
                if ui.button("Add construct…").clicked() {
                    self.open_construct_dialog();
                }
                if ui.button("Elements…").clicked() {
                    self.notation_ui.navigator = true;
                }
            });
            self.notation_document_settings(ui);
        }
    }
    fn notation_identity(&mut self, ui: &mut egui::Ui, line: usize, id: Option<&str>) {
        if self.notation() == Notation::Sequence && id.is_none_or(|id| id.starts_with("line:")) {
            let probe = (1..)
                .map(|n| format!("Construct{n}"))
                .find(|name| !self.document.source().contains(name))
                .unwrap();
            if edit::sequence::assign_name(self.document.source(), line, &probe).is_err() {
                return;
            }
            let key = format!("name-construct-{line}");
            let mut name = self.property_drafts.get(&key).cloned().unwrap_or_default();
            let mut commit = false;
            inspector_ui::row(ui, "Name", |ui| {
                let field = inspector_ui::field(ui, &key, &mut name, false);
                self.source_ui.extra_drafts.push(field.response.id);
                commit = field.commit;
            });
            self.property_drafts.insert(key, name.clone());
            if commit && !name.trim().is_empty() {
                match edit::sequence::assign_name(self.document.source(), line, &name) {
                    Ok(edits) => self.submit_command(Command {
                        edits,
                        label: "Name construct".into(),
                        expectation: worker::Expectation::Notation {
                            line: Some(line),
                            id: Some(name),
                        },
                    }),
                    Err(error) => self.command_error = Some(error),
                }
            }
            return;
        }
        let Some(id) = id.filter(|id| {
            !id.starts_with("line:") && !id.starts_with("flow:") && !id.starts_with("decompose:")
        }) else {
            return;
        };
        let short = id.rsplit('.').next().unwrap();
        let key = format!("notation-id-{id}");
        let mut value = self
            .property_drafts
            .get(&key)
            .cloned()
            .unwrap_or_else(|| short.into());
        let mut commit = false;
        inspector_ui::row(ui, "ID", |ui| {
            let f = inspector_ui::field(ui, &key, &mut value, false);
            self.source_ui.extra_drafts.push(f.response.id);
            commit = f.commit;
        });
        self.property_drafts.insert(key, value.clone());
        if commit && value != short {
            match edit::rename(self.document.source(), id, line, &value) {
                Ok(edits) => {
                    let new_id = id
                        .rsplit_once('.')
                        .map_or(value.clone(), |(p, _)| format!("{p}.{value}"));
                    self.submit_command(Command {
                        edits,
                        label: "Rename element".into(),
                        expectation: worker::Expectation::Notation {
                            line: Some(line),
                            id: Some(new_id),
                        },
                    });
                }
                Err(e) => self.command_error = Some(e),
            }
        }
    }
    pub(super) fn notation_delete(&mut self, line: usize, id: Option<&str>, keep: bool) {
        let result = if self.notation() == Notation::Bpmn
            && !keep
            && id.is_some_and(|id| {
                id.starts_with("flow:")
                    || self
                        .preview
                        .as_ref()
                        .is_some_and(|p| p.regions.iter().any(|r| r.id == id && r.kind == "node"))
            }) {
            edit::bpmn::delete_item(self.document.source(), id.unwrap())
        } else {
            edit::delete(self.document.source(), line, id, keep)
        };
        match result {
            Ok(plan) => self.notation_ui.deletion = Some((self.document.revision(), plan)),
            Err(e) => self.command_error = Some(e),
        }
    }
}
fn destination(
    ui: &mut egui::Ui,
    source: &str,
    section: &str,
    parent: &mut Option<usize>,
    before: &mut Option<usize>,
    exclude: Option<usize>,
) {
    let records = edit::records(source);
    egui::ComboBox::from_id_salt("construct-parent")
        .selected_text(
            parent
                .and_then(|line| records.iter().find(|r| r.line == line))
                .map_or("Root", |r| r.heading.as_str()),
        )
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            ui.selectable_value(parent, None, "Root");
            for r in records.iter().filter(|r| {
                r.section == section && (r.opens || body_capable(r)) && Some(r.line) != exclude
            }) {
                ui.selectable_value(parent, Some(r.line), &r.heading);
            }
        });
    egui::ComboBox::from_id_salt("construct-before")
        .selected_text(
            before
                .and_then(|line| records.iter().find(|r| r.line == line))
                .map_or("At end", |r| r.heading.as_str()),
        )
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            ui.selectable_value(before, None, "At end");
            for r in records
                .iter()
                .filter(|r| r.section == section && r.parent == *parent && Some(r.line) != exclude)
            {
                ui.selectable_value(before, Some(r.line), format!("Before {}", r.heading));
            }
        });
}

fn body_capable(r: &edit::Record) -> bool {
    r.section == "semantics"
        && [
            "class ",
            "interface ",
            "enum ",
            "object ",
            "datatype ",
            "component ",
            "state ",
            "action ",
            "node ",
            "device ",
            "executionEnvironment ",
            "package ",
            "profile ",
            "structure ",
            "stereotype ",
        ]
        .iter()
        .any(|prefix| r.heading.starts_with(prefix))
}

fn notation_choice_title<'a>(field: &str, value: &'a str) -> &'a str {
    if field.ends_with("activation / lifetime") {
        match value {
            "" => "Unchanged",
            "+" => "Start activation / create if new",
            "-" => "End activation",
            "!" => "Destroy participant",
            _ => value,
        }
    } else {
        value
    }
}

#[cfg(test)]
mod tests;
