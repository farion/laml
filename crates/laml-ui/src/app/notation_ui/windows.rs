use super::*;
impl Editor {
    pub(in crate::app) fn notation_windows(&mut self, ctx: &egui::Context) {
        self.relation_choice_window(ctx);
        self.sequence_window(ctx);
        self.bpmn_inline_window(ctx);
        if self.notation_ui.navigator {
            let mut choose = None;
            let dismissed = dialogs::show(
                ctx,
                ("notation-elements", self.id),
                "Elements",
                480.0,
                |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(480.0)
                        .show(ui, |ui| {
                            let records = edit::records(self.document.source());
                            for section in ["semantics", "layout", "styling"] {
                                ui.strong(crate::i18n::tr(section));
                                for r in records.iter().filter(|r| r.section == section) {
                                    let depth = r.parent.map_or(0.0, |_| 16.0);
                                    ui.horizontal(|ui| {
                                        ui.add_space(depth);
                                        if ui
                                            .selectable_label(
                                                self.notation_ui
                                                    .selected
                                                    .as_ref()
                                                    .is_some_and(|s| s.0 == r.line),
                                                &r.heading,
                                            )
                                            .clicked()
                                        {
                                            choose = Some(r.line);
                                        }
                                    });
                                }
                            }
                        });
                },
            );
            if let Some(line) = choose {
                self.notation_select(line, None);
                self.notation_ui.navigator = false;
            }
            if dismissed {
                self.notation_ui.navigator = false;
            }
        }
        if let Some(mut creator) = self.notation_ui.create.take() {
            let recipes = edit::catalog::recipes(self.notation());
            if creator.recipe >= recipes.len() {
                return;
            }
            let mut cancel = false;
            let mut submit = false;
            let dismissed = dialogs::show(
                ctx,
                ("new-construct", self.id),
                "Add construct",
                440.0,
                |ui| {
                    let old = creator.recipe;
                    egui::ComboBox::from_id_salt("construct-kind")
                        .width(340.0)
                        .selected_text(crate::i18n::tr(recipes[creator.recipe].title))
                        .show_ui(ui, |ui| {
                            for (i, recipe) in recipes.iter().enumerate() {
                                ui.selectable_value(
                                    &mut creator.recipe,
                                    i,
                                    crate::i18n::tr(recipe.title),
                                );
                            }
                        });
                    if old != creator.recipe {
                        if recipes[old].section != recipes[creator.recipe].section {
                            creator.parent = None;
                        }
                        creator.values = self.construct_defaults(recipes[creator.recipe]);
                        creator.error = None;
                        creator.before = None;
                    }
                    let recipe = recipes[creator.recipe];
                    let pool = self.notation() == Notation::Bpmn && recipe.title == "Pool";
                    if pool {
                        creator.parent = None;
                        creator.before = None;
                    }
                    ui.add_space(12.0);
                    egui::ScrollArea::vertical()
                        .max_height(380.0)
                        .show(ui, |ui| {
                            for key in edit::catalog::parameters(recipe) {
                                ui.label(crate::i18n::tr(&key.replace('_', " ")));
                                let field = self.construct_value(
                                    ui,
                                    &key,
                                    creator.values.entry(key.clone()).or_default(),
                                    creator.parent,
                                );
                                self.source_ui.extra_drafts.push(field);
                            }
                            ui.add_space(12.0);
                            if pool {
                                let first = laml::bpmn::parse(self.document.source())
                                    .is_ok_and(|doc| doc.pools.is_empty() && !doc.nodes.is_empty());
                                ui.label(if first {
                                    crate::i18n::tr(
                                        "The existing process will be placed in this pool.",
                                    )
                                } else {
                                    crate::i18n::tr("Adds a pool at the diagram root.")
                                });
                            } else {
                                ui.label(crate::i18n::tr("Insert into"));
                                destination(
                                    ui,
                                    self.document.source(),
                                    recipe.section,
                                    &mut creator.parent,
                                    &mut creator.before,
                                    None,
                                );
                            }
                        });
                    if let Some(error) = &creator.error {
                        command_errors::details(ui, error);
                    }
                    let preflight = edit::catalog::create(
                        self.document.source(),
                        recipe,
                        &creator.values,
                        creator.parent,
                        creator.before,
                    )
                    .and_then(|edits| edit::apply(self.document.source(), &edits))
                    .and_then(|source| laml_ui::notation::inspect(&source).map(|_| ()));
                    if let Err(error) = &preflight {
                        ui.label(crate::i18n::message(error));
                    }
                    ui.horizontal(|ui| {
                        submit = ui
                            .add_enabled(
                                preflight.is_ok()
                                    && self.can_edit_visually()
                                    && creator.revision == self.document.revision(),
                                egui::Button::new(crate::i18n::tr("Add")),
                            )
                            .clicked();
                        cancel = ui.button(crate::i18n::tr("Cancel")).clicked();
                    });
                    if creator.revision != self.document.revision() {
                        ui.label(crate::i18n::tr("Document changed. Reopen this dialog."));
                    }
                },
            );
            if submit {
                let recipe = recipes[creator.recipe];
                let result = edit::catalog::create(
                    self.document.source(),
                    recipe,
                    &creator.values,
                    creator.parent,
                    creator.before,
                );
                match result {
                    Ok(edits) => {
                        self.notation_submit(&format!("Add {}", recipe.title), edits, None);
                        cancel = true;
                    }
                    Err(e) => creator.error = Some(e),
                }
            }
            if !cancel && !dismissed {
                self.notation_ui.create = Some(creator);
            }
        }
        if let Some((revision, plan)) = self.notation_ui.deletion.take() {
            let mut confirm = false;
            let mut cancel = false;
            let dismissed = dialogs::show(
                ctx,
                ("notation-delete", self.id),
                "Confirm deletion",
                480.0,
                |ui| {
                    ui.label(crate::i18n::tr("This change affects:"));
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for affected in &plan.affected {
                                ui.label(affected);
                            }
                        });
                    ui.horizontal(|ui| {
                        confirm = ui
                            .add_enabled(
                                self.can_edit_visually() && revision == self.document.revision(),
                                egui::Button::new(crate::i18n::tr("Delete")),
                            )
                            .clicked();
                        cancel = ui.button(crate::i18n::tr("Cancel")).clicked();
                    });
                    if revision != self.document.revision() {
                        ui.label(crate::i18n::tr(
                            "Document changed. Reopen the deletion preview.",
                        ));
                    }
                },
            );
            if confirm {
                self.notation_submit("Delete constructs", plan.edits, None);
            } else if !cancel && !dismissed {
                self.notation_ui.deletion = Some((revision, plan));
            }
        }
    }
}
