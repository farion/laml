//! Centered modal dialogs, with one shared backdrop and dismissal contract.
use super::*;

/// Shared chrome for current and future dialogs, including content/action padding.
pub(super) fn frame(ctx: &egui::Context) -> egui::Frame {
    egui::Frame::popup(&ctx.style_of(ctx.theme()))
        .fill(appearance::color(ctx, appearance::PANEL))
        .inner_margin(egui::Margin::same(16))
}

pub(super) fn show(
    ctx: &egui::Context,
    id: impl std::hash::Hash + std::fmt::Debug,
    title: &str,
    width: f32,
    content: impl FnOnce(&mut egui::Ui),
) -> bool {
    let id = egui::Id::unique(id);
    let response = egui::Modal::new(id)
        .area(egui::Modal::default_area(id).fade_in(false))
        .frame(frame(ctx))
        .backdrop_color(egui::Color32::from_black_alpha(170))
        .show(ctx, |ui| {
            ui.set_width(width.min((ctx.content_rect().width() - 64.0).max(120.0)));
            // Do not let the area's cached/default height trap a nested scroll
            // viewport and push confirmation controls outside the visible body.
            ui.set_max_height((ctx.content_rect().height() - 64.0).max(120.0));
            ui.spacing_mut().text_edit_width = ui.available_width();
            ui.spacing_mut().combo_width = ui.available_width();
            ui.horizontal(|ui| {
                ui.heading(title);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if workspace::icon(ui, egui_phosphor::regular::X, "Close dialog").clicked() {
                        ui.close();
                    }
                });
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height((ctx.content_rect().height() - 140.0).max(80.0))
                .show(ui, content);
        });
    response.should_close()
}

pub(super) fn blocked(ctx: &egui::Context) -> bool {
    ctx.memory(|memory| memory.top_modal_layer().is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modal_is_centered_opaque_and_blocks_the_background() {
        let ctx = egui::Context::default();
        appearance::install(&ctx);
        let mut clicks = 0;
        let mut frame = |events: Vec<egui::Event>| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    if ui.button("Background").clicked() {
                        clicks += 1;
                    }
                    show(ui.ctx(), "test-modal", "Settings", 360.0, |ui| {
                        ui.label("Modal content");
                        ui.allocate_space(egui::vec2(330.0, 120.0));
                    });
                },
            );
            out.textures_delta.clear();
            out
        };
        frame(vec![]);
        let out = frame(vec![]);
        assert!(blocked(&ctx));
        fn collect(shape: &egui::Shape, rects: &mut Vec<egui::epaint::RectShape>) {
            match shape {
                egui::Shape::Rect(r) => rects.push(r.clone()),
                egui::Shape::Vec(shapes) => {
                    for s in shapes {
                        collect(s, rects);
                    }
                }
                _ => {}
            }
        }
        let mut rects = vec![];
        for s in &out.shapes {
            collect(&s.shape, &mut rects);
        }
        let rect = rects
            .iter()
            .find(|r| {
                r.fill == appearance::PANEL && r.rect.width() > 300.0 && r.rect.width() < 500.0
            })
            .unwrap()
            .rect;
        assert!((rect.center() - egui::pos2(500.0, 350.0)).length() < 2.0);
        assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Rect(r) if r.fill==egui::Color32::from_black_alpha(170)&&r.rect.width()>=1000.0)));
        let point = egui::pos2(30.0, 20.0);
        for pressed in [true, false] {
            frame(vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    pressed,
                    button: egui::PointerButton::Primary,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
        }
        assert_eq!(clicks, 0);
    }

    #[test]
    fn close_uses_titlebar_icon_hover_chrome_and_dismisses_on_click() {
        let ctx = egui::Context::default();
        appearance::install(&ctx);
        let frame = |events| {
            let mut closed = false;
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    closed = show(ui.ctx(), "icon-close-test", "Settings", 360.0, |ui| {
                        ui.label("Content");
                    });
                },
            );
            out.textures_delta.clear();
            (out, closed)
        };
        frame(vec![]);
        let (out, _) = frame(vec![]);
        let icon = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == egui_phosphor::regular::X => Some(t),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            icon.galley.job.sections[0].format.font_id.family,
            egui::FontFamily::Name("phosphor".into())
        );
        let point = icon.pos + icon.galley.size() * 0.5;
        fn has_chrome(shape: &egui::Shape, point: egui::Pos2) -> bool {
            match shape {
                egui::Shape::Rect(r) => {
                    (r.rect.center() - point).length() < 1.0
                        && (32.0..=48.0).contains(&r.rect.width())
                        && (r.rect.height() - 32.0).abs() < 1.0
                        && (r.fill.a() > 0 || (r.stroke.width > 0.0 && r.stroke.color.a() > 0))
                }
                egui::Shape::Vec(shapes) => shapes.iter().any(|s| has_chrome(s, point)),
                _ => false,
            }
        }
        assert!(!out.shapes.iter().any(|s| has_chrome(&s.shape, point)));
        frame(vec![egui::Event::PointerMoved(point)]);
        let (hover, _) = frame(vec![]);
        assert!(hover.shapes.iter().any(|s| has_chrome(&s.shape, point)));
        for pressed in [true, false] {
            let (_, closed) = frame(vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            }]);
            assert_eq!(closed, !pressed);
        }
    }
}
