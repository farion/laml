use super::*;
#[test]
fn resize_and_message_origins_are_disjoint_and_heads_have_both_pluses() {
    let s = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n B+\n A -> B\n B-\n}\n";
    let p = laml_ui::preview::build(s, std::path::Path::new("areas.laml")).unwrap();
    let scene = Scene::new(
        &p,
        egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(
                (p.bounds.2 - p.bounds.0) as f32,
                (p.bounds.3 - p.bounds.1) as f32,
            ),
        ),
    );
    assert_eq!(
        scene
            .areas
            .iter()
            .filter(|a| matches!(a.hit, Hit::ParticipantPlus { .. }))
            .count(),
        4
    );
    for edge in scene
        .areas
        .iter()
        .filter(|a| matches!(a.hit, Hit::Resize(..)))
    {
        for origin in scene
            .areas
            .iter()
            .filter(|a| matches!(a.hit, Hit::Origin(_)))
        {
            assert!(
                !edge.rect.intersects(origin.rect),
                "{:?} overlaps {:?}",
                edge.rect,
                origin.rect
            );
        }
    }
}

fn rendered_scene(source: &str) -> Scene {
    let preview = laml_ui::preview::build(source, std::path::Path::new("picking.laml")).unwrap();
    Scene::new(
        &preview,
        egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(
                (preview.bounds.2 - preview.bounds.0) as f32,
                (preview.bounds.3 - preview.bounds.1) as f32,
            ),
        ),
    )
}
#[test]
fn fragment_and_activation_ends_do_not_create_selectable_empty_rows() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n A -> B+\n B-\n opt \"Foo=bar\" {\n A -> B\n }\n Note1: note \"Hier ist cool\"\n B -> A\n}\nlayout {\n Note1 over B\n}\n";
    let scene = rendered_scene(source);
    assert!(!scene.regions.iter().any(|r| r.kind == "event"));
    let frame = scene.regions.iter().find(|r| r.kind == "frame").unwrap();
    let empty = scene.point(frame.x + 10.0, frame.y + frame.height + 24.0);
    assert!(
        scene.hit(empty).is_none(),
        "The screenshot's empty row below opt is not an object: {:?}",
        scene.hit(empty)
    );
    let header = scene.point(frame.x + frame.width / 2.0, frame.y + 10.0);
    assert!(matches!(scene.hit(header), Some(Hit::Event(r)) if r.kind == "frame"));
    let activation = scene
        .regions
        .iter()
        .find(|r| r.kind == "execution")
        .unwrap();
    let beside_end = scene.point(frame.x + 10.0, activation.y + activation.height);
    assert!(scene.hit(beside_end).is_none());
    let note = scene.regions.iter().find(|r| r.kind == "note").unwrap();
    let beside_note = scene.point(frame.x + 10.0, note.y + note.height / 2.0);
    assert!(scene.hit(beside_note).is_none());
    assert!(
        matches!(scene.hit(scene.shape(note).center()), Some(Hit::Event(r)) if r.kind == "note")
    );
    assert!(
        !scene.docks.is_empty(),
        "Removing phantom selections must retain insertion positions"
    );
}
#[test]
fn visible_reference_and_ordering_publish_their_actual_pick_geometry() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n first: A -> B\n second: B -> A\n order[first.send,second.receive]\n ref \"Other interaction\"\n}\n";
    let scene = rendered_scene(source);
    let reference = scene
        .regions
        .iter()
        .find(|r| r.kind == "reference")
        .unwrap();
    let at = scene.point(reference.x + reference.width / 2.0, reference.y + 10.0);
    assert!(matches!(scene.hit(at), Some(Hit::Event(r)) if r.kind == "reference"));
    let order = scene.regions.iter().find(|r| r.kind == "ordering").unwrap();
    assert_eq!(order.path.len(), 4);
    let at = scene.point(
        order.path[1].0,
        order.path[1].1 * 0.75 + order.path[2].1 * 0.25,
    );
    assert!(matches!(scene.hit(at), Some(Hit::Event(r)) if r.kind == "ordering"));
    assert!(!scene.regions.iter().any(|r| r.kind == "event"));
}

#[test]
fn coregion_brackets_are_selectable_without_claiming_their_empty_span() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n first: A ->> B\n second: B ->> A\n coregion A {\n first.send\n second.receive\n }\n}\n";
    assert_coregion_picking(source, 9);
}
#[test]
fn decomposed_coregion_brackets_keep_their_real_source_line() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n Root\n decompose Root \"Inside\" {\n A\n B\n first: A ->> B\n second: B ->> A\n coregion A {\n first.send\n second.receive\n }\n }\n}\n";
    assert_coregion_picking(source, 11);
}
fn assert_coregion_picking(source: &str, line: usize) {
    let preview = laml_ui::preview::build(source, std::path::Path::new("coregion.laml")).unwrap();
    for zoom in [0.5, 1.0, 2.0] {
        let scene = Scene::new(
            &preview,
            egui::Rect::from_min_size(
                egui::pos2(37.0, 53.0),
                egui::vec2(
                    ((preview.bounds.2 - preview.bounds.0) * zoom) as f32,
                    ((preview.bounds.3 - preview.bounds.1) * zoom) as f32,
                ),
            ),
        );
        let brackets = scene
            .regions
            .iter()
            .filter(|r| r.kind == "coregion")
            .collect::<Vec<_>>();
        assert_eq!(brackets.len(), 2);
        for bracket in &brackets {
            assert_eq!(bracket.line, line);
            assert_eq!(bracket.path.len(), 4);
            for segment in bracket.path.windows(2) {
                let at = scene.point(
                    (segment[0].0 + segment[1].0) / 2.0,
                    (segment[0].1 + segment[1].1) / 2.0,
                );
                assert!(
                    matches!(scene.hit(at), Some(Hit::Event(r)) if r.kind == "coregion" && r.line == line),
                    "Coregion bracket at {at:?}, zoom {zoom}: {:?}",
                    scene.hit(at)
                );
            }
        }
        let x = brackets[0].path[0].0 - 4.0;
        let y = (brackets[0].y + brackets[1].y) / 2.0;
        assert!(
            !matches!(scene.hit(scene.point(x, y)), Some(Hit::Event(r)) if r.kind == "coregion"),
            "Empty space between brackets must not become a coregion hit area"
        );
    }
}
