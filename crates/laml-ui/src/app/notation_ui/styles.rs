//! Specialized style controls expose only the compiler's existing notation rules.
use super::*;
impl Editor {
    pub(super) fn notation_style_fields(&mut self, ui: &mut egui::Ui, line: usize, id: Option<&str>) {
        let Some(id) = id.filter(|id| !id.starts_with("line:") && !id.starts_with("flow:")) else {
            inspector_ui::content(ui, |ui| { ui.label("Select a named element to edit its local style."); }); return;
        };
        let kind = self.preview.as_ref().and_then(|p| p.regions.iter().find(|r| r.id == id && r.line == line)).map(|r| r.kind.clone()).unwrap_or_else(|| "node".into());
        let relation = self.preview.as_ref().is_some_and(|p| p.diagram.relations.iter().any(|r| r.line == line));
        let (mut selector, properties) = match (self.notation(), kind.as_str()) {
            (Notation::Bpmn, "node") => (format!("node[{id}]"), vec!["base-color", "fill", "border-color", "color", "icon"]),
            (Notation::Timing, "lifeline") => (format!("node[{id}]"), vec!["border-color", "border-width", "color", "font-size"]),
            (Notation::Sequence, "participant") => (format!("node[{id}]"), vec!["base-color", "fill", "border-color", "color", "font-size", "width"]),
            (Notation::Sequence, "message") => (format!("relation[{id}]"), vec!["base-color", "color", "width", "font-size"]),
            (Notation::Sequence, "note") => (format!("note[{id}]"), vec!["base-color", "fill", "border-color", "color", "font-size"]),
            (Notation::Sequence, "continuation") => (format!("continuation[{id}]"), vec!["base-color", "fill", "border-color", "color", "font-size"]),
            (Notation::Sequence, "frame") => (format!("fragment[{id}]"), vec!["base-color", "fill", "border-color", "color", "font-size"]),
            (ty, _) if ty.c4() && relation => (format!("relation[{id}]"), vec!["base-color", "color", "width", "font-size"]),
            (ty, _) if ty.c4() => (format!("node[{id}]"), vec!["base-color", "fill", "border-color", "color", "font-size"]),
            _ => { inspector_ui::content(ui, |ui| { ui.label("This construct has no independent local style selector."); }); return; },
        };
        if self.notation() == Notation::Sequence && id.contains('.') {
            let mut path: Vec<_> = id.split('.').collect();
            let leaf = path.pop().unwrap();
            let prefix = path.into_iter().map(|id| format!("node[{id}] ")).collect::<String>();
            selector = format!("{prefix}{}[{leaf}]", selector.split('[').next().unwrap());
        }
        inspector_ui::category(ui, "Local style");
        for property in properties {
            if property == "icon" && self.notation() == Notation::Bpmn && laml::bpmn::parse(self.document.source()).is_ok_and(|d| d.nodes.iter().find(|n| n.id == id).is_some_and(|n| !matches!(n.kind,
                laml::bpmn::Kind::Task | laml::bpmn::Kind::UserTask | laml::bpmn::Kind::ServiceTask | laml::bpmn::Kind::SendTask | laml::bpmn::Kind::ReceiveTask | laml::bpmn::Kind::ScriptTask | laml::bpmn::Kind::ManualTask | laml::bpmn::Kind::Collapsed | laml::bpmn::Kind::Subprocess))) { continue; }
            let current = edit::records(self.document.source()).into_iter().find(|r| r.heading == selector && r.opens)
                .and_then(|rule| edit::records(self.document.source()).into_iter().find(|r| r.parent == Some(rule.line) && r.heading.starts_with(&format!("{property}:"))))
                .and_then(|r| r.heading.split_once(':').map(|(_, value)| value.trim().to_owned())).unwrap_or_default();
            let key = format!("notation-style-{id}-{property}");
            let mut draft = self.property_drafts.get(&key).cloned().unwrap_or_else(|| current.clone());
            let mut commit = false; let mut reset = false;
            inspector_ui::row(ui, property, |ui| {
                ui.horizontal(|ui| {
                    let field = ui.add(egui::TextEdit::singleline(&mut draft).hint_text("Automatic / inherited").desired_width((ui.available_width() - 30.0).max(30.0)));
                    self.source_ui.extra_drafts.push(field.id);
                    commit = field.lost_focus() || (field.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                    reset = appearance::property_action(ui, !current.is_empty()).clicked();
                });
            });
            self.property_drafts.insert(key, draft.clone());
            if reset || (commit && draft != current) {
                let value = if reset || draft.trim().is_empty() { None } else { Some(draft.as_str()) };
                match edit::style(self.document.source(), &selector, property, value) {
                    Ok(edits) => self.notation_submit("Change local style", edits, Some(line)),
                    Err(e) => self.command_error = Some(e),
                }
            }
        }
    }
    pub(super) fn notation_optional_fields(&mut self, ui: &mut egui::Ui, line: usize, id: Option<&str>) {
        let record = edit::record(self.document.source(), line).ok();
        let heading = record.as_ref().map_or("", |r| r.heading.as_str());
        let region_kind = self.preview.as_ref().and_then(|p| p.regions.iter().find(|r| r.line == line && id.is_none_or(|id| r.id == id))).map(|r| r.kind.as_str());
        let options: &[&str] = match self.notation() {
            Notation::C4Code if !heading.starts_with("component ") => &["label", "style-classes"],
            ty if ty.c4() && heading.starts_with('@') => &["technology"],
            ty if ty.c4() && heading.starts_with("container ") => &["label", "description", "technology", "appearance", "style-classes"],
            ty if ty.c4() && heading.starts_with("component ") => &["label", "description", "technology", "style-classes"],
            ty if ty.c4() => &["label", "description", "style-classes"],
            Notation::Sequence if region_kind == Some("participant") => &["label", "selector", "type", "style-classes"],
            Notation::Sequence if region_kind == Some("message") && id.is_some_and(|id| !id.starts_with("line:")) => &["style-classes"],
            _ => &[],
        };
        if options.is_empty() { return; }
        let tokens = edit::fields::tokens(heading);
        let missing: Vec<_> = options.iter().filter(|name| !tokens.windows(2).any(|t| &heading[t[0].clone()] == **name && &heading[t[1].clone()] == "=")).copied().collect();
        if missing.is_empty() { return; }
        inspector_ui::content(ui, |ui| {
            egui::CollapsingHeader::new("Optional attributes").show(ui, |ui| {
                for name in missing {
                    let key = format!("notation-optional-{line}-{name}");
                    let mut value = self.property_drafts.get(&key).cloned().unwrap_or_default();
                    ui.label(name);
                    let field = inspector_ui::field(ui, &key, &mut value, false);
                    self.source_ui.extra_drafts.push(field.response.id);
                    self.property_drafts.insert(key, value.clone());
                    if field.commit && !value.trim().is_empty() {
                        match edit::attribute(self.document.source(), line, id, name, &value) {
                            Ok(edits) => self.notation_submit("Set optional attribute", edits, Some(line)),
                            Err(e) => self.command_error = Some(e),
                        }
                    }
                }
            });
        });
    }
}
