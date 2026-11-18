//! Fixed-width line numbers beside the code's shared horizontal/vertical viewport.
use super::*;
use egui::text::CCursor;
use std::ops::Range;

#[cfg(test)]
mod tests;

pub(super) struct Gutter {
    rect: egui::Rect,
    painter: egui::Painter,
}
impl Gutter {
    pub(super) fn reserve(ui: &mut egui::Ui, text: &str, visible: bool) -> Option<Self> {
        if !visible {
            return None;
        }
        let lines = text.bytes().filter(|&b| b == b'\n').count() + 1;
        let digits = (lines.ilog10() + 1) as f32;
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(digits * 9.0 + 24.0, ui.available_height()),
            egui::Sense::hover(),
        );
        Some(Self {
            rect,
            painter: ui.painter().clone(),
        })
    }
    pub(super) fn paint(
        &self,
        ui: &egui::Ui,
        text: &str,
        galley: &egui::Galley,
        position: egui::Pos2,
        diagnostic: Option<&Range<usize>>,
    ) {
        // Use the code viewport's vertical clip, but never its horizontal offset.
        let clip = egui::Rect::from_x_y_ranges(self.rect.x_range(), ui.clip_rect().y_range())
            .intersect(self.rect);
        let painter = self.painter.with_clip_rect(clip);
        let index = laml_ui::source::SourceIndex::new(text);
        let offsets = std::iter::once(0).chain(
            text.chars()
                .enumerate()
                .filter_map(|(i, c)| (c == '\n').then_some(i + 1)),
        );
        for (line, offset) in offsets.enumerate() {
            let row = galley.pos_from_cursor(CCursor::new(offset));
            let point = egui::pos2(self.rect.right() - 8.0, position.y + row.top());
            if point.y + row.height() < clip.top() || point.y > clip.bottom() {
                continue;
            }
            let marked = diagnostic
                .is_some_and(|d| index.line(line + 1).is_some_and(|r| r.start == d.start));
            painter.text(
                point,
                egui::Align2::RIGHT_TOP,
                (line + 1).to_string(),
                egui::FontId::monospace(14.0),
                if marked {
                    egui::Color32::LIGHT_RED
                } else {
                    ui.visuals().weak_text_color()
                },
            );
        }
    }
}
