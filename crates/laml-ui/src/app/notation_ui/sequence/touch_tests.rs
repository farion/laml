use super::*;
fn tap(ctx: &egui::Context, e: &mut Editor, image: egui::Rect, pos: egui::Pos2) {
    tests::frame(ctx, e, image, vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        tests::frame(
            ctx,
            e,
            image,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}
#[test]
fn touch_reveals_controls_until_tapping_free_canvas() {
    let (ctx, mut e, image) = tests::editor();
    e.responsive.touch = true;
    let scene = Scene::new(e.preview.as_ref().unwrap(), image);
    tap(&ctx, &mut e, image, scene.heads[0].1.center());
    assert!(e.notation_ui.sequence.touch_anchor.is_some());
    tests::frame(&ctx, &mut e, image, vec![egui::Event::PointerGone]);
    assert!(e.notation_ui.sequence.touch_anchor.is_some());
    tap(&ctx, &mut e, image, egui::pos2(1100.0, 800.0));
    assert!(e.notation_ui.sequence.touch_anchor.is_none());
    assert_eq!(e.document.revision(), 0);
}
#[test]
fn hidden_insertion_controls_cannot_mutate_on_first_touch() {
    let (ctx, mut e, image) = tests::editor();
    e.responsive.touch = true;
    let scene = Scene::new(e.preview.as_ref().unwrap(), image);
    let at = scene
        .areas
        .iter()
        .find(|a| matches!(a.hit, Hit::ParticipantPlus { .. }))
        .unwrap()
        .rect
        .center();
    tap(&ctx, &mut e, image, at);
    assert!(e.pending_command.is_none());
    assert_eq!(e.document.revision(), 0);
}
