//! BPMN's typed properties refer to a single element/flow, including inline syntax.
use super::*;
impl Editor {
    pub(super) fn bpmn_properties(&mut self, ui: &mut egui::Ui, id: &str, line: usize) -> bool {
        let Ok(doc) = laml::bpmn::parse(self.document.source()) else {
            return false;
        };
        if let Some(index) = id
            .strip_prefix("flow:")
            .and_then(|s| s.parse::<usize>().ok())
        {
            let Some(flow) = doc.edges.get(index) else {
                return false;
            };
            for (name, value) in [
                ("from", flow.from.as_str()),
                ("to", flow.to.as_str()),
                ("label", flow.label.as_deref().unwrap_or("")),
            ] {
                let key = format!("bpmn-flow-{index}-{name}");
                let mut draft = self
                    .property_drafts
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| value.into());
                let mut commit = false;
                inspector_ui::row(ui, name, |ui| {
                    let field = inspector_ui::field(ui, &key, &mut draft, false);
                    self.source_ui.extra_drafts.push(field.response.id);
                    commit = field.commit;
                });
                self.property_drafts.insert(key, draft.clone());
                if commit && draft != value {
                    let edits =
                        edit::bpmn::flow_property(self.document.source(), index, name, &draft);
                    self.submit_bpmn_property(edits, line);
                }
            }
            let old = match flow.kind {
                laml::bpmn::EdgeKind::Sequence => "-->",
                laml::bpmn::EdgeKind::Message => "~~>",
                laml::bpmn::EdgeKind::Data => "..>",
                laml::bpmn::EdgeKind::Association => "..",
            };
            let mut chosen = old;
            inspector_ui::row(ui, "Type", |ui| {
                egui::ComboBox::from_id_salt("bpmn-flow-kind")
                    .selected_text(crate::i18n::tr(flow_title(old)))
                    .show_ui(ui, |ui| {
                        for op in ["-->", "~~>", "..>", ".."] {
                            let candidate = edit::bpmn::flow_property(
                                self.document.source(),
                                index,
                                "type",
                                op,
                            );
                            let response = ui.add_enabled(
                                candidate.is_ok(),
                                egui::Button::selectable(
                                    chosen == op,
                                    crate::i18n::tr(flow_title(op)),
                                ),
                            );
                            if let Err(error) = candidate {
                                response
                                    .clone()
                                    .on_disabled_hover_text(crate::i18n::message(&(error)));
                            }
                            if response.clicked() {
                                chosen = op;
                            }
                        }
                    });
            });
            if chosen != old {
                self.submit_bpmn_property(
                    edit::bpmn::flow_property(self.document.source(), index, "type", chosen),
                    line,
                );
            }
            return true;
        }
        let Some(node) = doc.nodes.iter().find(|n| n.id == id) else {
            return false;
        };
        if !matches!(
            node.kind,
            laml::bpmn::Kind::Boundary | laml::bpmn::Kind::Subprocess
        ) {
            let original = edit::bpmn::keyword(node);
            let mut selected = original;
            inspector_ui::row(ui, "Type", |ui| {
                egui::ComboBox::from_id_salt("bpmn-node-kind")
                    .selected_text(
                        edit::bpmn::TYPES
                            .iter()
                            .find(|(key, _)| *key == original)
                            .map_or(original, |(_, title)| crate::i18n::tr(title)),
                    )
                    .show_ui(ui, |ui| {
                        for &(key, title) in edit::bpmn::TYPES {
                            let candidate =
                                edit::bpmn::change_type(self.document.source(), id, key);
                            let response = ui.add_enabled(
                                candidate.is_ok(),
                                egui::Button::selectable(key == selected, crate::i18n::tr(title)),
                            );
                            if let Err(error) = candidate {
                                response
                                    .clone()
                                    .on_disabled_hover_text(crate::i18n::message(&(error)));
                            }
                            if response.clicked() {
                                selected = key;
                            }
                        }
                    });
            });
            if selected != original {
                self.submit_bpmn_property(
                    edit::bpmn::change_type(self.document.source(), id, selected),
                    line,
                );
            }
        }
        let mut properties = vec![("label", node.label.as_str())];
        if matches!(
            node.kind,
            laml::bpmn::Kind::Task
                | laml::bpmn::Kind::UserTask
                | laml::bpmn::Kind::ServiceTask
                | laml::bpmn::Kind::SendTask
                | laml::bpmn::Kind::ReceiveTask
                | laml::bpmn::Kind::ScriptTask
                | laml::bpmn::Kind::ManualTask
                | laml::bpmn::Kind::Collapsed
                | laml::bpmn::Kind::Subprocess
        ) {
            properties.push(("icon", node.icon.as_deref().unwrap_or("")));
        }
        if node.kind == laml::bpmn::Kind::Boundary {
            properties.push(("marker", node.marker.as_deref().unwrap_or("timer")));
            properties.push((
                "interrupting",
                if node.interrupting { "true" } else { "false" },
            ));
        }
        for (name, original) in properties {
            let key = format!("bpmn-node-{id}-{name}");
            let mut value = self
                .property_drafts
                .get(&key)
                .cloned()
                .unwrap_or_else(|| original.into());
            let mut commit = false;
            inspector_ui::row(ui, name, |ui| {
                let f = inspector_ui::field(ui, &key, &mut value, false);
                self.source_ui.extra_drafts.push(f.response.id);
                commit = f.commit;
            });
            self.property_drafts.insert(key, value.clone());
            if commit && value != original {
                self.submit_bpmn_property(
                    edit::bpmn::node_property(self.document.source(), id, name, &value),
                    line,
                );
            }
        }
        true
    }
    fn submit_bpmn_property(
        &mut self,
        result: Result<Vec<laml_ui::document::Edit>, String>,
        line: usize,
    ) {
        match result {
            Ok(edits) => self.notation_submit("Change BPMN property", edits, Some(line)),
            Err(e) => self.command_error = Some(e),
        }
    }
}
fn flow_title(op: &str) -> &str {
    match op {
        "-->" => "Sequence flow",
        "~~>" => "Message flow",
        "..>" => "Data association",
        ".." => "Annotation association",
        _ => op,
    }
}
