//! Whole action groups collapse; binary downloads are the first thing removed.
use super::*;
#[derive(Debug)]
pub(super) struct Layout {
    pub left_menu: bool,
    pub right_menu: bool,
    pub download: bool,
}
impl Layout {
    pub fn new(width: f32, left: f32, right: f32, title: f32, download: f32, mobile: bool) -> Self {
        let half = ((width - title) / 2.0 - 12.0).max(32.0);
        Self {
            left_menu: left > half,
            right_menu: right > half,
            download: !mobile && left <= half && right + download + 10.0 <= half,
        }
    }
}
impl Workspace {
    pub(super) fn titlebar(&mut self, ui: &mut egui::Ui, locked: bool) {
        let rect = ui.max_rect();
        let title = ui.painter().layout_no_wrap(
            crate::i18n::tr("LAML Editor").into(),
            egui::FontId::proportional(16.0),
            appearance::color(ui.ctx(), appearance::TEXT),
        );
        let binary_width = ui
            .painter()
            .layout_no_wrap(
                crate::i18n::tr("Download LAML for local use").into(),
                egui::TextStyle::Button.resolve(ui.style()),
                ui.visuals().text_color(),
            )
            .size()
            .x
            + 24.0;
        let left_width = self.left_title_width(ui);
        let right_width = group_width(ui, &[GEAR_SIX, QUESTION, egui_phosphor::regular::INFO]);
        let mobile = rect.width() < 600.0 || mobile_browser();
        let title_width = title.size().x;
        let layout = Layout::new(
            rect.width(),
            left_width,
            right_width,
            title.size().x,
            binary_width,
            mobile,
        );
        ui.painter().galley(
            rect.center() - title.size() / 2.0,
            title,
            appearance::color(ui.ctx(), appearance::TEXT),
        );
        let half = ((rect.width() - title_width) / 2.0 - 12.0).max(32.0);
        let mut left = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("titlebar-left")
                .max_rect(egui::Rect::from_min_size(
                    rect.min,
                    egui::vec2(half, rect.height()),
                ))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        left.add_enabled_ui(!locked, |ui| {
            if layout.left_menu {
                let button = icon(ui, egui_phosphor::regular::LIST, "Document actions");
                egui::Popup::menu(&button).show(|ui| {
                    ui.set_min_width(240.0);
                    self.left_title_actions(ui, true);
                });
            } else {
                self.left_title_actions(ui, false);
            }
        });
        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("titlebar-right")
                .max_rect(egui::Rect::from_min_size(
                    egui::pos2(rect.right() - half, rect.top()),
                    egui::vec2(half, rect.height()),
                ))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        if layout.right_menu {
            let button = icon(
                &mut right,
                egui_phosphor::regular::DOTS_THREE,
                "More actions",
            );
            egui::Popup::menu(&button).show(|ui| {
                ui.set_min_width(180.0);
                self.right_title_actions(ui, true);
            });
        } else {
            self.right_title_actions(&mut right, false);
            if cfg!(target_arch = "wasm32") && layout.download {
                let download = right.button(crate::i18n::tr("Download LAML for local use"));
                super::chrome::paint_download_glow(&right, &download);
                if download.clicked() {
                    self.download.show(right.ctx());
                }
                let title_rect =
                    egui::Rect::from_center_size(rect.center(), egui::vec2(150.0, 24.0));
                super::chrome::download_mcp_hint(&mut right, download.rect.height(), title_rect);
            }
        }
    }
    fn left_title_width(&self, ui: &egui::Ui) -> f32 {
        use egui_phosphor::regular::*;
        let browser = laml_ui::services::browser_mode();
        let mut icons = vec![PLUS, FOLDER_OPEN];
        if browser {
            icons.insert(0, BOOKS);
        }
        if !self.tabs.is_empty() {
            if browser {
                icons.push(DOWNLOAD_SIMPLE);
            } else {
                icons.extend([FLOPPY_DISK, FLOPPY_DISK]);
            }
            icons.extend([EXPORT, ARROW_COUNTER_CLOCKWISE, ARROW_CLOCKWISE]);
        }
        let gap = ui.spacing().item_spacing.x;
        let separators = if self.tabs.is_empty() {
            0.0
        } else {
            2.0 * (6.0 + gap)
        };
        group_width(ui, &icons) + separators + if self.recovery_count() > 0 { 56.0 } else { 0.0 }
    }
    fn left_title_actions(&mut self, ui: &mut egui::Ui, menu: bool) {
        if !laml_ui::services::browser_mode() {
            if menu {
                let count = self.recovery_count();
                if count > 0
                    && action(
                        ui,
                        egui_phosphor::regular::CLOCK_COUNTER_CLOCKWISE,
                        &crate::i18n::format(
                            "Restore unsaved work ({count})",
                            &[("count", count.to_string())],
                        ),
                        true,
                    )
                    .clicked()
                {
                    self.open_recovery();
                }
            } else {
                self.recovery_button(ui);
            }
        }
        #[cfg(target_arch = "wasm32")]
        if laml_ui::services::browser_mode()
            && action(ui, egui_phosphor::regular::BOOKS, "Browser library", menu).clicked()
        {
            self.library.open = true;
        }
        if action(ui, PLUS, "New LAML (Ctrl/Cmd+N)", menu).clicked() {
            self.begin_new_diagram();
        }
        if action(ui, FOLDER_OPEN, "Open LAML (Ctrl/Cmd+O)", menu).clicked() {
            self.open_dialog(ui.ctx());
        }
        if let Some(editor) = self.tabs.get_mut(self.active) {
            ui.add(egui::Separator::default().spacing(6.0));
            editor.document_title_actions_mode(ui, menu);
        }
    }
    fn right_title_actions(&mut self, ui: &mut egui::Ui, menu: bool) {
        // Right-to-left outside; natural top-to-bottom order inside the menu.
        let items = if menu {
            [
                (GEAR_SIX, "Settings"),
                (QUESTION, "Help"),
                (egui_phosphor::regular::INFO, "About"),
            ]
        } else {
            [
                (egui_phosphor::regular::INFO, "About"),
                (QUESTION, "Help"),
                (GEAR_SIX, "Settings"),
            ]
        };
        for (glyph, label) in items {
            if action(ui, glyph, label, menu).clicked() {
                match label {
                    "Settings" => self.settings_open = true,
                    "Help" => self.help_open = true,
                    _ => self.about_open = true,
                }
            }
        }
    }
}
pub(in crate::app) fn action(
    ui: &mut egui::Ui,
    glyph: &str,
    label: &str,
    menu: bool,
) -> egui::Response {
    if !menu {
        return icon(ui, glyph, label);
    }
    let item = toolbar::Item {
        label,
        icon: Some(glyph),
        enabled: true,
        separator_before: false,
    };
    let response = toolbar::button(ui, &item, true);
    if response.clicked() {
        ui.close();
    }
    response
}
fn group_width(ui: &egui::Ui, icons: &[&str]) -> f32 {
    icons
        .iter()
        .map(|glyph| {
            let font = egui::FontId::new(19.0, egui::FontFamily::Name("phosphor".into()));
            (ui.painter()
                .layout_no_wrap((*glyph).into(), font, ui.visuals().text_color())
                .size()
                .x
                + 2.0 * ui.spacing().button_padding.x)
                .max(32.0)
        })
        .sum::<f32>()
        + icons.len().saturating_sub(1) as f32 * ui.spacing().item_spacing.x
}
fn mobile_browser() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Reflect::get(&js_sys::global(), &"navigator".into())
            .ok()
            .and_then(|nav| js_sys::Reflect::get(&nav, &"userAgent".into()).ok())
            .and_then(|ua| ua.as_string())
            .is_some_and(|ua| {
                ua.contains("Mobi") || ua.contains("iPhone") || ua.contains("Android")
            })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn title_never_overlaps_action_groups_at_intermediate_widths() {
        let ctx = egui::Context::default();
        appearance::install(&ctx);
        let mut w = Workspace::new(Editor::new("semantics { A }".into(), None));
        for width in (280..=1200).step_by(20) {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width as f32, 700.0),
                    )),
                    ..Default::default()
                },
                |ui| w.chrome(ui),
            );
            let labels: Vec<_> = out
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.pos.y < 40.0 => Some((
                        t.galley.job.text.as_str(),
                        egui::Rect::from_min_size(t.pos, t.galley.size()),
                    )),
                    _ => None,
                })
                .collect();
            let title = labels.iter().find(|(s, _)| *s == "LAML Editor").unwrap().1;
            for (label, rect) in &labels {
                if *label != "LAML Editor" {
                    assert!(
                        !rect.intersects(title),
                        "width={width} {label:?}: {rect:?} vs {title:?}"
                    );
                }
                assert!(
                    rect.left() >= 0.0 && rect.right() <= width as f32,
                    "width={width}: {rect:?}"
                );
            }
            out.textures_delta.clear();
        }
    }
    #[test]
    fn download_disappears_before_whole_groups_collapse_and_never_on_mobile() {
        let wide = Layout::new(1500.0, 342.0, 116.0, 130.0, 230.0, false);
        assert!(wide.download && !wide.left_menu && !wide.right_menu);
        let medium = Layout::new(850.0, 342.0, 116.0, 130.0, 230.0, false);
        assert!(!medium.download && !medium.left_menu && !medium.right_menu);
        let narrow = Layout::new(320.0, 342.0, 116.0, 130.0, 230.0, true);
        assert!(narrow.left_menu && narrow.right_menu && !narrow.download);
        assert!(!Layout::new(1500.0, 342.0, 116.0, 130.0, 230.0, true).download);
    }
}
