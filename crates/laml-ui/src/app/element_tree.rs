use super::*;
use laml_ui::properties::port_id;
impl Editor {
    /// Includes hidden/overlapped elements so display:none can always be undone locally.
    pub(super) fn element_tree(&mut self, ui: &mut egui::Ui) {
        let Some(preview) = &self.preview else {
            return;
        };
        egui::ScrollArea::vertical()
            .id_salt("element-tree")
            .max_height(220.0)
            .show(ui, |ui| {
                for node in &preview.diagram.nodes {
                    let hidden = !preview.nodes.iter().any(|b| b.id == node.id);
                    if ui
                        .selectable_label(
                            self.selection.ids.contains(&node.id),
                            format!(
                                "{}{}",
                                node.id,
                                if hidden {
                                    crate::i18n::tr(" (hidden)")
                                } else {
                                    ""
                                }
                            ),
                        )
                        .clicked()
                    {
                        self.selection
                            .click(Some(&node.id), ui.input(|i| i.modifiers.shift));
                        self.property_drafts.clear();
                        if self.rendered == Some(self.document.revision()) {
                            self.source_jump = Some(node.line);
                        }
                    }
                    for port in &node.ports {
                        let id = port_id(&node.id, port);
                        if ui
                            .selectable_label(
                                self.selection.ports.contains(&id),
                                format!("    ◦ {id} · {}", port.label),
                            )
                            .clicked()
                        {
                            self.selection
                                .click_port(&id, ui.input(|i| i.modifiers.shift));
                            self.property_drafts.clear();
                            if self.rendered == Some(self.document.revision()) {
                                self.source_jump = Some(port.line);
                            }
                        }
                    }
                }
                for a in &preview.diagram.attachments {
                    if ui
                        .selectable_label(
                            self.selection.relations.contains(&a.line),
                            format!("⋯ {} .. relation[{}]", a.source, a.target),
                        )
                        .clicked()
                    {
                        self.selection
                            .click_relation(a.line, ui.input(|i| i.modifiers.shift));
                        self.property_drafts.clear();
                        if self.rendered == Some(self.document.revision()) {
                            self.source_jump = Some(a.line);
                        }
                    }
                }
                for edge in &preview.diagram.relations {
                    if ui
                        .selectable_label(
                            self.selection.relations.contains(&edge.line),
                            crate::i18n::format(
                                "→ {0} · line {1}",
                                &[
                                    (
                                        "0",
                                        edge.name
                                            .as_deref()
                                            .unwrap_or(crate::i18n::tr("unnamed"))
                                            .to_string(),
                                    ),
                                    ("1", format!("{}", edge.line)),
                                ],
                            ),
                        )
                        .clicked()
                    {
                        self.selection
                            .click_relation(edge.line, ui.input(|i| i.modifiers.shift));
                        self.property_drafts.clear();
                        if self.rendered == Some(self.document.revision()) {
                            self.source_jump = Some(edge.line);
                        }
                    }
                }
            });
    }
}
