//! Local, searchable Phosphor catalogue. Only confirmation changes source.
use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(super) enum Category {
    #[default]
    All,
    Architecture,
    People,
    Files,
    Security,
    Arrows,
}
impl Category {
    const ALL: [(Self, &'static str); 6] = [
        (Self::All, "All"),
        (Self::Architecture, "Architecture"),
        (Self::People, "People"),
        (Self::Files, "Files"),
        (Self::Security, "Security"),
        (Self::Arrows, "Arrows"),
    ];
    fn matches(self, name: &str) -> bool {
        let words: &[&str] = match self {
            Self::All => return true,
            Self::Architecture => &[
                "cloud",
                "database",
                "server",
                "network",
                "cube",
                "stack",
                "cpu",
                "browser",
                "desktop",
                "device",
                "hard-drive",
                "circuit",
                "tree-structure",
                "globe",
            ],
            Self::People => &["user", "person", "users", "identification"],
            Self::Files => &["file", "folder", "document", "notebook"],
            Self::Security => &["lock", "shield", "key", "fingerprint", "password"],
            Self::Arrows => &["arrow"],
        };
        words.iter().any(|word| name.contains(word))
    }
}
pub(super) struct Picker {
    pub revision: u64,
    pub ids: Vec<String>,
    pub selected: Option<String>,
    pub query: String,
    pub category: Category,
    focus_search: bool,
    error: Option<String>,
}
impl Picker {
    pub fn filtered(&self) -> Vec<&'static str> {
        let query = self.query.to_lowercase().replace('-', " ");
        laml::bundled_icon_names()
            .filter(|name| {
                self.category.matches(name)
                    && query.split_whitespace().all(|word| name.contains(word))
            })
            .collect()
    }
}

/// Use the exact compiler SVG rather than a similarly named font glyph.
pub(super) fn image(ui: &egui::Ui, name: &str) -> egui::Image<'static> {
    let svg = laml::bundled_icon_svg(name).unwrap_or_default().replace(
        "currentColor",
        if ui.visuals().dark_mode {
            "#e8eef2"
        } else {
            "#1e2430"
        },
    );
    egui::Image::from_bytes(
        format!(
            "bytes://phosphor-catalog-{}-{name}.svg",
            ui.visuals().dark_mode
        ),
        svg.into_bytes(),
    )
    .fit_to_exact_size(egui::vec2(32.0, 32.0))
}

