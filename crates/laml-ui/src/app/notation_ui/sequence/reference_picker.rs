//! Persistent reference affordances and an accent-backed mandatory-action banner.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Endpoint,
    Object,
    Lifeline,
}
struct Target {
    value: String,
    rect: egui::Rect,
    kind: Kind,
}
fn targets(scene: &Scene, key: &str, scope: Option<&str>, allowed: &[String]) -> Vec<Target> {
    let mut out = Vec::new();
    let occurrence = matches!(key, "occurrence" | "first" | "second");
    let kind = match key {
        "frame" => "frame",
        "note" => "note",
        "continuation" => "continuation",
        _ => "participant",
    };
    for r in &scene.regions {
        if r.id.rsplit_once('.').map(|(s, _)| s) != scope {
            continue;
        }
        if occurrence {
            if r.kind != "message" {
                continue;
            }
            if let Some((from, to)) = r.path.first().zip(r.path.last()) {
                for (p, end) in [(from, "send"), (to, "receive")] {
                    let value = format!("@{}.{end}", r.line);
                    if allowed.contains(&value) {
                        out.push(Target {
                            value,
                            rect: egui::Rect::from_center_size(
                                scene.point(p.0, p.1),
                                egui::Vec2::splat(18.0),
                            ),
                            kind: Kind::Endpoint,
                        });
                    }
                }
            }
        } else if r.kind == kind || kind == "participant" && r.kind == "lifeline" {
            let value = if r
                .id
                .rsplit('.')
                .next()
                .is_some_and(|id| id.starts_with("line:"))
            {
                format!("@{}.frame", r.line)
            } else {
                r.id.clone()
            };
            if allowed.contains(&value) {
                out.push(Target {
                    value,
                    rect: scene.shape(r),
                    kind: if r.kind == "lifeline" {
                        Kind::Lifeline
                    } else {
                        Kind::Object
                    },
                });
            }
        }
    }
    out
}
fn hovered(targets: &[Target], at: egui::Pos2) -> Option<usize> {
    targets
        .iter()
        .enumerate()
        .filter(|(_, t)| t.rect.expand(8.0).contains(at))
        .min_by(|(_, a), (_, b)| {
            if a.kind == Kind::Endpoint && b.kind == Kind::Endpoint {
                a.rect
                    .center()
                    .distance(at)
                    .total_cmp(&b.rect.center().distance(at))
            } else {
                (a.rect.width().max(1.0) * a.rect.height().max(1.0))
                    .total_cmp(&(b.rect.width().max(1.0) * b.rect.height().max(1.0)))
            }
        })
        .map(|(i, _)| i)
}
fn paint(ui: &egui::Ui, targets: &[Target], hot: Option<usize>) {
    let accent = appearance::color(ui.ctx(), appearance::ACCENT);
    let panel = appearance::color(ui.ctx(), appearance::PANEL);
    for (index, target) in targets.iter().enumerate() {
        let active = hot == Some(index);
        if target.kind == Kind::Endpoint {
            let at = target.rect.center();
            if active {
                ui.painter()
                    .circle_filled(at, 14.0, accent.linear_multiply(0.25));
            }
            ui.painter().circle_filled(
                at,
                if active { 9.0 } else { 7.0 },
                if active { accent } else { panel },
            );
            ui.painter().circle_stroke(
                at,
                if active { 9.0 } else { 7.0 },
                egui::Stroke::new(2.0, if active { egui::Color32::WHITE } else { accent }),
            );
            if !active {
                ui.painter().circle_filled(at, 3.0, accent);
            }
        } else {
            let rect = target.rect.expand(if active { 4.0 } else { 2.0 });
            ui.painter().rect_filled(
                rect,
                3.0,
                accent.linear_multiply(if active { 0.25 } else { 0.1 }),
            );
            ui.painter().rect_stroke(
                rect,
                3.0,
                egui::Stroke::new(if active { 3.0 } else { 1.5 }, accent),
                egui::StrokeKind::Outside,
            );
        }
    }
}
pub(super) fn show(
    ui: &egui::Ui,
    scene: &Scene,
    key: &str,
    scope: Option<&str>,
    allowed: &[String],
    pointer: Option<egui::Pos2>,
) -> Option<String> {
    let targets = targets(scene, key, scope, allowed);
    let hot = pointer.and_then(|p| hovered(&targets, p));
    paint(ui, &targets, hot);
    if hot.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    hot.map(|i| targets[i].value.clone())
}
pub(super) fn hint(ui: &egui::Ui, text: String) {
    let color = appearance::color(ui.ctx(), appearance::ACCENT);
    let text = ui.painter().layout(
        text,
        egui::FontId::new(16.0, egui::FontFamily::Name("bold".into())),
        egui::Color32::WHITE,
        (ui.clip_rect().width() - 64.0).max(32.0),
    );
    let size = text.size() + egui::Vec2::splat(32.0);
    let rect = egui::Rect::from_min_size(
        egui::pos2(
            ui.clip_rect().center().x - size.x / 2.0,
            ui.clip_rect().top() + 12.0,
        ),
        size,
    );
    ui.painter().rect_filled(rect, 6.0, color);
    ui.painter().galley(
        rect.min + egui::Vec2::splat(16.0),
        text,
        egui::Color32::WHITE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scene(zoom: f32) -> Scene {
        let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n first: A -> B\n second: B -> A\n}\n";
        let p =
            laml_ui::preview::build(source, std::path::Path::new("reference-points.laml")).unwrap();
        Scene::new(
            &p,
            egui::Rect::from_min_size(
                egui::pos2(40.0, 80.0),
                egui::vec2(
                    (p.bounds.2 - p.bounds.0) as f32 * zoom,
                    (p.bounds.3 - p.bounds.1) as f32 * zoom,
                ),
            ),
        )
    }
    fn frame(ctx: &egui::Context, draw: impl FnMut(&mut egui::Ui)) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 900.0),
                )),
                ..Default::default()
            },
            draw,
        );
        output.textures_delta.clear();
        output
    }
    #[test]
    fn every_valid_endpoint_is_visible_without_hover_and_hover_is_stronger() {
        for zoom in [0.5, 1.0, 2.0] {
            let scene = scene(zoom);
            let allowed = vec!["@7.send".into(), "@8.receive".into()];
            let ctx = egui::Context::default();
            appearance::install(&ctx);
            let candidates = targets(&scene, "first", None, &allowed);
            assert_eq!(candidates.len(), 2);
            let output = frame(&ctx, |ui| {
                assert!(show(ui, &scene, "first", None, &allowed, None).is_none());
            });
            let rings = output.shapes.iter().filter(|s| matches!(&s.shape, egui::Shape::Circle(c) if c.radius == 7.0 && c.stroke.width == 2.0)).count();
            assert_eq!(
                rings, 2,
                "All valid candidates must be painted before hover"
            );
            let hot = candidates[0].rect.center();
            let output = frame(&ctx, |ui| {
                assert_eq!(
                    show(ui, &scene, "first", None, &allowed, Some(hot)),
                    Some(candidates[0].value.clone())
                );
            });
            assert!(
                output
                    .shapes
                    .iter()
                    .any(|s| matches!(&s.shape, egui::Shape::Circle(c) if c.radius == 14.0)),
                "Hovered candidate has an additional halo"
            );
            assert_eq!(
                targets(&scene, "first", Some("different-scope"), &allowed).len(),
                0
            );
        }
    }
    #[test]
    fn participant_targets_remain_visible_and_invalid_participants_are_not_pickable() {
        let scene = scene(1.0);
        let allowed = vec!["B".into()];
        let candidates = targets(&scene, "participant", None, &allowed);
        assert!(candidates.iter().any(|t| t.kind == Kind::Object));
        assert!(candidates.iter().any(|t| t.kind == Kind::Lifeline));
        assert!(candidates.iter().all(|t| t.value == "B"));
        let a = scene
            .heads
            .iter()
            .find(|(r, _)| r.id == "A")
            .unwrap()
            .1
            .center();
        assert!(hovered(&candidates, a).is_none());
        let ctx = egui::Context::default();
        appearance::install(&ctx);
        let output = frame(&ctx, |ui| {
            paint(ui, &candidates, None);
        });
        assert_eq!(
            output
                .shapes
                .iter()
                .filter(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.stroke.width == 1.5))
                .count(),
            candidates.len()
        );
    }
    #[test]
    fn mandatory_hint_has_solid_accent_background_and_readable_bold_text_in_both_themes() {
        for dark in [false, true] {
            let ctx = egui::Context::default();
            appearance::install(&ctx);
            if !dark {
                ctx.set_visuals(egui::Visuals::light());
            }
            let output = frame(&ctx, |ui| {
                ui.set_clip_rect(egui::Rect::from_min_size(
                    egui::pos2(100.0, 20.0),
                    egui::vec2(220.0, 500.0),
                ));
                hint(
                    ui,
                    "Duration constraint — Select first reference · Esc: Cancel".into(),
                );
            });
            let accent = appearance::color(&ctx, appearance::ACCENT);
            let background = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r) if r.fill == accent => Some(r.rect),
                    _ => None,
                })
                .expect("Solid accent banner");
            assert!(background.left() >= 100.0 && background.right() <= 320.0);
            let text = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) => Some(t),
                    _ => None,
                })
                .unwrap();
            assert!(
                text.galley.job.sections.iter().all(
                    |s| s.format.color == egui::Color32::WHITE && s.format.font_id.size == 16.0
                )
            );
        }
    }
}
