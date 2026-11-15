//! Semantic properties instead of an editable raw declaration heading.
use super::*;
use laml::syntax::NodeKind;
use laml_ui::declaration_fields::{self, Change, KINDS};
impl Editor {
    pub(super) fn declaration_fields(&mut self, ui: &mut egui::Ui) {
        if self.selection.ids.len() != 1
            || !self.selection.ports.is_empty()
            || !self.selection.relations.is_empty()
        {
            return;
        }
        let Some(node) = self
            .preview
            .as_ref()
            .and_then(|p| {
                p.diagram
                    .nodes
                    .iter()
                    .find(|n| n.id == self.selection.ids[0])
            })
            .cloned()
        else {
            return;
        };
        ui.add_enabled_ui(self.can_edit_visually(),|ui| {
            inspector_ui::row(ui,"Type",|ui| {
                if KINDS.contains(&node.kind) {
                    let mut kind=node.kind;
                    egui::ComboBox::from_id_salt("node-kind-property").width(ui.available_width()).selected_text(declaration_fields::title(kind)).show_ui(ui,|ui| {
                        for option in KINDS {
                            let valid=declaration_fields::change(self.document.source(),&node.id,Change::Kind(option)).is_ok();
                            if ui.add_enabled(valid,egui::Button::selectable(kind==option,declaration_fields::title(option))).clicked(){kind=option;}
                        }
                    });
                    if kind!=node.kind {self.change_declaration(&node.id,Change::Kind(kind));}
                }else {ui.label(declaration_fields::title(node.kind));}
            });
            if !matches!(node.kind,NodeKind::Object|NodeKind::Note|NodeKind::Junction) {
                inspector_ui::row(ui,"Visibility",|ui| {
                    let mut visibility=node.visibility;
                    let title=visibility_title(visibility);
                    egui::ComboBox::from_id_salt("access-visibility").width(ui.available_width()).selected_text(title).show_ui(ui,|ui| {
                        for value in [None,Some('+'),Some('-'),Some('#'),Some('~')] {ui.selectable_value(&mut visibility,value,visibility_title(value));}
                    }).response.on_hover_text("Access visibility, not whether this node is drawn. Use Style → Display to hide an element.");
                    if visibility!=node.visibility {self.change_declaration(&node.id,Change::Visibility(visibility));}
                });
            }
            if KINDS.contains(&node.kind) && (node.kind!=NodeKind::Generic || node.abstract_node) {
                inspector_ui::row(ui,"Abstract",|ui| {
                    let mut value=node.abstract_node;
                    if ui.checkbox(&mut value,"Enabled").changed(){self.change_declaration(&node.id,Change::Abstract(value));}
                });
            }
            if node.kind==NodeKind::Class {
                inspector_ui::row(ui,"Templates",|ui| {
                    let before=node.template_signature.as_deref().unwrap_or("");
                    let draft=self.property_drafts.entry("template-parameters".into()).or_insert_with(||before.into());
                    let field=inspector_ui::field(ui,"template-parameters",draft,false);
                    self.source_ui.extra_drafts.push(field.response.id);
                    let value=draft.clone();
                    if field.commit && value!=before {self.change_declaration(&node.id,Change::Templates(value));}
                });
            }
        });
    }
    pub(super) fn change_declaration(&mut self, id: &str, change: Change) {
        if !self.can_edit_visually() {
            return;
        }
        self.submit_structure(
            "Change element property",
            Some(id.into()),
            declaration_fields::change(self.document.source(), id, change),
        );
    }
}
fn visibility_title(value: Option<char>) -> &'static str {
    match value {
        Some('+') => "Public (+)",
        Some('-') => "Private (-)",
        Some('#') => "Protected (#)",
        Some('~') => "Package (~)",
        _ => "Default",
    }
}
