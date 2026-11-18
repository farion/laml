//! Code-sized text rows. Pointer interaction never transfers keyboard focus from source.
use super::*;
const PAD: f32 = 8.0;
const GAP: f32 = 20.0;
const FOOTER: f32 = 22.0;
fn row(
    ui: &mut egui::Ui,
    label: &str,
    detail: &str,
    selected: bool,
    width: f32,
    height: f32,
    label_width: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::CLICK);
    if ui.is_rect_visible(rect) {
        let visuals = ui.visuals();
        let foreground = if selected {
            visuals.selection.stroke.color
        } else {
            visuals.text_color()
        };
        if selected {
            ui.painter().rect_filled(rect, 0, visuals.selection.bg_fill);
        } else if response.hovered() {
            ui.painter()
                .rect_filled(rect, 0, visuals.widgets.hovered.weak_bg_fill);
        }
        let label_rect = egui::Rect::from_min_max(
            rect.min + egui::vec2(PAD, 0.0),
            egui::pos2(rect.left() + PAD + label_width, rect.bottom()),
        );
        ui.painter()
            .with_clip_rect(ui.clip_rect().intersect(label_rect))
            .text(
                egui::pos2(label_rect.left(), rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::monospace(14.0),
                foreground,
            );
        let detail_rect = egui::Rect::from_min_max(
            egui::pos2(label_rect.right() + GAP, rect.top()),
            rect.right_bottom() - egui::vec2(PAD, 0.0),
        );
        let detail_color = if selected {
            foreground
        } else {
            visuals.weak_text_color()
        };
        let mut job = egui::text::LayoutJob::simple_singleline(
            detail.into(),
            egui::FontId::monospace(12.0),
            detail_color,
        );
        job.wrap.max_width = detail_rect.width().max(0.0);
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        let pos = egui::pos2(
            detail_rect.right() - galley.size().x,
            rect.center().y - galley.size().y * 0.5,
        );
        ui.painter()
            .with_clip_rect(ui.clip_rect().intersect(detail_rect))
            .galley(pos, galley, detail_color);
    }
    response
}
impl Editor {
    pub(in crate::app) fn source_completion_popup(&mut self, ui: &mut egui::Ui) {
        let Some((revision, result)) = &self.advanced.completion else {
            return;
        };
        if *revision != self.document.revision() || self.source_ui.composing {
            self.advanced.completion = None;
            return;
        }
        let Some(anchor) = self.advanced.anchor else {
            return;
        };
        let screen = ui.ctx().content_rect();
        if !screen.intersects(anchor) {
            self.advanced.completion = None;
            return;
        }
        let font = egui::FontId::monospace(14.0);
        let row_height = ui.fonts_mut(|f| f.row_height(&font)).ceil() + 2.0;
        let descriptions: Vec<_> = result
            .entries
            .iter()
            .map(|e| crate::i18n::tr(&e.detail))
            .collect();
        let (label_width, detail_width) = ui.fonts_mut(|f| {
            let mut labels = 0.0_f32;
            let mut details = 0.0_f32;
            for (entry, detail) in result.entries.iter().zip(&descriptions) {
                labels = labels.max(
                    f.layout_no_wrap(entry.label.clone(), font.clone(), egui::Color32::WHITE)
                        .size()
                        .x,
                );
                details = details.max(
                    f.layout_no_wrap(
                        detail.to_string(),
                        egui::FontId::monospace(12.0),
                        egui::Color32::WHITE,
                    )
                    .size()
                    .x,
                );
            }
            (labels, details)
        });
        let width = (label_width + detail_width.min(280.0) + GAP + 2.0 * PAD)
            .clamp(220.0, 620.0)
            .min((screen.width() - 8.0).max(40.0));
        let label_width = label_width.min((width - 2.0 * PAD - GAP).max(0.0));
        let desired_height = (result.entries.len() as f32 * row_height + FOOTER + 2.0).min(322.0);
        let below = (screen.bottom() - anchor.bottom() - 3.0).max(0.0);
        let above = (anchor.top() - screen.top() - 3.0).max(0.0);
        // Stay directly beneath the text, reducing the scrollable height first.
        let use_above = below < row_height + FOOTER + 2.0 && above > below;
        let height = desired_height.min(if use_above { above } else { below });
        let x = anchor.left().clamp(
            screen.left() + 2.0,
            (screen.right() - width - 4.0).max(screen.left() + 2.0),
        );
        let y = if use_above {
            anchor.top() - height - 1.0
        } else {
            anchor.bottom() + 1.0
        };
        let mut picked = None;
        let selected = self.advanced.selected;
        let scroll_to_selected = std::mem::take(&mut self.advanced.scroll_selected);
        let area = egui::Area::new(egui::Id::unique("source-inline-completion"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(x, y))
            .movable(false)
            .sense(egui::Sense::hover())
            .show(ui.ctx(), |ui| {
                egui::Frame::new()
                    .fill(ui.visuals().extreme_bg_color)
                    .stroke(ui.visuals().window_stroke)
                    .corner_radius(1)
                    .show(ui, |ui| {
                        ui.set_width(width);
                        ui.spacing_mut().item_spacing.y = 0.0;
                        egui::ScrollArea::vertical()
                            .max_height((height - FOOTER - 2.0).max(row_height))
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for (index, (entry, detail)) in
                                    result.entries.iter().zip(&descriptions).enumerate()
                                {
                                    let response = row(
                                        ui,
                                        &entry.label,
                                        detail,
                                        selected == Some(index),
                                        width,
                                        row_height,
                                        label_width,
                                    );
                                    #[cfg(test)]
                                    if index == 0 {
                                        self.advanced.first_row = Some(response.rect);
                                    }
                                    if response.clicked() {
                                        picked = Some(index);
                                    }
                                    if scroll_to_selected && selected == Some(index) {
                                        response.scroll_to_me(None);
                                    }
                                }
                            });
                        let (footer, _) =
                            ui.allocate_exact_size(egui::vec2(width, FOOTER), egui::Sense::hover());
                        ui.painter().hline(
                            footer.x_range(),
                            footer.top(),
                            ui.visuals().window_stroke,
                        );
                        let painter = ui
                            .painter()
                            .with_clip_rect(ui.clip_rect().intersect(footer));
                        painter.text(
                            footer.left_center() + egui::vec2(PAD, 0.0),
                            egui::Align2::LEFT_CENTER,
                            crate::i18n::tr("Tab: accept · Esc: close"),
                            egui::FontId::monospace(10.0),
                            ui.visuals().weak_text_color(),
                        );
                    });
            });
        let close = ui.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !area.response.rect.contains(p))
        });
        if let Some(n) = picked {
            self.accept_completion(n);
        } else if close {
            self.advanced.completion = None;
        }
    }
}
