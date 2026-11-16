//! Reference fields can be filled from compiler identities without typing addresses.
use super::*;
impl Editor {
    pub(super) fn construct_value(
        &self,
        ui: &mut egui::Ui,
        key: &str,
        value: &mut String,
        parent: Option<usize>,
    ) -> egui::Id {
        let references = [
            "from",
            "to",
            "participant",
            "classifier",
            "third",
            "relation",
            "frame",
            "note",
            "continuation",
            "occurrence",
            "first",
            "second",
        ];
        let mut choices = Vec::new();
        if let Some(preview) = &self.preview {
            if key == "selector" {
                choices.push(("node".into(), crate::i18n::tr("All nodes").into()));
                choices.push((
                    "relation".into(),
                    crate::i18n::tr("All relationships").into(),
                ));
                let ids: Vec<_> = if preview.notation.graph() {
                    preview.diagram.nodes.iter().map(|n| n.id.clone()).collect()
                } else {
                    preview
                        .regions
                        .iter()
                        .filter(|r| matches!(r.kind.as_str(), "participant" | "node" | "lifeline"))
                        .map(|r| r.id.clone())
                        .collect()
                };
                for id in ids {
                    let selector = if preview.notation == Notation::Sequence {
                        id.split('.')
                            .map(|part| format!("node[{part}]"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    } else {
                        format!("node[{id}]")
                    };
                    choices.push((selector.clone(), id.clone()));
                    if preview.notation == Notation::Sequence {
                        for part in ["lifeline", "activation", "coregion"] {
                            choices.push((format!("{selector}.{part}"), format!("{id} · {part}")));
                        }
                    }
                }
                if preview.notation == Notation::Sequence {
                    for part in [
                        "note",
                        "fragment",
                        "continuation",
                        "time-observation",
                        "time-constraint",
                        "duration-observation",
                        "duration-constraint",
                        "invariant",
                        "state",
                        "action",
                    ] {
                        choices.push((part.into(), format!("All {part}")));
                    }
                }
            } else if references.contains(&key) {
                if preview.notation.graph() {
                    for node in &preview.diagram.nodes {
                        choices.push((
                            node.id.clone(),
                            node.label
                                .as_deref()
                                .and_then(|s| s.lines().next())
                                .map_or_else(
                                    || node.id.clone(),
                                    |label| format!("{label} · {}", node.id),
                                ),
                        ));
                        for port in &node.ports {
                            let id = format!("{}::{}", node.id, port.id);
                            choices.push((id.clone(), id));
                        }
                    }
                    for r in &preview.diagram.relations {
                        if let Some(name) = &r.name {
                            choices.push((name.clone(), name.clone()));
                        }
                    }
                } else {
                    let scope = parent
                        .and_then(|line| edit::record(self.document.source(), line).ok())
                        .map(|r| {
                            if r.heading.starts_with("decompose ") {
                                let parent =
                                    edit::references_scope_for_creation(self.document.source(), &r);
                                let local = r.heading.split_whitespace().nth(1).unwrap_or("");
                                if parent.is_empty() {
                                    local.into()
                                } else {
                                    format!("{parent}.{local}")
                                }
                            } else {
                                edit::references_scope_for_creation(self.document.source(), &r)
                            }
                        })
                        .unwrap_or_default();
                    for region in &preview.regions {
                        if region.id.starts_with("line:") || region.id.contains(".line:") {
                            continue;
                        }
                        let id = if preview.notation == Notation::Sequence && !scope.is_empty() {
                            let Some(id) = region.id.strip_prefix(&format!("{scope}.")) else {
                                continue;
                            };
                            id.to_owned()
                        } else {
                            region.id.clone()
                        };
                        if matches!(key, "occurrence" | "first" | "second") {
                            if region.kind == "message" {
                                for end in ["send", "receive"] {
                                    let id = format!("{id}.{end}");
                                    choices.push((id.clone(), id));
                                }
                            }
                        } else if matches!(
                            region.kind.as_str(),
                            "participant" | "node" | "lifeline" | "frame" | "note" | "continuation"
                        ) {
                            choices.push((id.clone(), id));
                        }
                    }
                }
            }
        }
        choices.sort();
        choices.dedup();
        let mut field_id = egui::Id::NULL;
        ui.horizontal(|ui| {
            let reserve = if choices.is_empty() { 0.0 } else { 38.0 };
            field_id = ui
                .add(
                    egui::TextEdit::singleline(value)
                        .desired_width((ui.available_width() - reserve).max(40.0)),
                )
                .id;
            if !choices.is_empty() {
                egui::ComboBox::from_id_salt(("reference-options", key))
                    .width(20.0)
                    .selected_text("…")
                    .show_ui(ui, |ui| {
                        for (id, title) in choices {
                            ui.selectable_value(value, id, title);
                        }
                    });
            }
        });
        field_id
    }
}
