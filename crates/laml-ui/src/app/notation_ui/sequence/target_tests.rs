//! Target coverage: a valid drag start must not silently lose its destinations.
use super::*;
fn scene(source: &str) -> Scene {
    let p = laml_ui::preview::build(source, std::path::Path::new("target-coverage.laml")).unwrap();
    let image = egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(
            (p.bounds.2 - p.bounds.0) as f32,
            (p.bounds.3 - p.bounds.1) as f32,
        ),
    );
    Scene::new(&p, image)
}
const NAMED: &str = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n C\n work: B+ : \"Keep this activation label\"\n A ->> B: first\n B ->> C: second\n B-\n}\n";
#[test]
fn named_activation_origins_keep_every_legal_gap_target() {
    let scene = scene(NAMED);
    for lower in [false, true] {
        let origin = scene
            .areas
            .iter()
            .find_map(|a| match &a.hit {
                Hit::Origin(p)
                    if p.dock.owner == "B"
                        && if lower {
                            matches!(p.dock.boundary, Some(Boundary::End { .. }))
                        } else {
                            matches!(p.dock.boundary, Some(Boundary::Start(_)))
                        } =>
                {
                    Some(p)
                }
                _ => None,
            })
            .unwrap();
        let destinations =
            proposals::destinations(NAMED, &scene, &Hit::Origin(origin.clone()), "->");
        assert!(!destinations.is_empty());
        for gap in scene.docks.iter().filter(|p| p.dock.boundary.is_none()) {
            assert!(
                destinations
                    .iter()
                    .any(|d| d.dock.as_ref() == Some(&gap.dock) && d.at.distance(gap.at) < 0.1),
                "Missing {:?} from {} edge",
                gap.dock,
                if lower { "lower" } else { "upper" }
            );
        }
        for d in &destinations {
            let candidate = edit::apply(NAMED, &d.proposal.edits).unwrap();
            assert!(candidate.contains("work: B+ : \"Keep this activation label\""));
            let doc = laml::sequence::parse(&candidate).unwrap();
            assert_eq!(laml::sequence::activation_layout::executions(&doc).len(), 1);
        }
    }
}
#[test]
fn activation_boundaries_and_inner_gaps_have_one_centered_target() {
    let scene = scene(NAMED);
    let bar = &scene.bars[0];
    for boundary in [
        Boundary::Start(bar.region.line),
        Boundary::End {
            start: bar.region.line,
            end: bar.region.end_line,
        },
    ] {
        let points = scene
            .docks
            .iter()
            .filter(|p| p.dock.owner == bar.owner && p.dock.boundary.as_ref() == Some(&boundary))
            .collect::<Vec<_>>();
        assert_eq!(points.len(), 1);
        assert!((points[0].at.x - bar.rect.center().x).abs() < 0.01);
    }
    let gaps = scene
        .docks
        .iter()
        .filter(|p| {
            p.dock.owner == bar.owner
                && p.dock.boundary.is_none()
                && p.at.y > bar.rect.top()
                && p.at.y < bar.rect.bottom()
        })
        .collect::<Vec<_>>();
    assert!(!gaps.is_empty());
    for gap in gaps {
        assert!((gap.at.x - bar.rect.center().x).abs() < 0.01);
        assert_eq!(
            scene
                .docks
                .iter()
                .filter(|p| p.dock == gap.dock && p.at.distance(gap.at) < 0.01)
                .count(),
            1
        );
        assert_eq!(
            scene
                .areas
                .iter()
                .filter(|a| matches!(&a.hit,Hit::Insert(p) if p.dock==gap.dock))
                .count(),
            1
        );
        let destination = proposals::message_destination(
            &scene,
            gap,
            Proposal {
                edits: vec![],
                line: None,
                label: "Test",
            },
        );
        let area = destination
            .hit_rect
            .expect("Activation width is the shared drop zone");
        assert!((area.left() - bar.rect.left()).abs() < 0.01);
        assert!((area.right() - bar.rect.right()).abs() < 0.01);
    }
}
#[test]
fn unbound_activation_top_and_bottom_are_actual_message_destinations() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n B+\n A ->> B\n B-\n}\n";
    let scene = scene(source);
    let origin = scene
        .docks
        .iter()
        .find(|p| p.dock.owner == "A" && p.dock.boundary.is_none())
        .unwrap();
    let destinations = proposals::destinations(source, &scene, &Hit::Origin(origin.clone()), "->");
    for lower in [false, true] {
        assert!(
            destinations
                .iter()
                .any(|d| d.dock.as_ref().is_some_and(|p| p.owner == "B"
                    && if lower {
                        matches!(p.boundary, Some(Boundary::End { .. }))
                    } else {
                        matches!(p.boundary, Some(Boundary::Start(_)))
                    }))
        );
    }
    assert!(
        destinations
            .iter()
            .filter(|d| d.hit_rect.is_some())
            .all(|d| d.hit_rect.unwrap().width() > 0.0)
    );
}

