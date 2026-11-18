use super::*;
use laml_ui::properties::{self, Property, Values};

impl Editor {
    pub(super) fn property_rows(
        &mut self,
        ui: &mut egui::Ui,
        ids: &[String],
        values: &Values,
        port: bool,
    ) {
        let groups: &[(&str, &[&str])] = if port {
            &[("Appearance", properties::PORT_PROPERTIES)]
        } else {
            &[
                (
                    "Appearance",
                    &[
                        "fill",
                        "border-color",
                        "border-width",
                        "color",
                        "base-color",
                        "corner-radius",
                    ],
                ),
                (
                    "Size & shape",
                    &[
                        "shape",
                        "width",
                        "height",
                        "min-width",
                        "min-height",
                        "padding",
                    ],
                ),
                (
                    "Typography & icons",
                    &["font-size", "icon", "symbol-rendering"],
                ),
                ("Visibility", &["display"]),
            ]
        };
        for (title, names) in groups {
            inspector_ui::category(ui, title);
            for name in *names {
                self.property_row(ui, ids, values, port, name);
            }
        }
    }
    fn property_row(
        &mut self,
        ui: &mut egui::Ui,
        ids: &[String],
        values: &Values,
        port: bool,
        name: &str,
    ) {
        let selected: Vec<&Property> = ids
            .iter()
            .filter_map(|id| values.get(id).and_then(|v| v.get(name)))
            .collect();
        if selected.len() != ids.len() || selected.is_empty() {
            return;
        }
        let first = selected[0].value.clone();
        let mixed = selected.iter().any(|p| p.value != first);
        let enabled = selected.iter().all(|p| p.editable) && self.can_edit_visually();
        let reason = selected.iter().find_map(|p| p.reason.as_deref());
        if name == "icon" {
            self.icon_property_row(ui, ids, first.as_deref(), mixed, enabled, reason);
            return;
        }
        if name == "symbol-rendering"
            && !self.preview.as_ref().is_some_and(|p| {
                p.diagram
                    .nodes
                    .iter()
                    .any(|n| ids.contains(&n.id) && n.visibility.is_some())
            })
        {
            return;
        }
        ui.push_id((port, name), |ui| {
            let mut commit = false;
            let mut reset = false;
            let draft = self
                .property_drafts
                .entry(if port {
                    format!("port-{name}")
                } else {
                    name.into()
                })
                .or_insert_with(|| {
                    if mixed {
                        String::new()
                    } else {
                        first.clone().unwrap_or_default()
                    }
                });
            let before = draft.clone();
            ui.add_enabled_ui(enabled, |ui| {
                let title = if name == "symbol-rendering" {
                    "Header mark"
                } else {
                    property_title(name)
                };
                inspector_ui::row(ui, title, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let width = (ui.available_width() - 30.0).max(48.0);
                        let choices: &[&str] = match name {
                            "shape" => &["rectangle", "ellipse", "diamond", "note"],
                            "display" => &["block", "none"],
                            "symbol-rendering" => &["text", "icons"],
                            _ => &[],
                        };
                        if !choices.is_empty() {
                            egui::ComboBox::from_id_salt("value")
                                .width(width)
                                .selected_text(if mixed && draft.is_empty() {
                                    inspector_ui::mixed_text()
                                } else {
                                    egui::RichText::new(crate::i18n::tr(choice_title(name, draft)))
                                })
                                .show_ui(ui, |ui| {
                                    for choice in choices {
                                        let valid = name != "shape"
                                            || !matches!(*choice, "ellipse" | "diamond")
                                            || !self.preview.as_ref().is_some_and(|p| {
                                                p.diagram.nodes.iter().any(|n| {
                                                    ids.contains(&n.id) && !n.parts.is_empty()
                                                })
                                            });
                                        if ui
                                            .add_enabled(
                                                valid,
                                                egui::Button::selectable(
                                                    draft.as_str() == *choice,
                                                    crate::i18n::tr(choice_title(name, choice)),
                                                ),
                                            )
                                            .on_hover_ui(|ui| {
                                                hover_help::show(
                                                    ui,
                                                    crate::i18n::tr(choice_help(name, choice)),
                                                )
                                            })
                                            .clicked()
                                        {
                                            *draft = (*choice).into();
                                            commit = true;
                                        }
                                    }
                                });
                        } else {
                            let mut width = width;
                            let rgb =
                                if matches!(name, "fill" | "border-color" | "color" | "base-color")
                                {
                                    inspector::parse_rgb(draft)
                                } else {
                                    None
                                };
                            if rgb.is_some() {
                                width -= appearance::FIELD_HEIGHT + 6.0;
                            }
                            ui.scope_builder(
                                egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                                    ui.cursor().min,
                                    egui::vec2(width.max(32.0), appearance::FIELD_HEIGHT),
                                )),
                                |ui| {
                                    let field =
                                        inspector_ui::mixed_field(ui, "value", draft, false, mixed);
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
                        }
                        reset = appearance::property_action(ui, true).clicked();
                    });
                });
            });
            let value = draft.clone();
            if reset
                || (commit
                    && (mixed || first.as_deref() != Some(value.as_str()) || before != value))
            {
                let value = (!reset).then_some(value.as_str());
                if port {
                    self.request_port_style(name, value);
                } else {
                    self.request_style(name, value.map(str::to_owned));
                }
            }
            if let Some(reason) = reason {
                inspector_ui::content(ui, |ui| {
                    ui.small(crate::i18n::message(reason));
                });
            }
        });
    }
}
pub(super) fn property_title(name: &str) -> &str {
    match name {
        "fill" => "Fill",
        "border-color" => "Border",
        "border-width" => "Stroke",
        "color" => "Text",
        "base-color" => "Base color",
        "corner-radius" => "Corners",
        "shape" => "Shape",
        "width" => "Width",
        "height" => "Height",
        "min-width" => "Min. width",
        "min-height" => "Min. height",
        "padding" => "Padding",
        "font-size" => "Font size",
        "icon" => "Icon",
        "symbol-rendering" => "Access marks",
        "icon-set" => "Icon set",
        "display" => "Display",
        _ => name,
    }
}

fn access_title(value: &str) -> &str {
    match value {
        "text" => "Text (+ - # ~)",
        "icons" => "Icons",
        _ => value,
    }
}

fn choice_title<'a>(name: &str, value: &'a str) -> &'a str {
    if name == "symbol-rendering" {
        access_title(value)
    } else {
        crate::i18n::tr(value)
    }
}
fn choice_help<'a>(name: &str, value: &'a str) -> &'a str {
    if name == "symbol-rendering" {
        "Header visibility: + public (globe), - private (lock), # protected (shield), ~ package. Member marks are configured separately below. The icon set is automatic."
    } else {
        value
    }
}
