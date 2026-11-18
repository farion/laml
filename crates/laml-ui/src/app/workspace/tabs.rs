//! Arrow navigation and native touch/trackpad scrolling, with no visible scrollbar.
use super::*;
#[derive(Clone, Default)]
struct Metrics {
    offset: f32,
    max: f32,
    starts: Vec<f32>,
}
impl Metrics {
    fn step(&self, forward: bool) -> f32 {
        if forward {
            self.starts
                .iter()
                .copied()
                .find(|x| *x > self.offset + 1.0)
                .unwrap_or(self.max)
                .min(self.max)
        } else {
            self.starts
                .iter()
                .copied()
                .rev()
                .find(|x| *x < self.offset - 1.0)
                .unwrap_or(0.0)
        }
    }
}
impl Workspace {
    pub(super) fn tab_bar(&mut self, ui: &mut egui::Ui, locked: bool) {
        if self.tabs.is_empty() {
            return;
        }
        let mut selected = None;
        let mut closed = None;
        let reveal = std::mem::take(&mut self.reveal_active);
        egui::Panel::top("document-tabs")
            .exact_size(38.0)
            .frame(appearance::panel_frame(appearance::color(
                ui.ctx(),
                appearance::HEADER,
            )))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let id = ui.make_persistent_id("tab-scroll-metrics");
                let previous = ui
                    .ctx()
                    .data_mut(|d| d.get_temp::<Metrics>(id))
                    .unwrap_or_default();
                let overflow = previous.max > 0.5;
                let mut requested = None;
                ui.add_enabled_ui(!locked, |ui| {
                    let area = if overflow {
                        rect.shrink2(egui::vec2(34.0, 0.0))
                    } else {
                        rect
                    };
                    for (forward, available, at, glyph, label) in [
                        (
                            false,
                            previous.offset > 0.5,
                            rect.left_top(),
                            egui_phosphor::regular::CARET_LEFT,
                            "Previous tab",
                        ),
                        (
                            true,
                            previous.offset < previous.max - 0.5,
                            egui::pos2(rect.right() - 32.0, rect.top()),
                            egui_phosphor::regular::CARET_RIGHT,
                            "Next tab",
                        ),
                    ] {
                        if overflow && available {
                            let mut arrow = ui.new_child(
                                egui::UiBuilder::new()
                                    .id_salt(("tab-arrow", forward))
                                    .max_rect(egui::Rect::from_min_size(
                                        at,
                                        egui::vec2(32.0, 32.0),
                                    )),
                            );
                            if icon(&mut arrow, glyph, label).clicked() {
                                requested = Some(previous.step(forward));
                            }
                        }
                    }
                    let mut strip =
                        ui.new_child(egui::UiBuilder::new().id_salt("tab-strip").max_rect(area));
                    strip.set_clip_rect(area.intersect(ui.clip_rect()));
                    let mut scroll = egui::ScrollArea::horizontal()
                        .id_salt("open-files")
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .auto_shrink([false, false]);
                    if let Some(offset) = requested {
                        scroll = scroll.horizontal_scroll_offset(offset);
                    }
                    let mut starts = vec![];
                    let output = scroll.show(&mut strip, |ui| {
                        let origin = ui.cursor().left();
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 1.0;
                            for (index, editor) in self.tabs.iter().enumerate() {
                                ui.push_id(editor.id, |ui| {
                                    let active = index == self.active;
                                    let tab = egui::Frame::new()
                                        .fill(appearance::color(
                                            ui.ctx(),
                                            if active {
                                                appearance::PANEL
                                            } else {
                                                appearance::HEADER
                                            },
                                        ))
                                        .inner_margin(egui::Margin::symmetric(8, 2))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing.x = 6.0;
                                                let name = editor
                                                    .path
                                                    .file_name()
                                                    .unwrap_or_default()
                                                    .to_string_lossy();
                                                let label = format!(
                                                    "{name}{}{}",
                                                    if editor.recovery.recovered
                                                        && editor.file.path.is_none()
                                                    {
                                                        " (recovered)"
                                                    } else {
                                                        ""
                                                    },
                                                    if !laml_ui::services::browser_mode()
                                                        && editor.document.is_dirty()
                                                    {
                                                        " •"
                                                    } else {
                                                        ""
                                                    }
                                                );
                                                if ui
                                                    .add(
                                                        egui::Button::new(label)
                                                            .wrap_mode(egui::TextWrapMode::Extend)
                                                            .frame(false),
                                                    )
                                                    .on_hover_text(
                                                        editor.path.display().to_string(),
                                                    )
                                                    .clicked()
                                                {
                                                    selected = Some(index);
                                                }
                                                if icon(ui, X, "Close file").clicked() {
                                                    closed = Some(index);
                                                }
                                            });
                                        });
                                    starts.push(tab.response.rect.left() - origin);
                                    if active {
                                        if reveal {
                                            tab.response.scroll_to_me(Some(egui::Align::Center));
                                        }
                                        ui.painter().hline(
                                            tab.response.rect.x_range(),
                                            tab.response.rect.bottom() - 1.0,
                                            egui::Stroke::new(
                                                2.0,
                                                appearance::color(ui.ctx(), appearance::ACCENT),
                                            ),
                                        );
                                    }
                                });
                            }
                        });
                    });
                    let max = (output.content_size.x - output.inner_rect.width()).max(0.0);
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(
                            id,
                            Metrics {
                                offset: output.state.clamped_offset().x,
                                max,
                                starts,
                            },
                        )
                    });
                    if overflow != (max > 0.5) {
                        ui.ctx().request_repaint();
                    }
                });
                ui.painter().hline(
                    rect.x_range(),
                    rect.bottom(),
                    egui::Stroke::new(1.0, appearance::color(ui.ctx(), appearance::BORDER)),
                );
            });
        if let Some(index) = selected {
            self.active = index;
            self.reveal_active = true;
        }
        if let Some(index) = closed {
            self.request_close(index, ui.ctx());
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arrows_scroll_by_tab_boundaries_and_clamp() {
        let mut m = Metrics {
            offset: 0.0,
            max: 250.0,
            starts: vec![0.0, 100.0, 220.0, 400.0],
        };
        assert_eq!(m.step(true), 100.0);
        m.offset = 120.0;
        assert_eq!(m.step(false), 100.0);
        assert_eq!(m.step(true), 220.0);
        m.offset = 220.0;
        assert_eq!(m.step(true), 250.0);
    }
}