#[test]
fn named_labelled_and_occupied_boundaries_offer_real_compiled_targets() {
    for source in [
        NAMED,
        "settings {\n type: sequence\n}\nsemantics {\n A\n B\n C\n call: A -> B+: request\n C ->> A\n B-\n}\n",
    ] {
        let scene = scene(source);
        let origin = scene
            .docks
            .iter()
            .find(|p| p.dock.owner == "C" && p.dock.boundary.is_none())
            .unwrap();
        let targets = proposals::destinations(source, &scene, &Hit::Origin(origin.clone()), "->");
        for lower in [false, true] {
            let found = targets
                .iter()
                .filter(|d| {
                    d.dock.as_ref().is_some_and(|p| {
                        p.owner == "B"
                            && if lower {
                                matches!(p.boundary, Some(Boundary::End { .. }))
                            } else {
                                matches!(p.boundary, Some(Boundary::Start(_)))
                            }
                    })
                })
                .collect::<Vec<_>>();
            assert_eq!(
                found.len(),
                1,
                "Expected one centered target for {} boundary",
                if lower { "lower" } else { "upper" }
            );
            for d in found {
                let ghost = d.preview.as_ref().expect("Compiled message ghost");
                assert_eq!(ghost.message.from, "C");
                assert_eq!(ghost.message.to, "B");
                let candidate = edit::apply(source, &d.proposal.edits).unwrap();
                let preview =
                    laml_ui::preview::build(&candidate, std::path::Path::new("actual-dock.laml"))
                        .unwrap();
                let actual = preview
                    .regions
                    .iter()
                    .find(|r| r.kind == "sequence-receive" && Some(r.line) == d.proposal.line)
                    .unwrap();
                assert_eq!(ghost.receive, (actual.x, actual.y));
                if source == NAMED {
                    assert!(candidate.contains("work: B+ : \"Keep this activation label\""));
                } else {
                    assert!(candidate.contains("call: A -> B+: request"));
                }
            }
        }
    }
}
#[test]
fn self_message_targets_have_a_real_noncollapsed_ghost() {
    let scene = scene(NAMED);
    let origin = scene
        .docks
        .iter()
        .find(|p| p.dock.owner == "B" && p.dock.boundary.is_none())
        .unwrap();
    let targets = proposals::destinations(NAMED, &scene, &Hit::Origin(origin.clone()), "->");
    for lower in [false, true] {
        let d = targets
            .iter()
            .find(|d| {
                d.dock.as_ref().is_some_and(|p| {
                    p.owner == "B"
                        && if lower {
                            matches!(p.boundary, Some(Boundary::End { .. }))
                        } else {
                            matches!(p.boundary, Some(Boundary::Start(_)))
                        }
                })
            })
            .expect("Self-message activation target");
        let ghost = d.preview.as_ref().unwrap();
        assert_eq!(ghost.message.from, ghost.message.to);
        assert!(ghost.receive.1 > ghost.send.1);
    }
}

#[test]
fn new_message_ghost_snaps_to_current_targets_at_zoom_and_not_future_layout() {
    let preview = laml_ui::preview::build(NAMED, std::path::Path::new("drag-target.laml")).unwrap();
    let mut different_future_positions = 0;
    for zoom in [0.65, 1.8] {
        let image = egui::Rect::from_min_size(
            egui::pos2(113.0, 71.0),
            egui::vec2(
                (preview.bounds.2 - preview.bounds.0) as f32 * zoom,
                (preview.bounds.3 - preview.bounds.1) as f32 * zoom,
            ),
        );
        let scene = Scene::new(&preview, image);
        let origin = scene
            .docks
            .iter()
            .find(|p| p.dock.owner == "C" && p.dock.boundary.is_none())
            .unwrap()
            .clone();
        let hit = Hit::Origin(origin.clone());
        let destinations = proposals::destinations(NAMED, &scene, &hit, "->");
        assert!(!destinations.is_empty());
        let mut g = gestures::Gesture {
            hit,
            destinations,
            hot: None,
        };
        for i in 0..g.destinations.len() {
            g.hot = Some(i);
            let d = &g.destinations[i];
            let ghost =
                message_paint::ghost_geometry(&scene, &g, d.at + egui::vec2(4.0, 3.0), NAMED, "->")
                    .unwrap();
            assert!(
                scene.point(ghost.receive.0, ghost.receive.1).distance(d.at) < 0.01,
                "New arrow must meet visible target {:?}",
                d.dock
            );
            assert!((scene.point(ghost.send.0, ghost.send.1).x - origin.at.x).abs() < 0.01);
            if let Some(candidate) = &d.preview {
                assert_eq!(ghost.message.kind, candidate.message.kind);
                if scene
                    .point(candidate.receive.0, candidate.receive.1)
                    .distance(d.at)
                    > 1.0
                {
                    different_future_positions += 1;
                }
            }
        }
    }
    assert!(
        different_future_positions > 0,
        "Regression must include candidate layouts that move the final endpoint"
    );
}

