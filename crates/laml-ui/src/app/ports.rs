use super::*;
use laml_ui::{port_edit, properties};
impl Editor {
    pub(super) fn port_inspector(&mut self, ui: &mut egui::Ui) {
        if !self.selection.ids.is_empty() || !self.selection.relations.is_empty() {
            ui.label("Mixed element types selected. Properties require only nodes, only ports or only relations.");
            return;
        }
        let ids = self.selection.ports.clone();
        let Some(preview) = &self.preview else {
            return;
        };
        let ports: Vec<_> = ids
            .iter()
            .filter_map(|id| port_edit::find(&preview.diagram, id).map(|(_, p)| p.clone()))
            .collect();
        let Some(first) = ports.first() else {
            return;
        };
        let values = preview.port_properties.clone();
        let value = first.label.clone();
        let mixed = ports.iter().any(|p| p.label != value);
        let explicit = ports.iter().all(|p| p.explicit);
        let inherited = ids.iter().any(|id| {
            port_edit::find(&preview.diagram, id).is_some_and(|(n, _)| {
                matches!(
                    n.kind,
                    laml::syntax::NodeKind::ComponentPart
                        | laml::syntax::NodeKind::CompositePart
                        | laml::syntax::NodeKind::CompositeRole
                )
            })
        });
        let sides: Vec<_> = ids
            .iter()
            .map(|id| {
                preview
                    .diagram
                    .port_layout
                    .iter()
                    .find(|r| &r.id == id)
                    .map(|r| match r.side {
                        laml::syntax::PortSide::Left => "left",
                        laml::syntax::PortSide::Right => "right",
                        laml::syntax::PortSide::Top => "top",
                        laml::syntax::PortSide::Bottom => "bottom",
                    })
                    .unwrap_or("auto")
            })
            .collect();
        let side = if sides.iter().all(|s| s == &sides[0]) {
            sides[0]
        } else {
            "Mixed"
        };
        let rendered_sides = ids
            .iter()
            .filter_map(|id| {
                preview
                    .nodes
                    .iter()
                    .flat_map(|b| &b.ports)
                    .find(|p| &p.id == id)
                    .map(|p| format!("{id}: {:?}", p.side))
            })
            .collect::<Vec<_>>()
            .join(", ");
        if self.inspector_tab == inspector_ui::Tab::Style {
            self.property_rows(ui, &ids, &values, true);
            return;
        }
        inspector_ui::category(ui, "Identity");
        let enabled = self.can_edit_visually();
        ui.add_enabled_ui(enabled, |ui| {
            ui.add_enabled_ui(explicit && !inherited, |ui| {
                inspector_ui::row(ui, "Name", |ui| {
                    let draft = self
                        .property_drafts
                        .entry("port-name".into())
                        .or_insert_with(|| if mixed { String::new() } else { value.clone() });
                    let field = inspector_ui::mixed_field(ui, "port-name", draft, false, mixed);
                    self.source_ui.extra_drafts.push(field.response.id);
                    let new = draft.clone();
                    if field.commit && (mixed || new != value) {
                        self.change_port_labels(&new);
                    }
                });
            });
            ui.add_enabled_ui(ids.len() == 1 && !inherited, |ui| {
                inspector_ui::row(ui, "ID", |ui| {
                    let original = if first.explicit {
                        first.id.clone()
                    } else {
                        String::new()
                    };
                    let draft = self
                        .property_drafts
                        .entry("port-id".into())
                        .or_insert_with(|| {
                            if ids.len() > 1 {
                                String::new()
                            } else {
                                original.clone()
                            }
                        });
                    if ids.len() > 1 {
                        draft.clear();
                    }
                    let field =
                        inspector_ui::mixed_field(ui, "port-id", draft, false, ids.len() > 1);
                    self.source_ui.extra_drafts.push(field.response.id);
                    let value = draft.clone();
                    if field.commit && value != original {
                        self.rename_port(&ids[0], &value);
                    }
                });
            });
            self.typed_port_attributes(ui);
            inspector_ui::category(ui, "Layout");

            let mut chosen = side;
            inspector_ui::row(ui, "Side", |ui| {
                egui::ComboBox::from_id_salt("port-side")
                    .width(ui.available_width())
                    .selected_text(if chosen == "Mixed" {
                        inspector_ui::mixed_text()
                    } else {
                        egui::RichText::new(chosen)
                    })
                    .show_ui(ui, |ui| {
                        for value in ["auto", "left", "right", "top", "bottom"] {
                            ui.selectable_value(&mut chosen, value, value);
                        }
                    });
            });
            if chosen != side {
                self.change_port_sides((chosen != "auto").then_some(chosen));
            }
            inspector_ui::row(ui, "Rendered", |ui| {
                ui.small(&rendered_sides);
            });
        });
    }
    pub(super) fn request_port_style(&mut self, name: &str, value: Option<&str>) {
        if !self.can_edit_visually()
            || !self.selection.ids.is_empty()
            || !self.selection.relations.is_empty()
        {
            return;
        }
        let value = match value
            .map(|value| laml::styles::property_value(name, value))
            .transpose()
        {
            Ok(value) => value.map(|v| properties::string(&v)),
            Err(error) => {
                self.command_error = Some(error);
                return;
            }
        };
        match port_edit::property(
            self.document.source(),
            &self.selection.ports,
            name,
            value.as_deref(),
        ) {
            Ok(edits) => self.submit_command(Command {
                edits,
                label: "Change port style".into(),
                expectation: worker::Expectation::PortStyle {
                    ids: self.selection.ports.clone(),
                    property: name.into(),
                    value,
                },
            }),
            Err(error) => self.command_error = Some(error),
        }
    }
    pub(super) fn change_port_sides(&mut self, side: Option<&str>) {
        if !self.can_edit_visually() {
            return;
        }
        match port_edit::side(self.document.source(), &self.selection.ports, side) {
            Ok(edits) => self.submit_command(Command {
                edits,
                label: "Change port sides".into(),
                expectation: worker::Expectation::Ports(self.selection.ports.clone()),
            }),
            Err(error) => self.command_error = Some(error),
        }
    }
    pub(super) fn change_port_labels(&mut self, value: &str) {
        if !self.can_edit_visually() {
            return;
        }
        let result = self
            .selection
            .ports
            .iter()
            .try_fold(Vec::new(), |mut edits, id| {
                edits.extend(port_edit::label(self.document.source(), id, value)?);
                Ok::<_, String>(edits)
            });
        match result {
            Ok(edits) => self.submit_command(Command {
                edits,
                label: "Change port names".into(),
                expectation: worker::Expectation::Ports(self.selection.ports.clone()),
            }),
            Err(error) => self.command_error = Some(error),
        }
    }
    pub(super) fn rename_port(&mut self, id: &str, value: &str) {
        if !self.can_edit_visually() {
            return;
        }
        match port_edit::rename_id(self.document.source(), id, value) {
            Ok(edits) => self.submit_command(Command {
                edits,
                label: "Rename port ID".into(),
                expectation: worker::Expectation::Port(value.into()),
            }),
            Err(error) => self.command_error = Some(error),
        }
    }
}
