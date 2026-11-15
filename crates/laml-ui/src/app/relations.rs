use super::*;
use laml::syntax::{EndPosition, PortSide};
use laml_ui::relation_edit;

impl Editor {
    pub(super) fn relation_inspector(&mut self, ui: &mut egui::Ui) {
        if !self.selection.ids.is_empty() || !self.selection.ports.is_empty() {
            ui.label("Select only connections to edit their properties.");
            return;
        }
        let Some(preview) = &self.preview else {
            return;
        };
        let lines = self.selection.relations.clone();
        let relations: Vec<_> = lines
            .iter()
            .filter_map(|line| preview.diagram.relations.iter().find(|r| r.line == *line))
            .cloned()
            .collect();
        let Some(first) = relations.first() else {
            ui.label("Note attachment");
            return;
        };
        let enabled = self.can_edit_visually();
        if self.inspector_tab == inspector_ui::Tab::Style {
            let values = preview.relation_properties.clone();
            for (title, names) in [
                ("Appearance", &["color", "base-color", "width"][..]),
                ("Typography", &["font-size"][..]),
                ("Visibility", &["display"][..]),
            ] {
                inspector_ui::category(ui, title);
                for name in names {
                    let selected: Vec<_> = lines
                        .iter()
                        .filter_map(|l| values.get(l).and_then(|v| v.get(*name)))
                        .collect();
                    if selected.len() != lines.len() {
                        continue;
                    }
                    let original = selected[0].value.clone().unwrap_or_default();
                    let mixed = selected.iter().any(|v| v.value != selected[0].value);
                    let mut commit = false;
                    let mut reset = false;
                    let draft = self
                        .property_drafts
                        .entry(format!("relation-{name}"))
                        .or_insert_with(|| {
                            if mixed {
                                String::new()
                            } else {
                                original.clone()
                            }
                        });
                    ui.add_enabled_ui(enabled, |ui| {
                        inspector_ui::row(ui, property_rows::property_title(name), |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                let rgb = if name.ends_with("color") {
                                    inspector::parse_rgb(draft)
                                } else {
                                    None
                                };
                                let width = (ui.available_width()
                                    - 30.0
                                    - if rgb.is_some() {
                                        appearance::FIELD_HEIGHT + 6.0
                                    } else {
                                        0.0
                                    })
                                .max(32.0);
                                ui.scope_builder(
                                    egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                                        ui.cursor().min,
                                        egui::vec2(width, appearance::FIELD_HEIGHT),
                                    )),
                                    |ui| {
                                        let field = inspector_ui::mixed_field(
                                            ui,
                                            format!("relation-{name}"),
                                            draft,
                                            false,
                                            mixed,
                                        );
                                        self.source_ui.extra_drafts.push(field.response.id);
                                        commit |= field.commit;
                                    },
                                );
                                if let Some(mut rgb) = rgb
                                    && ui.color_edit_button_srgb(&mut rgb).changed()
                                {
                                    *draft = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
                                    commit = true;
                                }
                                reset = appearance::property_action(ui, true).clicked();
                            });
                        });
                    });
                    let value = draft.clone();
                    if reset {
                        self.relation_property(name, None);
                    } else if commit && (mixed || value != original) {
                        self.relation_property(name, Some(&value));
                    }
                }
            }
            return;
        }
        let endpoints = laml_ui::structure::endpoints(&preview.diagram);
        let end_sides: Vec<_> = [EndPosition::Source, EndPosition::Target]
            .into_iter()
            .map(|end| {
                let sides: Vec<_> = relations
                    .iter()
                    .map(|r| {
                        preview
                            .diagram
                            .relation_end_layout
                            .iter()
                            .find(|rule| r.name.as_ref() == Some(&rule.name) && rule.end == end)
                            .and_then(|rule| rule.side)
                    })
                    .collect();
                let side = if sides.iter().any(|s| s != &sides[0]) {
                    "Mixed"
                } else {
                    match sides[0] {
                        None => "auto",
                        Some(PortSide::Left) => "left",
                        Some(PortSide::Right) => "right",
                        Some(PortSide::Top) => "top",
                        Some(PortSide::Bottom) => "bottom",
                    }
                };
                (end, side)
            })
            .collect();
        let modes: Vec<_> = relations
            .iter()
            .map(|r| {
                preview
                    .diagram
                    .relation_layout
                    .iter()
                    .find(|rule| r.name.as_ref() == Some(&rule.name))
                    .map(|rule| match rule.placement {
                        laml::syntax::LabelPlacement::Near => "near",
                        laml::syntax::LabelPlacement::Along => "along",
                        laml::syntax::LabelPlacement::Inline => "inline",
                    })
                    .unwrap_or("default")
            })
            .collect();
        let mode = if modes.iter().all(|m| m == &modes[0]) {
            modes[0]
        } else {
            "Mixed"
        };
        ui.add_enabled_ui(enabled, |ui| {
            inspector_ui::category(ui, "Connection");
            for (title, end, original) in [
                ("Start", EndPosition::Source, &first.from),
                ("End", EndPosition::Target, &first.to),
            ] {
                let mixed = relations.iter().any(|r| {
                    (if end == EndPosition::Source { &r.from } else { &r.to }) != original
                });
                let before = if mixed { "Mixed" } else { original.as_str() };
                let mut value = before.to_owned();
                inspector_ui::row(ui, title, |ui| {
                    egui::ComboBox::from_id_salt(title)
                        .width(ui.available_width())
                        .selected_text(inspector_ui::value_text(&value))
                        .show_ui(ui, |ui| {
                            for endpoint in &endpoints {
                                ui.selectable_value(&mut value, endpoint.clone(), endpoint);
                            }
                        });
                });
                if value != before {
                    self.relation_edits("Reconnect relation", |source, line| {
                        relation_edit::reconnect(source, line, end, &value)
                    });
                }
            }

            let original = first.label.clone().unwrap_or_default();
            let mixed = relations.iter().any(|r| r.label != first.label);
            let draft = self
                .property_drafts
                .entry("relation-label".into())
                .or_insert_with(|| {
                    if mixed {
                        String::new()
                    } else {
                        original.clone()
                    }
                });
            let field = inspector_ui::row(ui, "Label", |ui| {
                inspector_ui::mixed_field(ui, "relation-label", draft, true, mixed)
            });
            self.source_ui.extra_drafts.push(field.response.id);
            let value = draft.clone();
            if field.commit && (mixed || value != original) {
                self.relation_edits("Change relation label", |source, line| {
                    relation_edit::label(source, line, &value)
                });
            }

            let operators: Vec<_> = lines
                .iter()
                .map(|l| relation_edit::operator(self.document.source(), *l).unwrap_or_default())
                .collect();
            if self.notation() == laml_ui::notation::Notation::Gne {
            let first_type = relation_edit::relation_type(&operators[0]);
            let original = if operators.iter().all(|op| relation_edit::relation_type(op) == first_type) {
                first_type
            } else {
                "Mixed"
            };
            let mut op = original.to_string();
            inspector_ui::row(ui, "Type", |ui| {
                relation_picker::combo(
                    ui, "relation-operator", &mut op,
                    self.notation().operators(), ui.available_width(),
                );
            });
            if op != original {
                self.relation_edits("Change relation operator", |source, line| {
                    relation_edit::change_type(source, line, &op)
                });
            }
            } else {
                let choices = self.notation().relation_types();
                let key = |r: &laml::syntax::RelationDecl, op: &str| {
                    if let Some(stereotype) = r.stereotypes.first() { stereotype.clone() }
                    else if self.notation() == laml_ui::notation::Notation::Activity {
                        match r.activity_flow { Some(laml::syntax::ActivityFlow::Object) => "object", _ => "control" }.into()
                    } else { relation_edit::relation_type(op).into() }
                };
                let selected = key(&relations[0], &operators[0]);
                let mixed = relations.iter().zip(&operators).any(|(r, op)| key(r, op) != selected);
                let mut chosen = selected.clone();
                inspector_ui::row(ui, "Type", |ui| {
                    egui::ComboBox::from_id_salt("semantic-relation-type").width(ui.available_width())
                        .selected_text(if mixed { "Mixed" } else { choices.iter().find(|c| c.key == selected).map_or("Relationship", |c| c.title) })
                        .show_ui(ui, |ui| {
                            for choice in choices {
                                let valid = lines.iter().all(|line| laml_ui::notation_edit::relationship_type(self.document.source(), *line, choice.key).is_ok());
                                if ui.add_enabled(valid, egui::Button::selectable(!mixed && chosen == choice.key, choice.title)).clicked() {
                                    chosen = choice.key.into();
                                }
                            }
                        });
                });
                if chosen != selected { self.relation_edits("Change relationship type", |source, line| laml_ui::notation_edit::relationship_type(source, line, &chosen)); }
            }
            self.relation_direction_control(ui, &operators, &lines);
            inspector_ui::category(ui, "Layout");
            ui.add_enabled_ui(relations.iter().all(|r| r.label.is_some()), |ui| {
                let mut chosen = mode;
                inspector_ui::row(ui, "Label place", |ui| {
                    egui::ComboBox::from_id_salt("relation-label-mode")
                        .width(ui.available_width())
                        .selected_text(inspector_ui::value_text(chosen))
                        .show_ui(ui, |ui| {
                            for value in ["default", "near", "along", "inline"] {
                                ui.selectable_value(&mut chosen, value, value);
                            }
                        });
                });
                if chosen != mode {
                    self.change_label_modes((chosen != "default").then_some(chosen));
                }
            });
            for (end, original) in end_sides {
                let title = if end == EndPosition::Source {
                    "Start side"
                } else {
                    "End side"
                };
                let mut side = original;
                inspector_ui::row(ui, title, |ui| {
                    egui::ComboBox::from_id_salt(title)
                        .width(ui.available_width())
                        .selected_text(inspector_ui::value_text(side))
                        .show_ui(ui, |ui| {
                            for value in ["auto", "left", "right", "top", "bottom"] {
                                ui.selectable_value(&mut side, value, value);
                            }
                        });
                });
                if side != original {
                    self.relation_sides(end, (side != "auto").then_some(side));
                }
            }
        });
    }

    pub(super) fn change_label_modes(&mut self, mode: Option<&str>) {
        if !self.can_edit_visually() {
            return;
        }
        match relation_edit::label_modes(self.document.source(), &self.selection.relations, mode) {
            Ok(edits) => self.submit_command(Command {
                edits,
                label: "Change relation label placement".into(),
                expectation: worker::Expectation::RelationEdit(self.selection.relations.clone()),
            }),
            Err(error) => self.command_error = Some(error),
        }
    }
    fn relation_sides(&mut self, end: EndPosition, side: Option<&str>) {
        if !self.can_edit_visually() {
            return;
        }
        match relation_edit::endpoint_sides(
            self.document.source(),
            &self.selection.relations,
            end,
            side,
        ) {
            Ok(edits) => self.submit_command(Command {
                edits,
                label: "Change endpoint sides".into(),
                expectation: worker::Expectation::RelationEdit(self.selection.relations.clone()),
            }),
            Err(error) => self.command_error = Some(error),
        }
    }
    pub(super) fn relation_edits(
        &mut self,
        title: &str,
        build: impl Fn(&str, usize) -> Result<Vec<laml_ui::document::Edit>, String>,
    ) {
        if !self.can_edit_visually() {
            return;
        }
        let mut edits = Vec::new();
        for line in &self.selection.relations {
            match build(self.document.source(), *line) {
                Ok(next) => edits.extend(next),
                Err(error) => {
                    self.command_error = Some(error);
                    return;
                }
            }
        }
        self.submit_command(Command {
            edits,
            label: title.into(),
            expectation: worker::Expectation::RelationEdit(self.selection.relations.clone()),
        });
    }
    pub(super) fn relation_property(&mut self, name: &str, value: Option<&str>) {
        if !self.can_edit_visually() {
            return;
        }
        let value = match value
            .map(|value| laml::styles::property_value(name, value))
            .transpose()
        {
            Ok(value) => value.map(|v| laml_ui::properties::string(&v)),
            Err(error) => {
                self.command_error = Some(error);
                return;
            }
        };
        let result = relation_edit::properties(
            self.document.source(),
            &self.selection.relations,
            name,
            value.as_deref(),
        );
        match result {
            Ok(edits) => self.submit_command(Command {
                edits,
                label: "Change relation style".into(),
                expectation: worker::Expectation::RelationStyle {
                    lines: self.selection.relations.clone(),
                    property: name.into(),
                    value,
                },
            }),
            Err(error) => self.command_error = Some(error),
        }
    }
}
