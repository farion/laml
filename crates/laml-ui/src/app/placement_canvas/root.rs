//! Moving onto genuinely empty canvas removes semantic container membership.
use super::*;

pub(super) fn empty(preview: &Preview, canvas: egui::Rect, pointer: egui::Pos2) -> bool {
    // Bounding frames deliberately exclude headers, cube faces and the original
    // container's entire interior from the root-level fallback.
    if preview
        .nodes
        .iter()
        .any(|n| connection_geometry::node_rect(preview, canvas, n).contains(pointer))
    {
        return false;
    }
    let (left, top, right, bottom) = preview.bounds;
    let x = left + f64::from((pointer.x - canvas.left()) / canvas.width()) * (right - left);
    let y = top + f64::from((pointer.y - canvas.top()) / canvas.height()) * (bottom - top);
    let tolerance = 5.0 * (right - left) / f64::from(canvas.width());
    laml_ui::relation_selection::hit(preview, x, y, tolerance).is_none()
        && laml_ui::selection::hit_port(&preview.nodes, x, y, tolerance).is_none()
}

pub(super) fn feedback(ui: &egui::Ui, pointer: egui::Pos2, ready: bool) {
    let color = if ready {
        egui::Color32::LIGHT_GREEN
    } else {
        egui::Color32::YELLOW
    };
    let text = ui.painter().layout_no_wrap(
        crate::i18n::tr("Move out of container").into(),
        egui::FontId::proportional(13.0),
        appearance::color(ui.ctx(), appearance::TEXT),
    );
    let mut rect = egui::Rect::from_min_size(
        pointer + egui::vec2(18.0, 22.0),
        text.size() + egui::vec2(20.0, 12.0),
    );
    let bounds = ui.clip_rect().shrink(4.0);
    rect = rect.translate(egui::vec2(
        (bounds.right() - rect.right()).min(0.0),
        (bounds.bottom() - rect.bottom()).min(0.0),
    ));
    ui.painter()
        .rect_filled(rect, 4.0, appearance::color(ui.ctx(), appearance::PANEL));
    ui.painter().rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.5, color),
        egui::StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.min + egui::vec2(10.0, 6.0), text, color);
}
