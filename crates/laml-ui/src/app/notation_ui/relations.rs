//! The configured default is attempted first. Invalid endpoint combinations ask,
//! rather than silently substituting a different relationship.
use super::*;
pub(super) struct Choice {
    pub revision: u64,
    pub from: String,
    pub to: String,
    pub operator: String,
    pub label: String,
    pub reason: String,
    pub from_side: Option<String>,
    pub to_side: Option<String>,
}
impl Editor {
    fn relation_candidate(
        &self,
        choice: &Choice,
        operator: &str,
    ) -> Result<Vec<laml_ui::document::Edit>, String> {
        let source = self.document.source();
        let edits = if self.notation().graph() {
            laml_ui::graph_edit::add_relation_labeled(
                source,
                &choice.from,
                &choice.to,
                operator,
                Some(&choice.label),
                choice.from_side.as_deref(),
                choice.to_side.as_deref(),
            )?
            .1
        } else {
            let mut parent = None;
            let (mut from, mut to) = (choice.from.as_str(), choice.to.as_str());
            if self.notation() == Notation::Sequence && (from.contains('.') || to.contains('.')) {
                let scope = |id: &str| {
                    self.preview
                        .as_ref()
                        .and_then(|p| {
                            p.regions
                                .iter()
                                .find(|r| r.id == id && r.kind == "participant")
                        })
                        .and_then(|r| edit::record(source, r.line).ok())
                        .and_then(|r| r.parent)
                };
                let a = scope(from);
                let b = scope(to);
                if a != b || a.is_none() {
                    return Err("Messages must stay inside the same inline decomposition".into());
                }
                parent = a;
                from = from.rsplit('.').next().unwrap();
                to = to.rsplit('.').next().unwrap();
            }
            let mut declaration = format!("{from} {operator} {to}");
            if !choice.label.is_empty() {
                let label = if self.notation() == Notation::Bpmn {
                    choice.label.clone()
                } else {
                    laml_ui::relation_edit::quote(&choice.label)
                };
                if label.contains(['\r', '\n']) {
                    return Err("Enter a single flow label".into());
                }
                declaration.push_str(&format!(" : {label}"));
            }
            edit::insert(source, "semantics", parent, None, &declaration)?
        };
        let candidate = edit::apply(source, &edits)?;
        laml_ui::notation::inspect(&candidate)?;
        Ok(edits)
    }
    pub(in crate::app) fn connect_notation_default(
        &mut self,
        from: &str,
        to: &str,
        operator: &str,
        from_side: Option<&str>,
        to_side: Option<&str>,
    ) {
        let mut choice = Choice {
            revision: self.document.revision(),
            from: from.into(),
            to: to.into(),
            operator: operator.into(),
            label: String::new(),
            reason: String::new(),
            from_side: from_side.map(str::to_owned),
            to_side: to_side.map(str::to_owned),
        };
        match self.relation_candidate(&choice, operator) {
            Ok(edits) => self.submit_relationship_edits(edits),
            Err(reason) => {
                choice.reason = reason;
                // A C4 relation requires a meaningful description, not a silent
                // placeholder. The user supplies it in the same fallback dialog.
                self.notation_ui.relation = Some(choice);
            }
        }
    }
    fn submit_relationship_edits(&mut self, edits: Vec<laml_ui::document::Edit>) {
        if self.notation().graph()
            && let (Ok(before), Ok(source)) = (
                laml_ui::notation::parse_graph(self.document.source()),
                edit::apply(self.document.source(), &edits),
            )
            && let Ok(after) = laml_ui::notation::parse_graph(&source)
            && let Some(id) = after
                .relations
                .iter()
                .filter_map(|r| r.name.as_ref())
                .find(|id| {
                    !before
                        .relations
                        .iter()
                        .any(|r| r.name.as_ref() == Some(*id))
                })
        {
            self.submit_command(Command {
                edits,
                label: "Add relationship".into(),
                expectation: worker::Expectation::Relation(id.clone()),
            });
            return;
        }
        self.notation_submit("Add relationship", edits, None);
    }
    pub(super) fn relation_choice_window(&mut self, ctx: &egui::Context) {
        let Some(mut choice) = self.notation_ui.relation.take() else {
            return;
        };
        let mut cancel = false;
        let mut submit = false;
        let mut selected_valid = false;
        let dismissed = dialogs::show(
            ctx,
            ("notation-relation-choice", self.id),
            "Choose relationship",
            460.0,
            |ui| {
                ui.label(format!("{} → {}", choice.from, choice.to));
                ui.label("The default cannot connect these endpoints with the current properties.");
                ui.label("Label");
                self.source_ui
                    .extra_drafts
                    .push(ui.text_edit_singleline(&mut choice.label).id);
                ui.add_space(8.0);
                for relation in self.notation().relation_types() {
                    let operator = relation.key;
                    let valid = self.relation_candidate(&choice, operator);
                    if operator == choice.operator {
                        selected_valid = valid.is_ok();
                    }
                    let response = ui.add_enabled(
                        valid.is_ok(),
                        egui::Button::selectable(choice.operator == operator, relation.title),
                    );
                    if let Err(error) = valid {
                        response.clone().on_disabled_hover_text(error);
                    }
                    if response.clicked() {
                        choice.operator = operator.into();
                        selected_valid = true;
                    }
                }
                egui::CollapsingHeader::new("Details").show(ui, |ui| {
                    ui.label(&choice.reason);
                });
                ui.horizontal(|ui| {
                    submit = ui
                        .add_enabled(
                            selected_valid
                                && choice.revision == self.document.revision()
                                && self.can_edit_visually(),
                            egui::Button::new("Connect"),
                        )
                        .clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            },
        );
        if submit {
            match self.relation_candidate(&choice, &choice.operator) {
                Ok(edits) => {
                    self.submit_relationship_edits(edits);
                    cancel = true;
                }
                Err(e) => choice.reason = e,
            }
        }
        if !cancel && !dismissed {
            self.notation_ui.relation = Some(choice);
        }
    }
}
