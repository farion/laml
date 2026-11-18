//! Compiler arrow mechanics with drag endpoints anchored to the current canvas.
use super::*;
use laml::sequence::{Message, MessageKind, render::message_geometry as geometry};
pub(super) fn paint(
    ui: &egui::Ui,
    scene: &Scene,
    preview: &Preview,
    message: &Message,
    send: (f64, f64),
    receive: (f64, f64),
    diagonal: bool,
    loop_width: Option<f64>,
) {
    let scale = scene.image.width() / (scene.bounds.2 - scene.bounds.0) as f32;
    let mut sheet = preview.styles.clone();
    sheet.theme = preview.diagram.settings.theme;
    let (color, width) = geometry::appearance(&sheet, message.name.as_deref(), &message.classes);
    if color == "none" {
        return;
    }
    let color = egui::Color32::from_hex(&color)
        .unwrap_or_else(|_| appearance::color(ui.ctx(), appearance::TEXT));
    let stroke = egui::Stroke::new((width as f32 * scale).max(0.5), color);
    let path = if message.from == message.to {
        geometry::self_path(
            send,
            receive,
            send.0.max(receive.0) + loop_width.unwrap_or(44.0),
        )
    } else {
        geometry::path(send, receive, false, diagonal)
    };
    let points = path
        .iter()
        .map(|p| scene.point(p.0, p.1))
        .collect::<Vec<_>>();
    if geometry::dashed(message.kind) {
        ui.painter().extend(egui::Shape::dashed_line(
            &points,
            stroke,
            7.0 * scale,
            5.0 * scale,
        ));
    } else {
        ui.painter().add(egui::Shape::line(points, stroke));
    }
    if message.kind == MessageKind::Delete {
        let at = scene.point(receive.0, receive.1);
        for sign in [-1.0, 1.0] {
            ui.painter().line_segment(
                [
                    at + egui::vec2(-7.0, -7.0 * sign) * scale,
                    at + egui::vec2(7.0, 7.0 * sign) * scale,
                ],
                stroke,
            );
        }
    } else if message.to != "?" {
        let previous = path[path.len() - 2];
        let head = geometry::arrowhead(
            receive,
            (receive.0 - previous.0, receive.1 - previous.1),
            sheet.theme.modern(),
        );
        let points = head
            .iter()
            .map(|p| scene.point(p.0, p.1))
            .collect::<Vec<_>>();
        if geometry::filled(message.kind, message.filled_reply) {
            ui.painter().add(egui::Shape::convex_polygon(
                points,
                color,
                egui::Stroke::NONE,
            ));
        } else {
            ui.painter().add(egui::Shape::line(
                vec![points[1], points[0], points[2]],
                stroke,
            ));
        }
    }
}
pub(super) fn ghost(
    ui: &egui::Ui,
    scene: &Scene,
    preview: &Preview,
    g: &gestures::Gesture,
    pointer: egui::Pos2,
    source: &str,
    operator: &str,
) {
    if let Some(ghost) = ghost_geometry(scene, g, pointer, source, operator) {
        paint(
            ui,
            scene,
            preview,
            &ghost.message,
            ghost.send,
            ghost.receive,
            ghost.diagonal,
            ghost.loop_width,
        );
    }
}

/// Candidate compilation validates the edit, not the on-screen drag coordinates.
/// Its layout may move/resize participants and choose a different attachment side.
/// During the gesture, keep the active endpoint exactly on the visible target.
pub(super) fn ghost_geometry(
    scene: &Scene,
    g: &gestures::Gesture,
    pointer: egui::Pos2,
    source: &str,
    operator: &str,
) -> Option<MessagePreview> {
    let doc = laml::sequence::parse(source).ok()?;
    let at = g.hot.map_or(pointer, |i| g.destinations[i].at);
    let local = |p: egui::Pos2| {
        let scale = f64::from(scene.image.width()) / (scene.bounds.2 - scene.bounds.0);
        (
            scene.bounds.0 + f64::from(p.x - scene.image.left()) / scale,
            scene.bounds.1 + f64::from(p.y - scene.image.top()) / scale,
        )
    };
    let (message, send, receive) = match &g.hit {
        Hit::Origin(origin) => {
            let to = g
                .hot
                .and_then(|i| g.destinations[i].dock.as_ref())
                .map_or("Target".to_string(), |dock| dock.owner.clone());
            let message = laml::sequence::Message {
                name: None,
                classes: vec![],
                from: origin.dock.owner.clone(),
                to: to.clone(),
                label: None,
                kind: match operator {
                    "->>" => MessageKind::Async,
                    "<--" => MessageKind::Reply,
                    _ => MessageKind::Sync,
                },
                filled_reply: false,
                line: 0,
            };
            let receive = local(at);
            let from = local(origin.at);
            let send = (
                from.0,
                receive.1 - if to == origin.dock.owner { 24.0 } else { 0.0 },
            );
            (message, send, receive)
        }
        Hit::Endpoint {
            message: region,
            source: moving_source,
        } => {
            let m = edit::sequence_lookup::message(&doc, region.line)?;
            let mut m = m.clone();
            let mut send = region.path[0];
            let mut receive = *region.path.last().unwrap();
            let moving = local(at);
            let owner = g
                .hot
                .and_then(|i| g.destinations[i].dock.as_ref())
                .map(|dock| dock.owner.clone());
            if *moving_source {
                if let Some(owner) = owner {
                    m.from = owner;
                }
                send = moving;
                receive.1 = send.1 + if m.from == m.to { 24.0 } else { 0.0 };
            } else {
                if let Some(owner) = owner {
                    m.to = owner;
                }
                receive = moving;
                send.1 = receive.1 - if m.from == m.to { 24.0 } else { 0.0 };
            }
            (m, send, receive)
        }
        Hit::Event(region) if region.kind == "message" && !region.path.is_empty() => {
            let m = edit::sequence_lookup::message(&doc, region.line)?;
            let mut send = region.path[0];
            let mut receive = *region.path.last().unwrap();
            let dy = local(at).1 - send.1;
            send.1 += dy;
            receive.1 += dy;
            (m.clone(), send, receive)
        }
        _ => return None,
    };
    let candidate = g.hot.and_then(|i| g.destinations[i].preview.as_ref());
    Some(MessagePreview {
        message: candidate.map_or(message, |p| p.message.clone()),
        send,
        receive,
        diagonal: candidate.map_or(doc.allow_diagonal, |p| p.diagonal),
        loop_width: candidate
            .and_then(|p| p.loop_width)
            .or_else(|| match &g.hit {
                Hit::Endpoint { message, .. } | Hit::Event(message) if message.path.len() == 4 => {
                    Some(message.path[1].0 - message.path[0].0.max(message.path[3].0))
                }
                _ => None,
            }),
    })
}
