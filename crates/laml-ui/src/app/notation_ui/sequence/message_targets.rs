//! Nonblocking target preflight. Screen geometry stays owned by the current canvas.
use super::*;
use laml_ui::{
    backend::{
        async_client,
        protocol::{self, Operation, Reply, Stamp},
    },
    notation_edit::sequence_targets::{Action, Candidate, Choice, Target},
};

pub(super) struct Pending {
    pub(super) service: async_client::Client,
    pub(super) stamp: Stamp,
    pub(super) points: Vec<Point>,
    pub(super) action: Action,
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.service.forget(self.stamp);
    }
}
fn inputs(scene: &Scene, hit: &Hit, operator: &str) -> Option<(Vec<Point>, Action)> {
    let (message, moving_source) = match hit {
        Hit::Origin(origin) => {
            return Some((
                scene.docks.clone(),
                Action::Create {
                    origin: origin.dock.clone(),
                    operator: operator.into(),
                },
            ));
        }
        Hit::Endpoint { message, source } => (message, *source),
        _ => return None,
    };
    let mut points = scene.docks.clone();
    let send = message.path.first()?.1;
    let receive = message.path.last()?.1;
    let own_y = if moving_source { send } else { receive };
    let other_y = if moving_source { receive } else { send };
    let from = scene
        .regions
        .iter()
        .find(|r| r.line == message.line && r.kind == "sequence-send")?;
    let to = scene
        .regions
        .iter()
        .find(|r| r.line == message.line && r.kind == "sequence-receive")?;
    let mut rows = vec![(own_y, false)];
    if from.id != to.id && (send - receive).abs() >= 0.01 {
        rows.push((other_y, true));
    }
    for (y, straighten) in rows {
        for line in scene
            .regions
            .iter()
            .filter(|r| r.kind == "lifeline" && y >= r.y && y <= r.y + r.height)
        {
            let bar = scene
                .bars
                .iter()
                .filter(|b| {
                    b.owner == line.id && y > b.region.y && y < b.region.y + b.region.height
                })
                .max_by(|a, b| a.region.x.total_cmp(&b.region.x));
            let x = bar.map_or(line.x, |b| b.region.x + b.region.width / 2.0);
            points.push(Point {
                dock: Dock {
                    owner: line.id.clone(),
                    before: message.line,
                    boundary: None,
                },
                at: scene.point(x, y),
                same_row: true,
                straighten,
            });
        }
    }
    Some((
        points,
        Action::Reconnect {
            line: message.line,
            source: moving_source,
        },
    ))
}
fn target_input(points: &[Point]) -> Vec<Target> {
    points
        .iter()
        .map(|p| Target {
            dock: p.dock.clone(),
            same_row: p.same_row,
            straighten: p.straighten,
        })
        .collect()
}
fn proposal(candidate: &Candidate, label: &'static str) -> Proposal {
    Proposal {
        edits: candidate.edits.clone(),
        line: Some(candidate.line),
        label,
    }
}
fn materialize(
    scene: &Scene,
    points: &[Point],
    action: &Action,
    choices: Vec<Choice>,
) -> Vec<Destination> {
    let label = if matches!(action, Action::Create { .. }) {
        "Add message"
    } else {
        "Reconnect message"
    };
    choices
        .into_iter()
        .filter_map(|choice| {
            let point = points.get(choice.target)?;
            let mut d = super::proposals::message_destination(
                scene,
                point,
                proposal(&choice.candidate, label),
            );
            d.alternatives = choice
                .alternatives
                .iter()
                .map(|(key, c)| (key.clone(), proposal(c, label)))
                .collect();
            let c = choice.candidate;
            d.preview = Some(MessagePreview {
                message: c.message,
                send: c.send,
                receive: c.receive,
                diagonal: c.diagonal,
                loop_width: c.loop_width,
            });
            Some(d)
        })
        .collect()
}
pub(super) fn start(editor: &Editor, scene: &Scene, hit: &Hit) -> Result<Option<Pending>, String> {
    let Some((points, action)) = inputs(scene, hit, &editor.graph.operator) else {
        return Ok(None);
    };
    let mut service = editor.backend.client();
    let stamp = service
        .submit(
            editor.id,
            editor.document.revision(),
            Operation::SequenceTargets {
                source: editor.document.source().into(),
                address: protocol::address(&editor.path).map_err(|e| e.to_string())?,
                presentation: editor.compiler.presentation,
                action: action.clone(),
                targets: target_input(&points),
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(Some(Pending {
        service,
        stamp,
        points,
        action,
    }))
}
impl Pending {
    pub(super) fn poll(&mut self, scene: &Scene) -> Option<Result<Vec<Destination>, String>> {
        let response = self.service.poll()?;
        Some(match response.result {
            Ok(Reply::SequenceTargets(choices)) => {
                Ok(materialize(scene, &self.points, &self.action, choices))
            }
            Err(error) => Err(error.to_string()),
            _ => Err("Invalid sequence target response".into()),
        })
    }
}
#[cfg(test)]
pub(super) fn validated(
    source: &str,
    scene: &Scene,
    hit: &Hit,
    operator: &str,
) -> Vec<Destination> {
    let (points, action) = inputs(scene, hit, operator).unwrap();
    let choices = laml_ui::backend::local().sequence_targets(
        source,
        std::path::Path::new("sequence-preview.laml"),
        laml_ui::backend::Presentation::Document,
        &action,
        &target_input(&points),
    );
    materialize(scene, &points, &action, choices)
}
