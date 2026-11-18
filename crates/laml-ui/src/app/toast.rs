//! Transient non-modal feedback for gestures that leave the source unchanged.
use super::*;

#[cfg(test)]
mod tests;

const TIMEOUT: Duration = Duration::from_secs(5);

fn remaining(elapsed: Duration) -> f32 {
    (1.0 - elapsed.as_secs_f32() / TIMEOUT.as_secs_f32()).clamp(0.0, 1.0)
}

fn countdown(center: egui::Pos2, radius: f32, fraction: f32) -> Vec<egui::Pos2> {
    let sweep = std::f32::consts::TAU * fraction.clamp(0.0, 1.0);
    let start = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU - sweep;
    let steps = (64.0 * fraction).ceil().max(1.0) as usize;
    (0..=steps)
        .map(|i| {
            let angle = start + sweep * i as f32 / steps as f32;
            center + radius * egui::vec2(angle.cos(), angle.sin())
        })
        .collect()
}

fn dismiss(ui: &mut egui::Ui, fraction: f32) -> egui::Response {
    static SVG: std::sync::OnceLock<egui::load::Bytes> = std::sync::OnceLock::new();
    let bytes = SVG.get_or_init(|| {
        include_str!("../../../../assets/phosphor/regular/x.svg")
            .replace("currentColor", "#ffffff")
            .into_bytes()
            .into()
    });
    let icon = egui::Image::from_bytes("bytes://toast-phosphor-x.svg", bytes.clone())
        .tint(appearance::color(ui.ctx(), appearance::TEXT))
        .fit_to_exact_size(egui::Vec2::splat(12.0));
    let response = ui.add_sized(
        egui::Vec2::splat(28.0),
        egui::Button::image(icon)
            .frame_when_inactive(false)
            .corner_radius(14)
            .stroke(egui::Stroke::NONE),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::Role::Button, true, crate::i18n::tr("Dismiss"))
    });
    let center = response.rect.center();
    let radius = 11.5;
    let color = egui::Color32::LIGHT_RED;
    ui.painter().circle_stroke(
        center,
        radius,
        egui::Stroke::new(1.5, color.gamma_multiply(0.2)),
    );
    if fraction > 0.0 {
        ui.painter().add(egui::Shape::line(
            countdown(center, radius, fraction),
            egui::Stroke::new(1.5, color),
        ));
    }
    response.on_hover_text(crate::i18n::tr("Dismiss"))
}

pub(super) struct Toast {
    message: String,
    since: Instant,
}
impl Editor {
    pub(super) fn error_toast(&mut self, message: impl Into<String>) {
        self.graph.toast = Some(Toast {
            message: message.into(),
            since: Instant::now(),
        });
    }
    pub(super) fn toast_overlay(&mut self, ctx: &egui::Context) {
        let Some(toast) = &self.graph.toast else {
            return;
        };
        let elapsed = toast.since.elapsed();
        if elapsed >= TIMEOUT {
            self.graph.toast = None;
            return;
        }
        let mut close = false;
        egui::Area::new(egui::Id::unique(("gesture-toast", self.id)))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(16.0, -48.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(appearance::color(ctx, appearance::PANEL))
                    .stroke(egui::Stroke::new(1.5, egui::Color32::LIGHT_RED))
                    .corner_radius(6)
                    .inner_margin(12)
                    .show(ui, |ui| {
                        ui.set_max_width(440.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(egui_phosphor::regular::WARNING_CIRCLE)
                                    .family(egui::FontFamily::Name("phosphor".into()))
                                    .color(egui::Color32::LIGHT_RED),
                            );
                            ui.add(egui::Label::new(crate::i18n::message(&toast.message)).wrap());
                            close = dismiss(ui, remaining(elapsed)).clicked();
                        });
                    });
            });
        ctx.request_repaint_after(Duration::from_millis(16));
        if close {
            self.graph.toast = None;
        }
    }
}
