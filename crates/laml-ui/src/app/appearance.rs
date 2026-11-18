//! Mockup-derived UI tokens, independent of the document's diagram theme.
use super::*;

pub(super) const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(30, 33, 41);
pub(super) const PANEL: egui::Color32 = egui::Color32::from_rgb(38, 41, 50);
pub(super) const HEADER: egui::Color32 = egui::Color32::from_rgb(28, 31, 39);
pub(super) const BORDER: egui::Color32 = egui::Color32::from_rgb(61, 65, 77);
pub(super) const TEXT: egui::Color32 = egui::Color32::from_rgb(228, 229, 234);
pub(super) const MUTED: egui::Color32 = egui::Color32::from_rgb(148, 153, 166);
pub(super) const ACCENT: egui::Color32 = egui::Color32::from_rgb(78, 117, 225);
pub(super) const SELECTED: egui::Color32 = egui::Color32::from_rgb(158, 149, 245);
pub(super) const HEADER_HEIGHT: f32 = 46.0;
pub(super) const FIELD_HEIGHT: f32 = 32.0;

pub(super) fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Inter".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../../assets/Inter-Regular.otf"
        ))),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "Inter".into());
    fonts.font_data.insert(
        "Inter-Bold".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../../assets/Inter-Bold.otf"
        ))),
    );
    fonts.families.insert(
        egui::FontFamily::Name("bold".into()),
        vec!["Inter-Bold".into(), "Inter".into()],
    );
    // egui-phosphor currently targets egui 0.36; register its version-independent
    // font bytes in our egui 0.37 definitions rather than mixing egui types.
    fonts.font_data.insert(
        "phosphor".into(),
        std::sync::Arc::new(egui::FontData::from_static(
            egui_phosphor::Variant::Regular.font_bytes(),
        )),
    );
    // A dedicated family prevents Inter's own private-use glyphs from shadowing icons.
    fonts.families.insert(
        egui::FontFamily::Name("phosphor".into()),
        vec!["phosphor".into()],
    );
    source_highlight::install_fonts(&mut fonts);
    ctx.set_fonts(fonts);
    let mut style = egui::Style {
        visuals: egui::Visuals::dark(),
        ..Default::default()
    };
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.panel_fill = PANEL;
    style.visuals.window_fill = PANEL;
    style.visuals.extreme_bg_color = BACKGROUND;
    style.visuals.faint_bg_color = HEADER;
    style.visuals.code_bg_color = BACKGROUND;
    style.visuals.window_stroke = egui::Stroke::new(1.0, BORDER);
    style.visuals.window_corner_radius = egui::CornerRadius::same(6);
    style.visuals.menu_corner_radius = egui::CornerRadius::same(6);
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(49, 65, 104);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, SELECTED);
    for (widget, fill, stroke) in [
        (&mut style.visuals.widgets.noninteractive, PANEL, BORDER),
        (
            &mut style.visuals.widgets.inactive,
            egui::Color32::from_rgb(46, 50, 60),
            BORDER,
        ),
        (
            &mut style.visuals.widgets.hovered,
            egui::Color32::from_rgb(55, 61, 76),
            MUTED,
        ),
        (
            &mut style.visuals.widgets.active,
            egui::Color32::from_rgb(56, 73, 118),
            ACCENT,
        ),
        (
            &mut style.visuals.widgets.open,
            egui::Color32::from_rgb(47, 54, 72),
            ACCENT,
        ),
    ] {
        widget.bg_fill = fill;
        widget.weak_bg_fill = fill;
        widget.bg_stroke = egui::Stroke::new(1.0, stroke);
        widget.fg_stroke = egui::Stroke::new(1.0, TEXT);
        widget.corner_radius = egui::CornerRadius::same(5);
        widget.expansion = 0.0;
    }
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.interact_size = egui::vec2(32.0, FIELD_HEIGHT);
    style.spacing.text_edit_width = 146.0;
    style.spacing.combo_width = 146.0;
    style.spacing.slider_width = 100.0;
    style.spacing.indent = 14.0;
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(15.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(17.0));
    style
        .text_styles
        .insert(egui::TextStyle::Monospace, egui::FontId::monospace(15.0));
    ctx.set_style_of(egui::Theme::Dark, style.clone());
    style.visuals = egui::Visuals::light();
    style.visuals.override_text_color = Some(light(TEXT));
    style.visuals.panel_fill = light(PANEL);
    style.visuals.window_fill = light(PANEL);
    style.visuals.extreme_bg_color = light(BACKGROUND);
    style.visuals.faint_bg_color = light(HEADER);
    style.visuals.code_bg_color = light(BACKGROUND);
    style.visuals.window_stroke = egui::Stroke::new(1.0, light(BORDER));
    style.visuals.window_corner_radius = egui::CornerRadius::same(6);
    style.visuals.menu_corner_radius = egui::CornerRadius::same(6);
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(210, 224, 255);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, light(SELECTED));
    for widget in [
        &mut style.visuals.widgets.noninteractive,
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::same(5);
        widget.expansion = 0.0;
    }
    ctx.set_style_of(egui::Theme::Light, style);
    ctx.options_mut(|options| options.fallback_theme = egui::Theme::Dark);
    super::editor_theme::Preferences::load().apply(ctx);
}