#[test]
fn reconnect_ghost_keeps_the_dragged_send_or_receive_end_on_every_visible_target() {
    for source in [
        NAMED.to_string(),
        NAMED.replace(" B-\n", " loop: B ->> B: again\n B-\n"),
    ] {
        let preview =
            laml_ui::preview::build(&source, std::path::Path::new("reconnect-target.laml"))
                .unwrap();
        let image = egui::Rect::from_min_size(
            egui::pos2(57.0, 93.0),
            egui::vec2(
                (preview.bounds.2 - preview.bounds.0) as f32 * 1.4,
                (preview.bounds.3 - preview.bounds.1) as f32 * 1.4,
            ),
        );
        let scene = Scene::new(&preview, image);
        for region in scene
            .regions
            .iter()
            .filter(|r| r.kind == "message" && !r.path.is_empty())
        {
            for moving_source in [false, true] {
                let hit = Hit::Endpoint {
                    message: region.clone(),
                    source: moving_source,
                };
                let destinations = proposals::destinations(&source, &scene, &hit, "->");
                assert!(!destinations.is_empty());
                let mut g = gestures::Gesture {
                    hit,
                    destinations,
                    hot: None,
                };
                for i in 0..g.destinations.len() {
                    g.hot = Some(i);
                    let d = &g.destinations[i];
                    let ghost = message_paint::ghost_geometry(
                        &scene,
                        &g,
                        d.at + egui::vec2(3.0, 2.0),
                        &source,
                        "->",
                    )
                    .unwrap();
                    let end = if moving_source {
                        ghost.send
                    } else {
                        ghost.receive
                    };
                    assert!(
                        scene.point(end.0, end.1).distance(d.at) < 0.01,
                        "Dragged endpoint must meet {:?}",
                        d.dock
                    );
                    let path = laml::sequence::render::message_geometry::path(
                        ghost.send,
                        ghost.receive,
                        ghost.message.from == ghost.message.to,
                        ghost.diagonal,
                    );
                    assert_eq!(path.first().copied(), Some(ghost.send));
                    assert_eq!(path.last().copied(), Some(ghost.receive));
                }
                g.hot = None;
                let pointer = egui::pos2(180.0, 210.0);
                let ghost =
                    message_paint::ghost_geometry(&scene, &g, pointer, &source, "->").unwrap();
                let end = if moving_source {
                    ghost.send
                } else {
                    ghost.receive
                };
                assert!(scene.point(end.0, end.1).distance(pointer) < 0.01);
            }
        }
    }
}

#[test]
fn activation_origins_are_large_centered_and_all_controls_show_as_one_hover_group() {
    let scene = scene(NAMED);
    let bar = &scene.bars[0];
    let pointer = egui::pos2(bar.rect.left() + 1.0, bar.rect.center().y);
    let controls = scene
        .areas
        .iter()
        .filter(|area| match &area.hit {
            Hit::Origin(p) | Hit::Insert(p) => {
                scene.bar_for(p).is_some_and(|b| std::ptr::eq(b, bar))
            }
            _ => false,
        })
        .collect::<Vec<_>>();
    assert!(controls.iter().any(|a| matches!(a.hit, Hit::Origin(_))));
    assert!(controls.iter().any(|a| matches!(a.hit, Hit::Insert(_))));
    for area in &controls {
        assert!(
            painting::control_visible(&scene, area, pointer, None),
            "Every control of the hovered activation must be visible"
        );
        if matches!(area.hit, Hit::Origin(_)) {
            assert!((area.rect.center().x - bar.rect.center().x).abs() < 0.01);
            assert!(area.rect.width() >= 20.0 && area.rect.height() >= 20.0);
            assert!(
                !scene.labels.iter().any(|t| t.intersects(area.rect)),
                "Centered handles must leave the activation label readable"
            );
            for plus in controls.iter().filter(|a| matches!(a.hit, Hit::Insert(_))) {
                assert!(!area.rect.intersects(plus.rect));
            }
        }
    }
    let plus = controls
        .iter()
        .find(|a| matches!(a.hit, Hit::Insert(_)))
        .unwrap();
    for area in &controls {
        assert!(
            painting::control_visible(&scene, area, plus.rect.center(), None),
            "Moving to the plus must retain the group"
        );
    }
    let away = egui::pos2(scene.image.right() + 100.0, scene.image.bottom() + 100.0);
    for area in controls {
        assert!(!painting::control_visible(&scene, area, away, None));
    }
}