impl Editor {
    pub(super) fn icon_property_row(
        &mut self,
        ui: &mut egui::Ui,
        ids: &[String],
        current: Option<&str>,
        mixed: bool,
        enabled: bool,
        reason: Option<&str>,
    ) {
        let mut open = false;
        let mut reset = false;
        ui.push_id("icon-property", |ui| {
            ui.add_enabled_ui(enabled, |ui| {
                inspector_ui::row(ui, "Icon", |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let title = if mixed {
                            "Mixed · choose…".into()
                        } else {
                            current
                                .map_or_else(|| "Choose icon…".into(), |name| format!("{name} …"))
                        };
                        let button = if let Some(name) = current.filter(|_| !mixed) {
                            egui::Button::image_and_text(image(ui, name), title)
                        } else {
                            egui::Button::new(if mixed {
                                inspector_ui::mixed_text()
                            } else {
                                egui::RichText::new(title)
                            })
                        };
                        open = ui
                            .add_sized(
                                [
                                    (ui.available_width() - 30.0).max(48.0),
                                    appearance::FIELD_HEIGHT,
                                ],
                                button,
                            )
                            .on_hover_ui(|ui| {
                                hover_help::show(
                                    ui,
                                    crate::i18n::tr(
                                        "Choose a node icon from the searchable Phosphor catalogue",
                                    ),
                                )
                            })
                            .clicked();
                        reset = appearance::property_action(ui, true)
                            .on_hover_ui(|ui| {
                                hover_help::show(
                                    ui,
                                    crate::i18n::tr(
                                        "Reset local icon override (inherited styles remain)",
                                    ),
                                )
                            })
                            .clicked();
                    })
                });
            })
        });
        if let Some(reason) = reason {
            inspector_ui::content(ui, |ui| {
                ui.small(crate::i18n::message(reason));
            });
        }
        if open {
            self.open_icon_picker(
                ids.to_vec(),
                if mixed {
                    None
                } else {
                    current.map(str::to_owned)
                },
            );
        }
        if reset {
            self.request_style("icon", None);
        }
    }
    pub(super) fn open_icon_picker(&mut self, ids: Vec<String>, selected: Option<String>) {
        if !self.can_edit_visually() || ids.is_empty() {
            return;
        }
        self.graph.icon_picker = Some(Picker {
            revision: self.document.revision(),
            ids,
            selected,
            query: String::new(),
            category: Category::All,
            focus_search: true,
            error: None,
        });
    }
    pub(super) fn submit_icon_picker(&mut self, picker: &Picker) -> Result<(), String> {
        if picker.revision != self.document.revision()
            || picker.ids != self.selection.ids
            || !self.can_edit_visually()
        {
            return Err(
                "Document or selection changed. Reopen the icon chooser; source unchanged.".into(),
            );
        }
        let icon = picker
            .selected
            .as_deref()
            .filter(|name| laml::bundled_icon_svg(name).is_some())
            .ok_or("Choose an icon first.")?;
        self.request_style("icon", Some(icon.into()));
        if let Some(error) = self.command_error.take() {
            Err(error)
        } else {
            Ok(())
        }
    }
    pub(super) fn icon_picker_window(&mut self, ctx: &egui::Context) {
        let Some(mut picker) = self.graph.icon_picker.take() else {
            return;
        };
        if let Some(error) = self.command_error.take() {
            picker.error = Some(error);
        }
        let mut submit = false;
        let mut cancel = false;
        let stale = picker.revision != self.document.revision() || picker.ids != self.selection.ids;
        let dismissed = dialogs::show(ctx, ("choose-icon", self.id), "Choose icon", 720.0, |ui| {
            ui.label(crate::i18n::format(
                "Phosphor · {0} icons · {1} selected node(s)",
                &[
                    ("0", format!("{}", laml::bundled_icon_names().count())),
                    ("1", format!("{}", picker.ids.len())),
                ],
            ));
            let search = ui.add(
                egui::TextEdit::singleline(&mut picker.query)
                    .hint_text(crate::i18n::tr("Search icons… cloud, database, user"))
                    .desired_width(ui.available_width()),
            );
            self.source_ui.extra_drafts.push(search.id);
            if picker.focus_search {
                search.request_focus();
                picker.focus_search = false;
            }
            ui.horizontal_wrapped(|ui| {
                for (category, title) in Category::ALL {
                    ui.selectable_value(&mut picker.category, category, title);
                }
            });
            let names = picker.filtered();
            ui.colored_label(
                appearance::color(ui.ctx(), appearance::MUTED),
                crate::i18n::format("{0} matching icons", &[("0", format!("{}", names.len()))]),
            );
            if names.is_empty() {
                ui.label(crate::i18n::tr(
                    "No matching icons. Try another name or choose All.",
                ));
                ui.add_space(80.0);
            }
            let columns = ((ui.available_width() + 8.0) / 96.0).floor().max(1.0) as usize;
            let tile_width = (ui.available_width() - 8.0 * (columns - 1) as f32) / columns as f32;
            let rows = names.len().div_ceil(columns);
            egui::ScrollArea::vertical()
                .id_salt(("icon-grid", &picker.query, picker.category))
                .max_height((ctx.content_rect().height() - 380.0).clamp(140.0, 440.0))
                .show_rows(ui, 90.0, rows, |ui, range| {
                    for row in range {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            for name in names.iter().skip(row * columns).take(columns) {
                                if tile(
                                    ui,
                                    name,
                                    tile_width,
                                    picker.selected.as_deref() == Some(*name),
                                )
                                .clicked()
                                {
                                    picker.selected = Some((*name).into());
                                }
                            }
                        });
                    }
                });
            ui.separator();
            if stale {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    crate::i18n::tr("Document changed. Close and reopen the icon chooser."),
                );
            }
            if let Some(error) = &picker.error {
                command_errors::details(ui, error);
            }
            ui.horizontal(|ui| {
                if let Some(name) = &picker.selected {
                    ui.add(image(ui, name));
                    ui.add(egui::Label::new(name).truncate())
                        .on_hover_text(name);
                } else {
                    ui.colored_label(
                        appearance::color(ui.ctx(), appearance::MUTED),
                        crate::i18n::tr("Select an icon to preview it here."),
                    );
                }
            });
            ui.horizontal(|ui| {
                cancel = ui.button(crate::i18n::tr("Cancel")).clicked();
                submit = ui
                    .add_enabled(
                        !stale && picker.selected.is_some() && self.can_edit_visually(),
                        egui::Button::new(crate::i18n::tr("Use icon")),
                    )
                    .clicked();
            });
        });
        if submit {
            match self.submit_icon_picker(&picker) {
                Ok(()) => return,
                Err(error) => picker.error = Some(error),
            }
        }
        if !dismissed && !cancel {
            self.graph.icon_picker = Some(picker);
        }
    }
}
fn tile(ui: &mut egui::Ui, name: &str, width: f32, selected: bool) -> egui::Response {
    let button = egui::Button::new("")
        .fill(if selected {
            appearance::color(ui.ctx(), appearance::ACCENT).gamma_multiply(0.25)
        } else {
            appearance::color(ui.ctx(), appearance::BACKGROUND)
        })
        .stroke(egui::Stroke::new(
            1.0,
            if selected {
                appearance::color(ui.ctx(), appearance::ACCENT)
            } else {
                appearance::color(ui.ctx(), appearance::BORDER)
            },
        ));
    let response = ui
        .push_id(name, |ui| ui.add_sized([width, 90.0], button))
        .inner
        .on_hover_text(name);
    if ui.is_rect_visible(response.rect) {
        image(ui, name).paint_at(
            ui,
            egui::Rect::from_center_size(
                response.rect.center_top() + egui::vec2(0.0, 30.0),
                egui::vec2(32.0, 32.0),
            ),
        );
        let label = egui::Rect::from_min_size(
            response.rect.min + egui::vec2(5.0, 60.0),
            egui::vec2(width - 10.0, 22.0),
        );
        let galley = egui::WidgetText::from(egui::RichText::new(name).size(12.0)).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            label.width(),
            egui::FontId::proportional(12.0),
        );
        let pos = label.center() - galley.size() * 0.5;
        ui.painter()
            .galley(pos, galley, appearance::color(ui.ctx(), appearance::TEXT));
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), name));
    response
}
