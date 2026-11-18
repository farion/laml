//! Solve mixed occurrence/activation equalities without changing semantic events.
use super::{anchors::MessageAnchor, retime::Shiftables};
use crate::sequence::{
    AnnotationEnd, Event, Message, SequenceDiagram, activation_layout, occurrence_layout::Endpoint,
};
use std::collections::{HashMap, HashSet};
#[derive(Clone, Copy)]
struct Var {
    node: usize,
    offset: f64,
}
struct Execution {
    owner: String,
    start: usize,
    top: Var,
    bottom: Option<Var>,
}
struct Graph {
    values: Vec<f64>,
    edges: Vec<Vec<(usize, f64)>>,
    executions: Vec<Execution>,
    closed: Vec<(Var, Var)>,
    open: Vec<Var>,
    standalone: HashMap<(String, usize, bool), Var>,
    references: HashSet<usize>,
}
impl Graph {
    fn new(doc: &SequenceDiagram, messages: &[(&Message, f64)], t: &Shiftables<'_, '_>) -> Self {
        let mut g = Self {
            values: t
                .anchors
                .iter()
                .flat_map(|a| [a.send.1, a.receive.1])
                .collect(),
            edges: vec![vec![]; t.anchors.len() * 2],
            executions: vec![],
            closed: vec![],
            open: vec![],
            standalone: HashMap::new(),
            references: HashSet::new(),
        };
        for b in t.bars.iter() {
            let top = g.boundary(
                doc,
                messages,
                t.anchors,
                &b.opened.participant,
                b.opened.line,
                false,
                b.opened.from,
            );
            let bottom = g.boundary(
                doc,
                messages,
                t.anchors,
                &b.opened.participant,
                b.end_line.unwrap_or(0),
                true,
                b.to,
            );
            g.executions.push(Execution {
                owner: b.opened.participant.clone(),
                start: b.opened.line,
                top,
                bottom: Some(bottom),
            });
            g.closed.push((top, bottom));
        }
        for b in t.open.iter() {
            let top = g.boundary(
                doc,
                messages,
                t.anchors,
                &b.participant,
                b.line,
                false,
                b.from,
            );
            g.executions.push(Execution {
                owner: b.participant.clone(),
                start: b.line,
                top,
                bottom: None,
            });
            g.open.push(top);
        }
        g
    }
    #[allow(clippy::too_many_arguments)]
    fn boundary(
        &mut self,
        doc: &SequenceDiagram,
        messages: &[(&Message, f64)],
        anchors: &[MessageAnchor],
        owner: &str,
        line: usize,
        lower: bool,
        y: f64,
    ) -> Var {
        if let Some(index) = messages.iter().position(|(m, _)| m.line == line) {
            let occurrences = doc.events.iter().filter_map(|e| match e {
                Event::Activate {
                    participant,
                    line: at,
                    at_send,
                    ..
                } if !lower && participant == owner && *at == line => Some(!at_send),
                Event::Deactivate {
                    participant,
                    line: at,
                    at_send,
                    ..
                } if lower && participant == owner && *at == line => Some(!at_send),
                Event::Destroy {
                    participant,
                    line: at,
                } if lower && participant == owner && *at == line => Some(true),
                _ => None,
            });
            if let Some(receive) = occurrences.min_by(|a, b| {
                let pos = |receive: bool| {
                    if receive {
                        anchors[index].receive.1
                    } else {
                        anchors[index].send.1
                    }
                };
                (pos(*a) - y).abs().total_cmp(&(pos(*b) - y).abs())
            }) {
                let node = index * 2 + usize::from(receive);
                return Var {
                    node,
                    offset: y - self.values[node],
                };
            }
        }
        let key = (owner.to_string(), line, lower);
        if let Some(v) = self.standalone.get(&key)
            && (self.values[v.node] + v.offset - y).abs() < 0.01
        {
            return *v;
        }
        let v = Var {
            node: self.values.len(),
            offset: 0.0,
        };
        self.values.push(y);
        self.edges.push(vec![]);
        self.standalone.insert(key, v);
        v
    }
    fn endpoint(
        &self,
        doc: &SequenceDiagram,
        messages: &[(&Message, f64)],
        end: &Endpoint,
    ) -> Result<Var, String> {
        match end {
            Endpoint::Message { name, end } => {
                let i = messages
                    .iter()
                    .position(|(m, _)| m.name.as_deref() == Some(name))
                    .ok_or_else(|| format!("Missing message '{name}'"))?;
                Ok(Var {
                    node: i * 2 + usize::from(*end == AnnotationEnd::Receive),
                    offset: 0.0,
                })
            }
            Endpoint::Activation { reference, lower } => {
                let e = activation_layout::resolve(doc, reference)?;
                let b = self
                    .executions
                    .iter()
                    .find(|b| b.owner == e.owner && b.start == e.start)
                    .ok_or("Missing activation geometry")?;
                if *lower {
                    b.bottom
                        .ok_or_else(|| "Close the activation before aligning its lower edge".into())
                } else {
                    Ok(b.top)
                }
            }
        }
    }
    fn constraints(
        &mut self,
        doc: &SequenceDiagram,
        messages: &[(&Message, f64)],
    ) -> Result<(), String> {
        for rule in &doc.occurrence_layout {
            let a = self.endpoint(doc, messages, &rule.first)?;
            let b = self.endpoint(doc, messages, &rule.second)?;
            // Explicit reference=second anchors the second operand. Otherwise the
            // activation is stationary, independent of the rule's spelling order.
            let (moving, fixed) =
                if rule.reference_second || matches!(rule.first, Endpoint::Message { .. }) {
                    (a, b)
                } else {
                    (b, a)
                };
            self.references.insert(fixed.node);
            let delta = fixed.offset - moving.offset;
            if moving.node == fixed.node {
                if delta.abs() > 0.01 {
                    return Err(
                        "Conflicting alignment of a message with its own activation boundary"
                            .into(),
                    );
                }
            } else {
                self.edges[moving.node].push((fixed.node, delta));
            }
        }
        Ok(())
    }
    fn place(
        &mut self,
        doc: &SequenceDiagram,
        messages: &[(&Message, f64)],
    ) -> Result<Vec<f64>, String> {
        for _ in 0..=messages.len() * 2 {
            let values = self.solve()?;
            let mut added = false;
            for (i, (m, _)) in messages.iter().enumerate() {
                let (send, receive) = (i * 2, i * 2 + 1);
                let gap = if m.from == m.to { 24.0 } else { 0.0 };
                if values[receive] + 0.001 >= values[send] + gap {
                    continue;
                }
                let movable = |node: usize, end: AnnotationEnd| {
                    self.edges[node].is_empty()
                        && !self.references.contains(&node)
                        && !doc.coregions.iter().any(|r| {
                            r.ends
                                .iter()
                                .any(|e| m.name.as_deref() == Some(&e.message) && e.end == end)
                        })
                };
                // Only free automatic placement yields to the causal send <=
                // receive relation. Explicitly fixed endpoints are never moved.
                if movable(send, AnnotationEnd::Send) {
                    self.edges[send].push((receive, -gap));
                    added = true;
                } else if movable(receive, AnnotationEnd::Receive) {
                    self.edges[receive].push((send, gap));
                    added = true;
                }
            }
            if !added {
                return Ok(values);
            }
        }
        Err("Cannot satisfy occurrence placement and message causality".into())
    }
    fn solve(&self) -> Result<Vec<f64>, String> {
        fn visit(i: usize, g: &Graph, seen: &mut [u8], out: &mut [f64]) -> Result<f64, String> {
            if seen[i] == 2 {
                return Ok(out[i]);
            }
            if seen[i] == 1 {
                return Err("Cyclic occurrence alignment references".into());
            }
            seen[i] = 1;
            let mut value = None;
            for (target, offset) in &g.edges[i] {
                let y = visit(*target, g, seen, out)? + offset;
                if value.is_some_and(|old: f64| (old - y).abs() > 0.01) {
                    return Err("Conflicting fixed occurrence alignment references".into());
                }
                value = Some(y);
            }
            out[i] = value.unwrap_or(g.values[i]);
            seen[i] = 2;
            Ok(out[i])
        }
        let mut seen = vec![0; self.values.len()];
        let mut out = self.values.clone();
        for i in 0..out.len() {
            visit(i, self, &mut seen, &mut out)?;
        }
        Ok(out)
    }
}
fn value(values: &[f64], v: Var) -> f64 {
    values[v.node] + v.offset
}
fn update_events(doc: &SequenceDiagram, messages: &[(&Message, f64)], t: &mut Shiftables<'_, '_>) {
    for (line, y) in t.positions.iter_mut() {
        if let Some(i) = messages.iter().position(|(m, _)| m.line == *line) {
            *y = if doc.births.iter().any(|(_, at)| at == line) {
                t.anchors[i].receive.1
            } else {
                t.anchors[i].send.1
            };
        } else if let Some(b) = t
            .bars
            .iter()
            .find(|b| b.opened.line == *line || b.end_line == Some(*line))
        {
            *y = if b.opened.line == *line {
                b.opened.from
            } else {
                b.to
            };
        } else if let Some(b) = t.open.iter().find(|b| b.line == *line) {
            *y = b.from;
        }
    }
    let mut counts = HashMap::<&str, usize>::new();
    for event in &doc.events {
        if let Event::Destroy { participant, line } = event {
            let n = counts.entry(participant).or_default();
            if let Some((_, y)) = t
                .deaths
                .iter_mut()
                .filter(|(id, _)| id == participant)
                .nth(*n)
            {
                if let Some(i) = messages.iter().position(|(m, _)| m.line == *line) {
                    *y = t.anchors[i].receive.1;
                } else if let Some((_, at)) = t.positions.iter().find(|(at, _)| at == line) {
                    *y = *at;
                }
            }
            *n += 1;
        }
    }
}
pub(super) fn apply(
    doc: &SequenceDiagram,
    messages: &mut [(&Message, f64)],
    t: &mut Shiftables<'_, '_>,
) -> Result<(), String> {
    if doc.occurrence_layout.is_empty() {
        return Ok(());
    }
    let mut g = Graph::new(doc, messages, t);
    g.constraints(doc, messages)?;
    let values = g.place(doc, messages)?;
    for (i, a) in t.anchors.iter_mut().enumerate() {
        a.send.1 = values[i * 2];
        a.receive.1 = values[i * 2 + 1];
        messages[i].1 = a.send.1;
    }
    for (b, (top, bottom)) in t.bars.iter_mut().zip(&g.closed) {
        b.opened.from = value(&values, *top);
        b.to = value(&values, *bottom);
    }
    for (b, top) in t.open.iter_mut().zip(&g.open) {
        b.from = value(&values, *top);
    }
    update_events(doc, messages, t);
    *t.final_y = values.iter().copied().fold(*t.final_y, f64::max);
    Ok(())
}
pub(super) fn verify(
    doc: &SequenceDiagram,
    messages: &[(&Message, f64)],
    t: &Shiftables<'_, '_>,
) -> Result<(), String> {
    if doc.occurrence_layout.is_empty() {
        return Ok(());
    }
    let g = Graph::new(doc, messages, t);
    for rule in &doc.occurrence_layout {
        let a = g.endpoint(doc, messages, &rule.first)?;
        let b = g.endpoint(doc, messages, &rule.second)?;
        if (value(&g.values, a) - value(&g.values, b)).abs() > 0.01 {
            return Err("Occurrence alignment conflicts with another mandatory layout rule".into());
        }
    }
    super::occurrence_checks::verify(doc, t)
}
