//! Gesture strokes use the compiler's path, head geometry, theme and LSS appearance.
use super::canvas::point;
use super::*;
use laml::sequence::{Message, MessageKind, render::message_geometry as geometry};

#[allow(clippy::too_many_arguments)] // Explicit compiler geometry for transient strokes.
pub(super) fn message(
    ui: &egui::Ui,
    preview: &Preview,
    rect: egui::Rect,
    message: &Message,
    send: (f64, f64),
    receive: (f64, f64),
    self_message: bool,
    diagonal: bool,
) {
    let scale = rect.width() / (preview.bounds.2 - preview.bounds.0) as f32;
    let mut sheet = preview.styles.clone();
    sheet.theme = preview.diagram.settings.theme;
    let (color, width) = geometry::appearance(&sheet, message.name.as_deref(), &message.classes);
    if color == "none" {
        return;
    }
    let color = egui::Color32::from_hex(&color)
        .unwrap_or_else(|_| appearance::color(ui.ctx(), appearance::TEXT));
    let stroke = egui::Stroke::new((width as f32 * scale).max(0.5), color);
    let path = geometry::path(send, receive, self_message, diagonal);
    let points: Vec<_> = path
        .iter()
        .map(|p| point(rect, preview.bounds, p.0, p.1))
        .collect();
    if geometry::dashed(message.kind) {
        ui.painter().extend(egui::Shape::dashed_line(
            &points,
            stroke,
            7.0 * scale,
            5.0 * scale,
        ));
    } else {
        ui.painter().add(egui::Shape::line(points, stroke));
    }
    if message.kind == MessageKind::Delete {
        let center = point(rect, preview.bounds, receive.0, receive.1);
        let d = egui::vec2(8.0, 8.0) * scale;
        ui.painter().line_segment([center - d, center + d], stroke);
        ui.painter().line_segment(
            [
                center - egui::vec2(d.x, -d.y),
                center + egui::vec2(d.x, -d.y),
            ],
            stroke,
        );
    } else if message.to != "?" {
        let previous = path[path.len() - 2];
        let head = geometry::arrowhead(
            receive,
            (receive.0 - previous.0, receive.1 - previous.1),
            sheet.theme.modern(),
        );
        let p: Vec<_> = head
            .iter()
            .map(|p| point(rect, preview.bounds, p.0, p.1))
            .collect();
        if geometry::filled(message.kind, message.filled_reply) {
            ui.painter()
                .add(egui::Shape::convex_polygon(p, color, egui::Stroke::NONE));
        } else {
            ui.painter().add(egui::Shape::line(
                vec![p[1], p[0], p[2]],
                egui::Stroke::new(1.5 * scale, color),
            ));
        }
    }
    if message.from == "?" || message.to == "?" {
        let at = if message.from == "?" { send } else { receive };
        ui.painter()
            .circle_filled(point(rect, preview.bounds, at.0, at.1), 4.0 * scale, color);
    }
}
#[allow(clippy::too_many_arguments)] // Explicit compiler geometry for transient activations.
pub(super) fn starting_bar(
    ui: &egui::Ui,
    preview: &Preview,
    rect: egui::Rect,
    source: &str,
    target: &targets::Target,
    toward_x: f64,
    self_message: bool,
    receive_y: f64,
) -> (f64, f64) {
    let mut sheet = preview.styles.clone();
    sheet.theme = preview.diagram.settings.theme;
    let classes = laml::sequence::parse(source)
        .ok()
        .and_then(|d| d.participants.into_iter().find(|p| p.id == target.owner))
        .map_or_else(Vec::new, |p| p.classes);
    let (fill, border, width, border_width) =
        geometry::activation_appearance(&sheet, &target.owner, &classes);
    let line_x = preview
        .regions
        .iter()
        .find(|r| r.kind == "lifeline" && r.id == target.owner)
        .map_or(target.x, |r| r.x);
    let center = target
        .bar
        .as_ref()
        .map_or(line_x, |bar| bar.x + bar.width / 2.0 + 5.0);
    let side = if self_message || toward_x >= center {
        1.0
    } else {
        -1.0
    };
    let scale = rect.width() / (preview.bounds.2 - preview.bounds.0) as f32;
    let r = egui::Rect::from_min_max(
        point(rect, preview.bounds, center - width / 2.0, receive_y),
        point(rect, preview.bounds, center + width / 2.0, receive_y + 48.0),
    );
    let radius = if sheet.theme.modern() {
        (3.0 * scale) as u8
    } else {
        0
    };
    if let Ok(fill) = egui::Color32::from_hex(&fill) {
        ui.painter().rect_filled(r, radius, fill);
    }
    if let Ok(border) = egui::Color32::from_hex(&border) {
        ui.painter().rect_stroke(
            r,
            radius,
            egui::Stroke::new(border_width as f32 * scale, border),
            egui::StrokeKind::Middle,
        );
    }
    (center + side * width / 2.0, receive_y)
}
pub(super) fn drops(
    ui: &egui::Ui,
    preview: &Preview,
    rect: egui::Rect,
    targets: &[targets::Target],
    hot: Option<&targets::Target>,
    valid: bool,
) {
    let accent = appearance::color(ui.ctx(), appearance::ACCENT);
    let hot_color = if valid {
        egui::Color32::LIGHT_GREEN
    } else {
        egui::Color32::LIGHT_RED
    };
    for target in targets {
        let active = hot.is_some_and(|t| {
            t.owner == target.owner
                && t.before == target.before
                && t.activation_line == target.activation_line
                && (t.y - target.y).abs() < 0.1
        });
        let center = point(rect, preview.bounds, target.x, target.y);
        if !ui.clip_rect().expand(8.0).contains(center) {
            continue;
        }
        let radius = if active { 6.5 } else { 4.0 };
        let stroke = egui::Stroke::new(
            if active { 2.0 } else { 1.0 },
            if active {
                hot_color
            } else {
                accent.gamma_multiply(0.7)
            },
        );
        if target.activation_line.is_some() {
            let r = egui::Rect::from_center_size(center, egui::Vec2::splat(radius * 2.0));
            ui.painter()
                .rect_filled(r, 2.0, appearance::color(ui.ctx(), appearance::PANEL));
            ui.painter()
                .rect_stroke(r, 2.0, stroke, egui::StrokeKind::Middle);
        } else {
            ui.painter().circle_filled(
                center,
                radius,
                appearance::color(ui.ctx(), appearance::PANEL),
            );
            ui.painter().circle_stroke(center, radius, stroke);
        }
    }
    if let Some(hot) = hot {
        if let Some(bar) = &hot.bar {
            let r = egui::Rect::from_min_max(
                point(rect, preview.bounds, bar.x, bar.y),
                point(rect, preview.bounds, bar.x + bar.width, bar.y + bar.height),
            );
            ui.painter().rect_stroke(
                r.expand(3.0),
                2.0,
                egui::Stroke::new(1.5, hot_color),
                egui::StrokeKind::Outside,
            );
        }
        if hot.activation_line.is_some() {
            ui.painter().text(
                point(rect, preview.bounds, hot.x, hot.y) + egui::vec2(12.0, -18.0),
                egui::Align2::LEFT_BOTTOM,
                crate::i18n::tr("Start activation"),
                egui::FontId::proportional(12.0),
                hot_color,
            );
        }
        let row = point(rect, preview.bounds, hot.x, hot.row_y).y;
        ui.painter().hline(
            rect.left()..=rect.right(),
            row,
            egui::Stroke::new(1.0, hot_color.gamma_multiply(0.35)),
        );
    }
}