pub(super) fn panel_frame(fill: egui::Color32) -> egui::Frame {
    egui::Frame::new().fill(fill).inner_margin(0)
}
pub(super) fn floating_frame(ctx: &egui::Context) -> egui::Frame {
    egui::Frame::new()
        .fill(color(ctx, PANEL))
        .stroke(egui::Stroke::new(1.0, color(ctx, BORDER)))
        .corner_radius(6)
        .inner_margin(10)
}

pub(super) fn pane_heading(ui: &mut egui::Ui, title: &str) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), HEADER_HEIGHT),
        egui::Sense::hover(),
    );
    ui.painter().rect_filled(rect, 0, color(ui.ctx(), HEADER));
    ui.painter().text(
        rect.left_center() + egui::vec2(16.0, 0.0),
        egui::Align2::LEFT_CENTER,
        crate::i18n::tr(title),
        egui::FontId::proportional(16.0),
        color(ui.ctx(), TEXT),
    );
    ui.painter().hline(
        rect.x_range(),
        rect.bottom(),
        egui::Stroke::new(1.0, color(ui.ctx(), BORDER)),
    );
}

/// Screen-space dots mark the exact compiler image bounds, never exported.
pub(super) fn image_bounds(painter: &egui::Painter, rect: egui::Rect) {
    let visible = rect.intersect(painter.clip_rect().expand(1.0));
    if !visible.is_positive() {
        return;
    }
    // Only iterate visible coordinates, even for very large zoomed images.
    let mut x = rect.left() + ((visible.left() - rect.left()) / 5.0).ceil() * 5.0;
    while x <= visible.right() {
        for y in [rect.top(), rect.bottom()] {
            painter.circle_filled(egui::pos2(x, y), 0.8, color(painter.ctx(), MUTED));
        }
        x += 5.0;
    }
    let mut y = rect.top() + ((visible.top() - rect.top()) / 5.0).ceil() * 5.0;
    while y <= visible.bottom() {
        for x in [rect.left(), rect.right()] {
            painter.circle_filled(egui::pos2(x, y), 0.8, color(painter.ctx(), MUTED));
        }
        y += 5.0;
    }
}

#[cfg(test)]
pub(super) fn alignment_button(
    ui: &mut egui::Ui,
    enabled: bool,
    horizontal: bool,
    width: f32,
) -> egui::Response {
    let response = ui.add_enabled(
        enabled,
        egui::Button::new("")
            .frame(false)
            .min_size(egui::vec2(width, 44.0)),
    );
    let painter = ui.painter().with_clip_rect(response.rect);
    let color = if enabled {
        color(ui.ctx(), TEXT)
    } else {
        color(ui.ctx(), MUTED)
    };
    let center = response.rect.center_top() + egui::vec2(0.0, 8.0);
    let stroke = egui::Stroke::new(1.0, color);
    if horizontal {
        painter.line_segment(
            [
                center - egui::vec2(10.0, 0.0),
                center + egui::vec2(10.0, 0.0),
            ],
            stroke,
        );
        for x in [-7.0, 0.0, 7.0] {
            painter.line_segment(
                [center + egui::vec2(x, -5.0), center + egui::vec2(x, 5.0)],
                stroke,
            );
        }
    } else {
        painter.line_segment(
            [center - egui::vec2(0.0, 7.0), center + egui::vec2(0.0, 7.0)],
            stroke,
        );
        for y in [-5.0, 0.0, 5.0] {
            painter.line_segment(
                [center + egui::vec2(-6.0, y), center + egui::vec2(6.0, y)],
                stroke,
            );
        }
    }
    painter.text(
        response.rect.center_bottom() - egui::vec2(0.0, 3.0),
        egui::Align2::CENTER_BOTTOM,
        match (horizontal, width >= 115.0) {
            (true, true) => crate::i18n::tr("Align Horizontal"),
            (false, true) => crate::i18n::tr("Align Vertical"),
            (true, false) => crate::i18n::tr("Horizontal"),
            (false, false) => crate::i18n::tr("Vertical"),
        },
        egui::FontId::proportional(14.0),
        color,
    );
    response.on_hover_ui(|ui| {
        hover_help::show(
            ui,
            if horizontal {
                crate::i18n::tr("Align centers on a horizontal line (center-y)")
            } else {
                crate::i18n::tr("Align centers on a vertical line (center-x)")
            },
        )
    })
}

