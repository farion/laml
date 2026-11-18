//! Backend-owned sequence target preflight; never render every candidate on the UI thread.
use super::{sequence_controls as controls, sequence_lookup};
use crate::{
    backend::{Backend, Presentation},
    document::Edit,
};
use controls::Dock;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    Create { origin: Dock, operator: String },
    Reconnect { line: usize, source: bool },
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Target {
    pub dock: Dock,
    pub same_row: bool,
    #[serde(default)]
    pub straighten: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub edits: Vec<Edit>,
    pub line: usize,
    pub message: laml::sequence::Message,
    pub send: (f64, f64),
    pub receive: (f64, f64),
    pub diagonal: bool,
    #[serde(default)]
    pub loop_width: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Choice {
    pub target: usize,
    pub candidate: Candidate,
    pub alternatives: Vec<(String, Candidate)>,
}
type Geometry = (
    laml::sequence::SequenceDiagram,
    Vec<laml::authoring::Region>,
);
fn compile(
    source: &str,
    result: Result<(Vec<Edit>, usize), String>,
    geometry: &mut impl FnMut(&str) -> Option<Geometry>,
) -> Option<Candidate> {
    let (edits, line) = result.ok()?;
    if edits.is_empty() {
        return None;
    }
    let text = super::apply(source, &edits).ok()?;
    let (doc, regions) = geometry(&text)?;
    let message = sequence_lookup::message(&doc, line)?;
    let send = regions
        .iter()
        .find(|r| r.kind == "sequence-send" && r.line == line && r.id == message.from)?;
    let receive = regions
        .iter()
        .find(|r| r.kind == "sequence-receive" && r.line == line && r.id == message.to)?;
    let diagonal = regions
        .iter()
        .any(|r| r.kind == "message" && r.line == line && r.path.len() == 2);
    let loop_width = if message.from == message.to {
        regions
            .iter()
            .find(|r| r.kind == "message" && r.line == line && r.path.len() > 1)
            .map(|r| r.path[1].0 - send.x.max(receive.x))
    } else {
        None
    };
    Some(Candidate {
        edits,
        line,
        message,
        send: (send.x, send.y),
        receive: (receive.x, receive.y),
        diagonal,
        loop_width,
    })
}
/// Conservative fallback for backends that own additional compilation policy.
pub fn build(
    backend: &(impl Backend + ?Sized),
    source: &str,
    path: &Path,
    presentation: Presentation,
    action: &Action,
    targets: &[Target],
) -> Vec<Choice> {
    build_with(source, action, targets, |text| {
        let preview = backend.compile(text, path, presentation).ok()?;
        Some((laml::sequence::parse(text).ok()?, preview.regions))
    })
}
/// Candidate generation is identical for full previews and geometry-only preflight.
pub(crate) fn build_with(
    source: &str,
    action: &Action,
    targets: &[Target],
    mut geometry: impl FnMut(&str) -> Option<Geometry>,
) -> Vec<Choice> {
    fn owners(
        doc: &laml::sequence::SequenceDiagram,
        prefix: &str,
        out: &mut std::collections::HashSet<String>,
    ) {
        out.extend(doc.participants.iter().map(|p| format!("{prefix}{}", p.id)));
        for d in &doc.decompositions {
            owners(&d.diagram, &format!("{prefix}{}.", d.parent), out);
        }
    }
    let Ok(doc) = laml::sequence::parse(source) else {
        return vec![];
    };
    let mut allowed = std::collections::HashSet::new();
    owners(&doc, "", &mut allowed);
    // Wire addresses are references, never source snippets or asset declarations.
    if matches!(action,Action::Create{origin,..} if !allowed.contains(&origin.owner)) {
        return vec![];
    }
    let mut cache = HashMap::<Target, Option<(Candidate, Vec<(String, Candidate)>)>>::new();
    let mut out = Vec::new();
    for (index, target) in targets.iter().enumerate() {
        if !allowed.contains(&target.dock.owner) {
            continue;
        }
        let choice = cache.entry(target.clone()).or_insert_with(|| match action {
            Action::Create { origin, operator } => {
                if let Some(c) = compile(
                    source,
                    controls::message_with_line(source, origin, &target.dock, operator),
                    &mut geometry,
                ) {
                    return Some((c, vec![]));
                }
                let alternatives = crate::notation::Notation::Sequence
                    .relation_types()
                    .into_iter()
                    .filter(|ty| ty.key != operator)
                    .filter_map(|ty| {
                        compile(
                            source,
                            controls::message_with_line(source, origin, &target.dock, ty.key),
                            &mut geometry,
                        )
                        .map(|c| (ty.key.to_owned(), c))
                    })
                    .collect::<Vec<_>>();
                alternatives
                    .first()
                    .map(|(_, c)| (c.clone(), alternatives.clone()))
            }
            Action::Reconnect {
                line,
                source: moving_source,
            } => {
                let result = if target.straighten {
                    super::sequence_horizontal::reconnect(
                        source,
                        *line,
                        *moving_source,
                        &target.dock,
                    )
                } else {
                    controls::reconnect(
                        source,
                        *line,
                        *moving_source,
                        &target.dock,
                        target.same_row,
                    )
                };
                compile(source, result, &mut geometry)
                    .filter(|c| !target.straighten || (c.send.1 - c.receive.1).abs() < 0.01)
                    .map(|c| (c, vec![]))
            }
        });
        if let Some((candidate, alternatives)) = choice {
            out.push(Choice {
                target: index,
                candidate: candidate.clone(),
                alternatives: alternatives.clone(),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scoped_target_preflight_roundtrips_through_the_wire_and_rejects_unknown_owners() {
        let source = "settings {\n type: sequence\n}\nsemantics {\n Root\n decompose Root \"Inside\" {\n  A\n  B\n  work: B+\n  request: A -> B\n  B-\n }\n}\n";
        let origin = Dock {
            owner: "Root.A".into(),
            before: 10,
            boundary: None,
        };
        let target = Target {
            dock: Dock {
                owner: "Root.B".into(),
                before: 9,
                boundary: Some(controls::Boundary::Start(9)),
            },
            same_row: false,
            straighten: false,
        };
        let action = Action::Create {
            origin: origin.clone(),
            operator: "->".into(),
        };
        let choices = build(
            crate::backend::local().as_ref(),
            source,
            Path::new("targets.laml"),
            Presentation::Document,
            &action,
            std::slice::from_ref(&target),
        );
        assert!(!choices.is_empty());
        let optimized = crate::backend::local().sequence_targets(
            source,
            Path::new("targets.laml"),
            Presentation::Document,
            &action,
            std::slice::from_ref(&target),
        );
        assert_eq!(
            serde_json::to_value(&optimized).unwrap(),
            serde_json::to_value(&choices).unwrap()
        );
        assert_eq!(choices[0].candidate.message.from, "Root.A");
        assert_eq!(choices[0].candidate.message.to, "Root.B");
        let response = crate::backend::protocol::Response {
            stamp: crate::backend::protocol::Stamp {
                version: 1,
                session: 1,
                document: 1,
                request: 1,
                revision: 0,
            },
            result: Ok(crate::backend::protocol::Reply::SequenceTargets(choices)),
        };
        let bytes = crate::backend::protocol::codec::encode_response(&response).unwrap();
        let decoded = crate::backend::protocol::codec::decode_response(&bytes).unwrap();
        assert!(
            matches!(decoded.result,Ok(crate::backend::protocol::Reply::SequenceTargets(c)) if !c.is_empty())
        );
        let bad = Action::Create {
            origin: Dock {
                owner: "Root.A\n}\nstyling {".into(),
                ..origin
            },
            operator: "->".into(),
        };
        assert!(
            build(
                crate::backend::local().as_ref(),
                source,
                Path::new("targets.laml"),
                Presentation::Document,
                &bad,
                &[target]
            )
            .is_empty()
        );
    }
}

#[cfg(test)]
#[path = "sequence_targets_bench.rs"]
mod bench;
