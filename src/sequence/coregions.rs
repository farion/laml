//! A coregion groups occurrence endpoints on one lifeline, not whole messages.

use super::{
    AnnotationEnd, Event, SequenceDiagram,
    annotations::{Anchor, anchor},
    error,
};
use crate::lex::{split_once_top_level, valid_identifier};
use crate::parser::ParseError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coregion {
    pub participant: String,
    pub ends: Vec<Anchor>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VerticalRule {
    pub above: Anchor,
    pub below: Anchor,
    pub gap: f64,
    pub line: usize,
}

pub(super) fn parse_rule(text: &str, line: usize) -> Result<Option<VerticalRule>, ParseError> {
    let Some((above, tail)) = text.split_once(" above ") else {
        return Ok(None);
    };
    let (below, attrs) = match split_once_top_level(tail, ':') {
        Some((below, attrs)) => (below.trim(), Some(attrs.trim())),
        None => (tail.trim(), None),
    };
    let (above, below) = (anchor(above, line)?, anchor(below, line)?);
    if above == below {
        return Err(error(line, "occurrence cannot be above itself"));
    }
    let gap = if let Some(attrs) = attrs {
        let (key, value) =
            split_once_top_level(attrs, '=').ok_or_else(|| error(line, "expected gap=number"))?;
        if key.trim() != "gap" {
            return Err(error(line, "only gap is supported in vertical layout"));
        }
        let value: f64 = value
            .trim()
            .parse()
            .map_err(|_| error(line, "gap must be a number"))?;
        if !value.is_finite() || value < 0.0 {
            return Err(error(line, "gap must be finite and nonnegative"));
        }
        value
    } else {
        24.0
    };
    Ok(Some(VerticalRule {
        above,
        below,
        gap,
        line,
    }))
}

#[derive(Default)]
pub(super) struct Collector(Option<Coregion>);

impl Collector {
    pub(super) fn is_open(&self) -> bool {
        self.0.is_some()
    }

    pub(super) fn consume(
        &mut self,
        text: &str,
        line: usize,
        diagram: &mut SequenceDiagram,
    ) -> Result<bool, ParseError> {
        if let Some(region) = &mut self.0 {
            if text == "}" {
                let finished = self.0.take().expect("open region");
                if finished.ends.len() < 2 {
                    return Err(error(line, "coregion needs at least two occurrences"));
                }
                diagram.coregions.push(finished);
            } else {
                let end = anchor(text, line)?;
                if region.ends.contains(&end) {
                    return Err(error(line, "duplicate occurrence in coregion"));
                }
                region.ends.push(end);
            }
            return Ok(true);
        }
        if let Some(participant) = text.strip_prefix("coregion ") {
            let participant = participant
                .strip_suffix('{')
                .ok_or_else(|| error(line, "coregion requires '{'"))?
                .trim();
            if !valid_identifier(participant) {
                return Err(error(line, "invalid coregion participant"));
            }
            self.0 = Some(Coregion {
                participant: participant.into(),
                ends: Vec::new(),
                line,
            });
            return Ok(true);
        }
        Ok(false)
    }
}

pub(super) fn validate(diagram: &SequenceDiagram) -> Result<(), ParseError> {
    let mut used = Vec::new();
    for region in &diagram.coregions {
        let participant = diagram
            .participants
            .iter()
            .find(|p| p.id == region.participant)
            .ok_or_else(|| {
                error(
                    region.line,
                    format!("unknown coregion participant '{}'", region.participant),
                )
            })?;
        for end in &region.ends {
            let message = diagram
                .events
                .iter()
                .find_map(|event| {
                    if let Event::Message(m) = event
                        && m.name.as_deref() == Some(&end.message)
                    {
                        Some(m)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| {
                    error(
                        region.line,
                        format!("unknown named message '{}'", end.message),
                    )
                })?;
            if message.line >= region.line {
                return Err(error(
                    region.line,
                    "coregion endpoints must refer to earlier messages",
                ));
            }
            let id = if end.end == AnnotationEnd::Send {
                &message.from
            } else {
                &message.to
            };
            if id != &participant.id {
                return Err(error(
                    region.line,
                    format!(
                        "{}.{} does not occur on '{}'",
                        end.message,
                        if end.end == AnnotationEnd::Send {
                            "send"
                        } else {
                            "receive"
                        },
                        participant.id
                    ),
                ));
            }
            if participant
                .created_line
                .is_some_and(|created| message.line < created)
                || participant
                    .destroyed_line
                    .is_some_and(|destroyed| message.line >= destroyed)
            {
                return Err(error(region.line, "coregion references inactive lifeline"));
            }
            if used.contains(end) {
                return Err(error(
                    region.line,
                    "occurrence belongs to multiple coregions",
                ));
            }
            used.push(end.clone());
        }
    }
    for rule in &diagram.vertical_layout {
        if !diagram
            .coregions
            .iter()
            .any(|region| region.ends.contains(&rule.above) && region.ends.contains(&rule.below))
        {
            return Err(error(
                rule.line,
                "vertical occurrence placement requires both ends in the same coregion",
            ));
        }
    }
    for region in &diagram.coregions {
        order(diagram, region, &(0..region.ends.len()).collect::<Vec<_>>())?;
    }
    Ok(())
}

/// Stable topological sort: the baseline source order is only a visual tie-break.
pub(super) fn order(
    diagram: &SequenceDiagram,
    region: &Coregion,
    baseline: &[usize],
) -> Result<Vec<usize>, ParseError> {
    let count = region.ends.len();
    let mut indegree = vec![0_usize; count];
    let mut edges = vec![Vec::new(); count];
    for rule in &diagram.vertical_layout {
        let (Some(a), Some(b)) = (
            region.ends.iter().position(|e| e == &rule.above),
            region.ends.iter().position(|e| e == &rule.below),
        ) else {
            continue;
        };
        if !edges[a].contains(&b) {
            edges[a].push(b);
            indegree[b] += 1;
        }
    }
    for (send, first) in region.ends.iter().enumerate() {
        if first.end != AnnotationEnd::Send {
            continue;
        }
        if let Some(receive) = region
            .ends
            .iter()
            .position(|other| other.message == first.message && other.end == AnnotationEnd::Receive)
            && !edges[send].contains(&receive)
        {
            edges[send].push(receive);
            indegree[receive] += 1;
        }
    }
    let mut used = vec![false; count];
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        let next = baseline
            .iter()
            .copied()
            .find(|&i| !used[i] && indegree[i] == 0)
            .ok_or_else(|| error(region.line, "contradictory vertical coregion placement"))?;
        result.push(next);
        used[next] = true;
        for &to in &edges[next] {
            indegree[to] -= 1;
        }
    }
    Ok(result)
}