pub(super) fn property_action(ui: &mut egui::Ui, reset: bool) -> egui::Response {
    let glyph = if reset {
        egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE
    } else {
        egui_phosphor::regular::CHECK
    };
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.x = 24.0;
        // Keep the 24pt control column without clipping the 18pt icon.
        ui.spacing_mut().button_padding.x = 0.0;
        ui.add_sized(
            [24.0, FIELD_HEIGHT],
            egui::Button::new(egui::RichText::new(glyph).font(egui::FontId::new(
                18.0,
                egui::FontFamily::Name("phosphor".into()),
            ))),
        )
    })
    .inner
    .on_hover_text(if reset {
        crate::i18n::tr("Reset local override")
    } else {
        crate::i18n::tr("Apply locally")
    })
}

/// The custom chrome uses the same palette as the active egui editor theme.
pub(super) fn color(ctx: &egui::Context, dark: egui::Color32) -> egui::Color32 {
    if ctx.theme() == egui::Theme::Dark {
        dark
    } else {
        light(dark)
    }
}
fn light(dark: egui::Color32) -> egui::Color32 {
    match dark {
        BACKGROUND => egui::Color32::from_rgb(250, 251, 253),
        PANEL => egui::Color32::from_rgb(244, 246, 250),
        HEADER => egui::Color32::from_rgb(233, 237, 244),
        BORDER => egui::Color32::from_rgb(199, 205, 216),
        TEXT => egui::Color32::from_rgb(30, 36, 48),
        MUTED => egui::Color32::from_rgb(96, 105, 123),
        ACCENT => egui::Color32::from_rgb(46, 86, 194),
        SELECTED => egui::Color32::from_rgb(106, 74, 194),
        _ => dark,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn property_actions_use_centered_phosphor_glyphs_in_the_existing_control_column() {
        let ctx = egui::Context::default();
        install(&ctx);
        for reset in [true, false] {
            let mut rect = egui::Rect::NOTHING;
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                rect = property_action(ui, reset).rect;
            });
            out.textures_delta.clear();
            assert!((rect.width() - 24.0).abs() < 0.1);
            assert!((rect.height() - FIELD_HEIGHT).abs() < 0.1);
            let glyph = if reset {
                egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE
            } else {
                egui_phosphor::regular::CHECK
            };
            let text = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.job.text == glyph => Some(t),
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                text.galley.job.sections[0].format.font_id.family,
                egui::FontFamily::Name("phosphor".into())
            );
            let bounds = egui::Rect::from_min_size(text.pos, text.galley.size());
            assert!(rect.contains_rect(bounds));
            assert!((bounds.center() - rect.center()).length() < 1.0);
        }
    }

    #[test]
    fn mockup_tokens_apply_to_every_widget_state() {
        let ctx = egui::Context::default();
        install(&ctx);
        let style = ctx.style_of(egui::Theme::Dark);
        assert_eq!(style.visuals.panel_fill, PANEL);
        assert_eq!(style.spacing.interact_size.y, FIELD_HEIGHT);
        assert_eq!(
            style.visuals.widgets.inactive.corner_radius,
            egui::CornerRadius::same(5)
        );
        assert_eq!(style.visuals.widgets.hovered.expansion, 0.0);
        assert_eq!(style.visuals.extreme_bg_color, BACKGROUND);
    }
}
