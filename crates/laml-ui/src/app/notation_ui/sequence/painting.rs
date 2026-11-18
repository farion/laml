//! Transient controls are painted only on hover or during the owning gesture.
use super::*;
use egui::emath::GuiRounding as _;
use egui_phosphor::regular::{PLUS, X};
pub(super) fn icon(ui: &egui::Ui, at: egui::Pos2, text: &str, hot: bool) {
    let color = appearance::color(ui.ctx(), appearance::ACCENT);
    let rect = egui::Rect::from_center_size(at, egui::Vec2::splat(20.0))
        .round_to_pixels(ui.pixels_per_point());
    ui.painter()
        .rect_filled(rect, 4.0, appearance::color(ui.ctx(), appearance::PANEL));
    ui.painter().rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, color.gamma_multiply(if hot { 1.0 } else { 0.5 })),
        egui::StrokeKind::Inside,
    );
    if text == PLUS {
        // The actual Phosphor SVG is symmetric. Font-mesh centering is not:
        // text baseline snapping still moves rasterized glyphs by half a pixel.
        static SVG: std::sync::OnceLock<egui::load::Bytes> = std::sync::OnceLock::new();
        let bytes = SVG.get_or_init(|| {
            include_str!("../../../../../../assets/phosphor/regular/plus.svg")
                .replace("currentColor", "#ffffff")
                .into_bytes()
                .into()
        });
        let ppp = ui.pixels_per_point();
        // Image painting snaps rectangle edges to pixels. Match the button's
        // odd/even pixel extent so snapping cannot shift the icon by half a pixel.
        let edge = |extent: f32| {
            let parity = (extent * ppp).round().rem_euclid(2.0);
            (((15.0 * ppp - parity) / 2.0).round() * 2.0 + parity) / ppp
        };
        let size = egui::vec2(edge(rect.width()), edge(rect.height()));
        egui::Image::from_bytes("bytes://sequence-inline-phosphor-plus.svg", bytes.clone())
            .tint(color)
            .paint_at(ui, egui::Rect::from_center_size(rect.center(), size));
        return;
    }
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        egui::FontId::new(15.0, egui::FontFamily::Name("phosphor".into())),
        color,
    );
    let origin = rect.center() - galley.mesh_bounds.center().to_vec2();
    ui.painter().galley(origin, galley, color);
}
/// Persistent message selection follows rendered crossings, not a bounding box.
pub(super) fn selected_message(ui: &egui::Ui, scene: &Scene, message: &laml::authoring::Region) {
    let color = appearance::color(ui.ctx(), appearance::ACCENT);
    let paths = if message.pick_paths.is_empty() {
        std::slice::from_ref(&message.path)
    } else {
        message.pick_paths.as_slice()
    };
    for path in paths {
        ui.painter().add(egui::Shape::line(
            path.iter().map(|p| scene.point(p.0, p.1)).collect(),
            egui::Stroke::new(6.0, color.gamma_multiply(0.28)),
        ));
    }
    for area in &scene.areas {
        if matches!(&area.hit, Hit::Endpoint { message: r, .. }
            if r.line == message.line && r.id == message.id)
        {
            ui.painter().circle_filled(area.rect.center(), 4.0, color);
            ui.painter().circle_stroke(
                area.rect.center(),
                8.0,
                egui::Stroke::new(2.0, color.gamma_multiply(0.3)),
            );
        }
    }
}
/// Show an activation's complete control group, including when crossing to its plus.
pub(super) fn control_visible(
    scene: &Scene,
    area: &scene::Area,
    pointer: egui::Pos2,
    hit: Option<&Hit>,
) -> bool {
    let control_bar = match &area.hit {
        Hit::Origin(p) | Hit::Insert(p) => scene.bar_for(p),
        _ => None,
    };
    if let Some(bar) = control_bar {
        let hovered = scene
            .bars
            .iter()
            .filter(|b| b.rect.contains(pointer))
            .max_by(|a, b| a.region.x.total_cmp(&b.region.x))
            .or_else(|| {
                scene
                    .areas
                    .iter()
                    .filter(|a| a.rect.expand(15.0).contains(pointer))
                    .filter_map(|a| match &a.hit {
                        Hit::Origin(p) | Hit::Insert(p) => scene.bar_for(p),
                        _ => None,
                    })
                    .min_by(|a, b| {
                        a.rect
                            .distance_to_pos(pointer)
                            .total_cmp(&b.rect.distance_to_pos(pointer))
                    })
            });
        if hovered.is_some_and(|b| std::ptr::eq(b, bar)) {
            return true;
        }
    }
    area.rect.expand(15.0).contains(pointer)
        || match &area.hit {
            Hit::ParticipantPlus { neighbor, .. } => scene
                .heads
                .iter()
                .any(|(r, s)| r.id == *neighbor && s.expand(28.0).contains(pointer)),
            Hit::Endpoint { message, .. } => {
                matches!(hit,Some(Hit::Event(r)) if r.line==message.line)
                    || matches!(hit,Some(Hit::Endpoint{message:r,..}) if r.line==message.line)
            }
            _ => false,
        }
}
pub(super) fn hover(ui: &egui::Ui, scene: &Scene, pointer: egui::Pos2, hit: Option<&Hit>) {
    let color = appearance::color(ui.ctx(), appearance::ACCENT);
    for area in &scene.areas {
        let visible = control_visible(scene, area, pointer, hit);
        if !visible {
            continue;
        }
        let hot = area.rect.contains(pointer);
        match &area.hit {
            Hit::ParticipantPlus { .. } | Hit::Insert(_) => icon(ui, area.rect.center(), PLUS, hot),
            Hit::Origin(_) => {
                ui.painter().circle_filled(
                    area.rect.center(),
                    if hot { 9.0 } else { 8.0 },
                    color.gamma_multiply(if hot { 1.0 } else { 0.75 }),
                );
                ui.painter().circle_stroke(
                    area.rect.center(),
                    if hot { 12.0 } else { 10.0 },
                    egui::Stroke::new(2.0, color.gamma_multiply(0.25)),
                );
            }
            Hit::Resize(bar, lower) if hot => {
                let y = if *lower {
                    bar.rect.bottom()
                } else {
                    bar.rect.top()
                };
                ui.painter()
                    .hline(bar.rect.x_range(), y, egui::Stroke::new(4.0, color));
            }
            Hit::Endpoint { .. } => {
                ui.painter()
                    .circle_filled(area.rect.center(), if hot { 6.0 } else { 4.0 }, color);
                ui.painter().circle_stroke(
                    area.rect.center(),
                    8.0,
                    egui::Stroke::new(2.0, color.gamma_multiply(0.3)),
                );
            }
            Hit::Destroy { .. } => {
                ui.painter().text(
                    area.rect.center(),
                    egui::Align2::CENTER_CENTER,
                    X,
                    egui::FontId::new(20.0, egui::FontFamily::Name("phosphor".into())),
                    color.gamma_multiply(if hot { 0.9 } else { 0.4 }),
                );
            }
            _ => {}
        }
    }
    if let Some(hit) = hit {
        ui.ctx().set_cursor_icon(match hit {
            Hit::Resize(..) => egui::CursorIcon::ResizeVertical,
            Hit::Origin(_) => egui::CursorIcon::Crosshair,
            Hit::Insert(_) | Hit::ParticipantPlus { .. } | Hit::Destroy { .. } => {
                egui::CursorIcon::PointingHand
            }
            _ => egui::CursorIcon::Grab,
        });
    }
}
pub(super) fn gesture(ui: &egui::Ui, scene: &Scene, g: &gestures::Gesture, pointer: egui::Pos2) {
    let color = appearance::color(ui.ctx(), appearance::ACCENT);
    let vertical = matches!(g.hit, Hit::Resize(..) | Hit::Event(_) | Hit::Destruction(_));
    for (i, d) in g.destinations.iter().enumerate() {
        let hot = g.hot == Some(i);
        let stroke = egui::Stroke::new(
            if hot { 2.5 } else { 1.0 },
            color.gamma_multiply(if hot { 1.0 } else { 0.2 }),
        );
        if vertical {
            ui.painter().hline(ui.clip_rect().x_range(), d.at.y, stroke);
        } else if matches!(g.hit, Hit::Participant(_)) {
            ui.painter().vline(d.at.x, ui.clip_rect().y_range(), stroke);
        } else {
            // These are the complete legal destinations, not faint hover hints.
            let ink = color.gamma_multiply(if hot { 1.0 } else { 0.85 });
            let fill = appearance::color(ui.ctx(), appearance::PANEL);
            let radius = if hot { 8.0 } else { 6.0 };
            if hot {
                ui.painter()
                    .circle_filled(d.at, 13.0, color.gamma_multiply(0.18));
            }
            if let Some(edge) = d.hit_rect {
                ui.painter().hline(
                    edge.x_range(),
                    edge.top(),
                    egui::Stroke::new(if hot { 3.0 } else { 1.5 }, ink),
                );
            }
            if d.dock.as_ref().is_some_and(|dock| dock.boundary.is_some()) {
                let square = egui::Rect::from_center_size(d.at, egui::Vec2::splat(radius * 2.0));
                ui.painter().rect_filled(square, 2.0, fill);
                ui.painter().rect_stroke(
                    square,
                    2.0,
                    egui::Stroke::new(2.0, ink),
                    egui::StrokeKind::Inside,
                );
            } else {
                ui.painter().circle_filled(d.at, radius, fill);
                ui.painter()
                    .circle_stroke(d.at, radius, egui::Stroke::new(2.0, ink));
            }
            ui.painter()
                .circle_filled(d.at, if hot { 3.0 } else { 2.0 }, ink);
            if hot {
                ui.painter().hline(
                    ui.clip_rect().x_range(),
                    d.at.y,
                    egui::Stroke::new(1.0, color.gamma_multiply(0.4)),
                );
            }
        }
    }
    let at = g.hot.map_or(pointer, |i| g.destinations[i].at);
    match &g.hit {
        Hit::Resize(bar, lower) => {
            let mut rect = bar.rect;
            if *lower {
                rect.max.y = at.y.max(rect.min.y + 1.0);
            } else {
                rect.min.y = at.y.min(rect.max.y - 1.0);
            }
            ui.painter().rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(2.0, color),
                egui::StrokeKind::Outside,
            );
        }
        Hit::Event(r) | Hit::Destruction(r) if r.kind != "message" => {
            let original = scene.shape(r);
            ui.painter().rect_stroke(
                original.translate(egui::vec2(0.0, at.y - original.center().y)),
                2.0,
                egui::Stroke::new(2.0, color),
                egui::StrokeKind::Outside,
            );
        }
        Hit::Participant(r) => {
            let original = scene.shape(r);
            ui.painter().rect_stroke(
                original.translate(egui::vec2(pointer.x - original.center().x, 0.0)),
                2.0,
                egui::Stroke::new(2.0, color),
                egui::StrokeKind::Outside,
            );
        }
        _ => {}
    }
}

#[cfg(test)]
mod icon_tests {
    use super::*;
    #[test]
    fn phosphor_plus_uses_centered_svg_not_a_baseline_snapped_font_glyph() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let ctx = egui::Context::default();
            appearance::install(&ctx);
            egui_extras::install_image_loaders(&ctx);
            ctx.set_pixels_per_point(scale);
            let center = egui::pos2(80.37, 80.19);
            let expected = egui::Rect::from_center_size(center, egui::Vec2::splat(20.0))
                .round_to_pixels(scale)
                .center();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(200.0, 200.0),
                    )),
                    ..Default::default()
                },
                |ui| icon(ui, center, PLUS, false),
            );
            output.textures_delta.clear();
            assert!(
                !output
                    .shapes
                    .iter()
                    .any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text==PLUS))
            );
            let mesh = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(rect) if rect.brush.is_some() => Some(rect),
                    _ => None,
                })
                .expect("Local Phosphor SVG image mesh");
            assert!(mesh.rect.center().distance(expected) < 0.001);
        }
    }
}
