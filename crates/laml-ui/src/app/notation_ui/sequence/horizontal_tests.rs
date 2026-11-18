//! Focused regression for reconnecting a bent reply onto its stationary end row.
use super::*;
const MINI: &str = "settings {\n type: sequence\n}\nsemantics {\n Client\n Service\n work: Service+\n Client ->> Service\n Service-\n response: Client <-- Service: response\n}\nlayout {\n response.send ? Service.work- : align=center-y; reference=second\n}\n";
const SCREENSHOT: &str = "settings {\n type: sequence\n theme: modern-dark\n background: transparent\n}\nsemantics {\n Participant1(label=\"Participant1\")\n Client\n Service\n Client -> Service: request\n message1: Service+ -> Service: \"\"\n Service -> Client\n activation1: Service+\n Service -> Client\n message3: Service -> Service\n Service -> Participant1\n Service-\n Service-\n message2: Client <-- Service: response\n Client -> Participant1\n Note1: note \"Hier ist cool\"\n Service -> Participant1\n Service!\n Client!\n Participant1!\n}\nlayout {\n Participant1 < Client\n Participant1 < Service\n Client < Service\n message2.send ? Service.message1- : align=center-y; reference=second\n Client! ? Service! : align=center-y\n Participant1! ? Client! : align=center-y\n message1.receive ? Service.message1- : align=center-y; reference=second\n message3.receive ? Service.activation1- : align=center-y; reference=second\n Note1 over Client\n}\n";
#[test]
fn bent_reply_can_be_straightened_at_either_endpoint_without_moving_the_other() {
    let scoped = MINI
        .replace(
            " Client\n Service\n",
            " Root\n decompose Root \"Inside\" {\n Client\n Service\n",
        )
        .replace("}\nlayout {", " }\n}\nlayout {")
        .replace(
            " response.send ? Service.work-",
            " Root.response.send ? Root.Service.work-",
        );
    let reversed=MINI.replace(" work: Service+\n Client ->> Service\n Service-\n response: Client <-- Service: response", " response: Client <-- Service: response\n work: Service+\n Client ->> Service\n Service-").replace("response.send", "response.receive");
    let marker = "settings {\n type: sequence\n}\nsemantics {\n Client\n Service\n work: Service+\n response: Service -> Client+: response\n Service-\n Client-\n}\nlayout {\n response.send ? Service.work+ : align=center-y; reference=second\n}\n";
    for source in [MINI, SCREENSHOT, scoped.as_str(), reversed.as_str(), marker] {
        let preview =
            laml_ui::preview::build(source, std::path::Path::new("straighten.laml")).unwrap();
        let line = source
            .lines()
            .position(|s| s.contains("<-- Service") || s.contains("response:"))
            .unwrap()
            + 1;
        for zoom in [0.7, 1.6] {
            let scene = Scene::new(
                &preview,
                egui::Rect::from_min_size(
                    egui::pos2(91.0, 63.0),
                    egui::vec2(
                        (preview.bounds.2 - preview.bounds.0) as f32 * zoom,
                        (preview.bounds.3 - preview.bounds.1) as f32 * zoom,
                    ),
                ),
            );
            let message = scene
                .regions
                .iter()
                .find(|r| r.kind == "message" && r.line == line)
                .unwrap();
            let send = scene
                .regions
                .iter()
                .find(|r| r.kind == "sequence-send" && r.line == line)
                .unwrap();
            let receive = scene
                .regions
                .iter()
                .find(|r| r.kind == "sequence-receive" && r.line == line)
                .unwrap();
            assert!(
                (send.y - receive.y).abs() > 0.01,
                "The original reply must have different heights"
            );
            for moving_source in [false, true] {
                let hit = Hit::Endpoint {
                    message: message.clone(),
                    source: moving_source,
                };
                let (moving, stationary) = if moving_source {
                    (send, receive)
                } else {
                    (receive, send)
                };
                let destinations = message_targets::validated(source, &scene, &hit, "->");
                let d = destinations
                    .iter()
                    .find(|d| {
                        d.dock.as_ref().is_some_and(|dock| dock.owner == moving.id)
                            && (d.at.y - scene.point(stationary.x, stationary.y).y).abs() < 0.01
                            && d.preview
                                .as_ref()
                                .is_some_and(|p| (p.send.1 - p.receive.1).abs() < 0.01)
                    })
                    .expect("Projected same-height target on the moved endpoint's lifeline");
                let candidate = d.preview.as_ref().unwrap();
                assert!((candidate.send.1 - stationary.y).abs() < 0.01);
                assert!((candidate.receive.1 - stationary.y).abs() < 0.01);
                let text = edit::apply(source, &d.proposal.edits).unwrap();
                let actual =
                    laml_ui::preview::build(&text, std::path::Path::new("straighten.laml"))
                        .unwrap();
                let end = actual
                    .regions
                    .iter()
                    .find(|r| r.kind == stationary.kind && Some(r.line) == d.proposal.line)
                    .unwrap();
                assert_eq!(end.id, stationary.id);
                assert!((end.y - stationary.y).abs() < 0.01);
                assert_eq!(
                    actual
                        .regions
                        .iter()
                        .filter(|r| r.kind == "sequence-execution")
                        .count(),
                    preview
                        .regions
                        .iter()
                        .filter(|r| r.kind == "sequence-execution")
                        .count()
                );
                let g = gestures::Gesture {
                    hit: hit.clone(),
                    destinations: vec![d.clone()],
                    hot: Some(0),
                };
                let ghost = message_paint::ghost_geometry(&scene, &g, d.at, &text, "->").unwrap();
                assert!((ghost.send.1 - ghost.receive.1).abs() < 0.01);
            }
        }
    }
}
