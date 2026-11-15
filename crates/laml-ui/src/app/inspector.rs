use super::*;

impl Editor {
    pub(super) fn inspector(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            egui::ScrollArea::vertical()
                .id_salt(("properties-scroll", self.inspector_tab as u8))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_max_width(ui.available_width());
                    self.inspector_contents(ui);
                });
        });
        if let Some(tab) = self.inspector_next_tab.take() {
            self.inspector_tab = tab;
        }
    }
    fn inspector_contents(&mut self, ui: &mut egui::Ui) {
        if self.selection.ids.is_empty()
            && self.selection.ports.is_empty()
            && self.selection.relations.is_empty()
        {
            self.document_settings(ui);
            return;
        }
        if !self.selection.ports.is_empty() {
            self.port_inspector(ui);
        } else if !self.selection.relations.is_empty() {
            self.relation_inspector(ui);
        } else if self.inspector_tab == inspector_ui::Tab::General {
            self.object_fields(ui);
            self.declaration_fields(ui);
            inspector_ui::category(ui, "Layout");
            self.membership_controls(ui);
            self.structured_controls(ui);
        } else if let Some(preview) = &self.preview {
            let values = preview.node_properties.clone();
            let ids = self.selection.ids.clone();
            self.property_rows(ui, &ids, &values, false);
            if self.selection.ids.len() == 1
                && let Some(node) = self
                    .preview
                    .as_ref()
                    .and_then(|p| {
                        p.diagram
                            .nodes
                            .iter()
                            .find(|n| n.id == self.selection.ids[0])
                    })
                    .cloned()
            {
                self.member_style_controls(ui, &node);
            }
        }
    }

    pub(super) fn request_style(&mut self, name: &str, value: Option<String>) {
        if !self.can_edit_visually()
            || !self.selection.ports.is_empty()
            || !self.selection.relations.is_empty()
        {
            return;
        }
        let value = match value
            .map(|v| {
                laml::styles::property_value(name, &laml_ui::properties::source_value(name, &v))
            })
            .transpose()
        {
            Ok(value) => value.map(|v| laml_ui::properties::string(&v)),
            Err(error) => {
                self.command_error = Some(error);
                return;
            }
        };
        if let Some(preview) = &self.preview
            && self.selection.ids.iter().any(|id| {
                preview
                    .node_properties
                    .get(id)
                    .and_then(|p| p.get(name))
                    .is_none_or(|p| !p.editable)
            })
        {
            self.command_error = Some("Property does not apply to every selected element.".into());
            return;
        }
        let mut properties = vec![(name.to_owned(), value.clone())];
        if name == "symbol-rendering" && value.as_deref() == Some("icons") {
            properties.push(("icon-set".into(), Some("phosphor".into())));
        }
        let source_properties: Vec<_> = properties
            .iter()
            .map(|(name, value)| {
                (
                    name.clone(),
                    value
                        .as_ref()
                        .map(|value| laml_ui::properties::source_value(name, value)),
                )
            })
            .collect();
        let edits = match laml_ui::style_edit::node_properties(
            self.document.source(),
            &self.selection.ids,
            &source_properties,
        ) {
            Ok(edits) => edits,
            Err(error) => {
                self.command_error = Some(error);
                return;
            }
        };
        self.submit_command(Command {
            edits,
            label: "Change node style".into(),
            expectation: worker::Expectation::Styles {
                ids: self.selection.ids.clone(),
                properties,
            },
        });
    }
}

pub(super) fn parse_rgb(value: &str) -> Option<[u8; 3]> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    Some([
        u8::from_str_radix(&hex[..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..], 16).ok()?,
    ])
}
