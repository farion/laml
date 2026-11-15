use super::*;
impl Editor {
    pub(in crate::app) fn member_style_controls(
        &mut self,
        ui: &mut egui::Ui,
        node: &laml::syntax::NodeDecl,
    ) {
        if node.parts.is_empty() {
            return;
        }
        ui.add_enabled_ui(self.can_edit_visually(), |ui| {
            for part in &node.parts {
                let laml::syntax::NodePart::Member { line, text, .. } = part else {
                    continue;
                };
                ui.scope(|ui| {
                    inspector_ui::category(ui, &format!("Member · {text}"));
                    let Some(address) = structure::member_address(&node.id, part) else {
                        if ui.button("Assign global member ID").clicked() {
                            self.submit_structure(
                                "Address member",
                                Some(node.id.clone()),
                                structure::assign_member_id(
                                    self.document.source(),
                                    &node.id,
                                    *line,
                                ),
                            );
                        }
                        return;
                    };
                    inspector_ui::row(ui, "Address", |ui| {
                        ui.small(&address);
                    });
                    if matches!(part, laml::syntax::NodePart::Member { id: None, .. })
                        && ui.button("Make ID independent of name").clicked()
                    {
                        self.submit_structure(
                            "Address member",
                            Some(node.id.clone()),
                            structure::assign_member_id(self.document.source(), &node.id, *line),
                        );
                    }
                    for name in structure::MEMBER_PROPERTIES.into_iter().filter(|name|*name!="icon-set" && (*name!="symbol-rendering" || text.starts_with(['+','-','#','~']))) {
                        let value = self
                            .preview
                            .as_ref()
                            .and_then(|p| structure::member_value(p, &node.id, &address, name));
                        let key = format!("member:{address}:{name}");
                        let draft = self
                            .property_drafts
                            .entry(key)
                            .or_insert_with(|| value.unwrap_or_default());
                        let (commit, reset) =
                            inspector_ui::row(ui, property_rows::property_title(name), |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    let width = (ui.available_width() - 30.0).max(32.0);
                                    let commit=if name=="symbol-rendering" {
                                        let before=draft.clone();
                                        egui::ComboBox::from_id_salt((&address,name)).width(width).selected_text(if draft=="icons"{"Icons"}else{"Text (+ - # ~)"}).show_ui(ui,|ui| {
                                            ui.selectable_value(draft,"text".into(),"Text (+ - # ~)");
                                            ui.selectable_value(draft,"icons".into(),"Icons").on_hover_text("Visibility marks use globe, lock, shield and package. Phosphor is selected automatically.");
                                        });
                                        *draft!=before
                                    }else{
                                        let field=ui.scope_builder(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(ui.cursor().min,egui::vec2(width,appearance::FIELD_HEIGHT))),|ui|inspector_ui::field(ui,(&address,name),draft,false)).inner;
                                        self.source_ui.extra_drafts.push(field.response.id);
                                        field.commit
                                    };
                                    (commit,appearance::property_action(ui,true).clicked())
                                })
                                .inner
                            });
                        let value = draft.clone();
                        if reset || commit {
                            let value = (!reset).then_some(value.as_str());
                            match structure::member_style(
                                self.document.source(),
                                &node.id,
                                &address,
                                name,
                                value,
                            ) {
                                Ok(edits) => self.submit_command(Command {
                                    edits,
                                    label: "Member style".into(),
                                    expectation: worker::Expectation::MemberStyle {
                                        owner: node.id.clone(),
                                        address: address.clone(),
                                        name: name.into(),
                                        value: value.map(str::to_string),
                                    },
                                }),
                                Err(e) => self.command_error = Some(e),
                            }
                        }
                    }
                });
            }
        });
        inspector_ui::category(ui, "Compartment visibility");
        ui.add_enabled_ui(self.can_edit_visually(), |ui| {
            for kind in ["attributes", "operations", "literals"] {
                let current = self
                    .preview
                    .as_ref()
                    .map(|p| structure::compartment_value(p, &node.id, kind))
                    .unwrap_or_default();
                let mut chosen = current.as_str();
                let mut reset = false;
                inspector_ui::row(ui, kind, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        egui::ComboBox::from_id_salt(kind)
                            .width((ui.available_width() - 30.0).max(32.0))
                            .selected_text(chosen)
                            .show_ui(ui, |ui| {
                                for value in ["block", "none"] {
                                    ui.selectable_value(&mut chosen, value, value);
                                }
                            });
                        reset = appearance::property_action(ui, true).clicked();
                    });
                });
                if reset || chosen != current {
                    let value = (!reset).then_some(chosen);
                    match structure::compartment(self.document.source(), &node.id, kind, value) {
                        Ok(edits) => self.submit_command(Command {
                            edits,
                            label: "Compartment visibility".into(),
                            expectation: worker::Expectation::CompartmentStyle {
                                owner: node.id.clone(),
                                kind: kind.into(),
                                value: value.map(str::to_string),
                            },
                        }),
                        Err(e) => self.command_error = Some(e),
                    }
                }
            }
        });
    }
}
