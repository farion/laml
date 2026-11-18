//! Global actions and document tabs, including the empty workspace.
use super::*;

impl Workspace {
    pub(super) fn chrome(&mut self, ui: &mut egui::Ui) {
        let locked = self.welcome_open
            || self.opening.is_some()
            || self.file_picker.is_some()
            || self.waiting_close.is_some()
            || self
                .tabs
                .get(self.active)
                .is_some_and(Editor::close_dialog_active);
        egui::Panel::top("global-title")
            .exact_size(44.0)
            .show_separator_line(false)
            .frame(
                appearance::panel_frame(appearance::color(ui.ctx(), appearance::HEADER))
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ui, |ui| self.titlebar(ui, locked));
        self.tab_bar(ui, locked);
    }
}

/// Fixed layout space keeps the text and download button still while the arrow nudges.
pub(super) fn download_mcp_hint(ui: &mut egui::Ui, height: f32, title: egui::Rect) {
    let caption = crate::i18n::tr("Now with MCP");
    let caption_width = ui
        .painter()
        .layout_no_wrap(
            caption.into(),
            egui::TextStyle::Body.resolve(ui.style()),
            ui.visuals().text_color(),
        )
        .size()
        .x;
    let hint_width = caption_width + 32.0 + ui.spacing().item_spacing.x;
    if ui.available_rect_before_wrap().right() - hint_width < title.right() + 12.0 {
        return;
    }
    const PERIOD: f64 = 8.0;
    const MOTION: f64 = 1.4;
    let phase = ui.input(|input| input.time).rem_euclid(PERIOD);
    let offset = if phase < MOTION {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(30));
        let progress = (phase / MOTION) as f32;
        4.0 * (progress * std::f32::consts::TAU * 2.0).sin()
            * (progress * std::f32::consts::PI).sin().powi(2)
    } else {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(PERIOD - phase));
        0.0
    };
    // The titlebar uses right-to-left layout: arrow first, then its caption.
    let (rect, _) = ui.allocate_exact_size(egui::vec2(32.0, height), egui::Sense::hover());
    let center = rect.center() + egui::vec2(offset, 0.0);
    ui.painter().arrow(
        center - egui::vec2(9.0, 0.0),
        egui::vec2(18.0, 0.0),
        egui::Stroke::new(1.6, appearance::color(ui.ctx(), appearance::ACCENT)),
    );
    ui.label(caption);
}

/// A short, softly fading comet follows the actual rounded button border.
pub(super) fn paint_download_glow(ui: &egui::Ui, response: &egui::Response) {
    const PERIOD: f64 = 4.0;
    const LAP: f64 = 2.6;
    let phase = ui.input(|input| input.time).rem_euclid(PERIOD);
    if phase >= LAP {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(PERIOD - phase));
        return;
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(30));
    let rect = response.rect.shrink(0.5);
    let radius = f32::from(ui.style().interact(response).corner_radius.nw)
        .min(rect.height() / 2.0)
        .min(rect.width() / 2.0);
    let straight_x = rect.width() - 2.0 * radius;
    let straight_y = rect.height() - 2.0 * radius;
    let arc = std::f32::consts::FRAC_PI_2 * radius;
    let perimeter = 2.0 * (straight_x + straight_y) + 4.0 * arc;
    if perimeter <= 0.0 {
        return;
    }
    let point = |distance: f32| {
        let mut distance = distance.rem_euclid(perimeter);
        let centers = [
            egui::pos2(rect.right() - radius, rect.top() + radius),
            egui::pos2(rect.right() - radius, rect.bottom() - radius),
            egui::pos2(rect.left() + radius, rect.bottom() - radius),
            egui::pos2(rect.left() + radius, rect.top() + radius),
        ];
        let starts = [
            egui::pos2(rect.left() + radius, rect.top()),
            egui::pos2(rect.right(), rect.top() + radius),
            egui::pos2(rect.right() - radius, rect.bottom()),
            egui::pos2(rect.left(), rect.bottom() - radius),
        ];
        let directions = [
            egui::vec2(1.0, 0.0),
            egui::vec2(0.0, 1.0),
            egui::vec2(-1.0, 0.0),
            egui::vec2(0.0, -1.0),
        ];
        for side in 0..4 {
            let length = if side % 2 == 0 {
                straight_x
            } else {
                straight_y
            };
            if distance <= length {
                return starts[side] + directions[side] * distance;
            }
            distance -= length;
            if distance <= arc && radius > 0.0 {
                let angle = (side as f32 - 1.0) * std::f32::consts::FRAC_PI_2 + distance / radius;
                return centers[side] + egui::vec2(angle.cos(), angle.sin()) * radius;
            }
            distance -= arc;
        }
        starts[0]
    };
    let fade = ((phase / 0.18).min((LAP - phase) / 0.3).min(1.0)) as f32;
    let head = (phase / LAP) as f32 * perimeter;
    let color = ui.visuals().selection.stroke.color;
    let painter = ui
        .painter()
        .with_clip_rect(response.rect.expand(6.0).intersect(ui.clip_rect()));
    for index in (0..24).rev() {
        let strength = (1.0 - index as f32 / 24.0).powi(2) * fade;
        let position = point(head - index as f32 * 1.4);
        painter.circle_filled(position, 4.5, color.gamma_multiply(0.035 * strength));
        painter.circle_filled(position, 2.5, color.gamma_multiply(0.12 * strength));
        painter.circle_filled(position, 1.0, color.gamma_multiply(0.8 * strength));
    }
    painter.circle_filled(point(head), 1.4, color.gamma_multiply(fade));
}
