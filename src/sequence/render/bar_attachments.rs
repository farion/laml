//! Message endpoints attach to visible execution-bar edges, not the lifeline
//! running behind them. A same-row incoming call can open the bar it hits.
use super::{
    Message, SequenceDiagram,
    anchors::MessageAnchor,
    executions::{DrawnExecution, OpenExecution},
    gates,
    geometry::{self, label_width_at},
    styles,
};
use crate::styles::StyleSheet;

#[allow(clippy::too_many_arguments)]
pub(super) fn anchors(
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    centers: &[f64],
    widths: &[f64],
    messages: &[(&Message, f64)],
    bars: &[DrawnExecution],
    open: &[OpenExecution],
    final_y: f64,
) -> Result<Vec<MessageAnchor>, String> {
    let mut anchors = messages
        .iter()
        .map(|(m, y)| {
            let (from, to) = geometry::message_endpoints(diagram, sheet, centers, widths, m)?;
            Ok(MessageAnchor {
                id: m.name.clone().unwrap_or_default(),
                send: (from, *y),
                receive: (to, if m.from == m.to { y + 24.0 } else { *y }),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    gates::align(diagram, messages, &mut anchors)?;
    let mut active: Vec<_> = bars
        .iter()
        .map(|bar| DrawnExecution {
            opened: bar.opened.clone(),
            to: bar.to,
        })
        .collect();
    active.extend(open.iter().cloned().map(|bar| bar.finish(final_y + 64.0)));
    attach(diagram, sheet, &active, messages, &mut anchors)?;
    Ok(anchors)
}

fn edge(
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    bars: &[DrawnExecution],
    participant: &str,
    at_y: f64,
    center: f64,
    side: f64,
) -> Result<Option<f64>, String> {
    let Some(bar) = bars
        .iter()
        .filter(|bar| {
            bar.opened.participant == participant
                && at_y >= bar.opened.from - 0.01
                && at_y <= bar.to + 0.01
        })
        .max_by_key(|bar| bar.opened.depth)
    else {
        return Ok(None);
    };
    let person = diagram
        .participants
        .iter()
        .find(|p| p.id == participant)
        .ok_or("unknown activation participant")?;
    let paint = styles::activation(
        sheet,
        participant,
        bar.opened.id.as_deref(),
        &person.classes,
    );
    let width = if let Some(label) = &bar.opened.label {
        paint
            .width
            .max(label_width_at(label, paint.font_size)? + 20.0)
    } else {
        paint.width
    };
    let middle = center + 5.0 * bar.opened.depth as f64;
    Ok(Some(middle + side * width / 2.0))
}

pub(super) fn attach(
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    bars: &[DrawnExecution],
    messages: &[(&Message, f64)],
    anchors: &mut [MessageAnchor],
) -> Result<(), String> {
    for ((message, _), anchor) in messages.iter().zip(anchors.iter_mut()) {
        let (from, to) = (anchor.send.0, anchor.receive.0);
        if message.from == message.to {
            if let Some(right) = edge(
                diagram,
                sheet,
                bars,
                &message.from,
                anchor.send.1,
                from,
                1.0,
            )? {
                anchor.send.0 = right;
            }
            if let Some(right) = edge(diagram, sheet, bars, &message.to, anchor.receive.1, to, 1.0)?
            {
                anchor.receive.0 = right;
            }
            continue;
        }
        if message.from != "?" && !message.from.starts_with("gate[") {
            let side = if to >= from { 1.0 } else { -1.0 };
            if let Some(x) = edge(
                diagram,
                sheet,
                bars,
                &message.from,
                anchor.send.1,
                from,
                side,
            )? {
                anchor.send.0 = x;
            }
        }
        if message.to != "?"
            && !message.to.starts_with("gate[")
            && !diagram
                .participants
                .iter()
                .any(|p| p.id == message.to && p.created_line == Some(message.line))
        {
            let side = if from < to { -1.0 } else { 1.0 };
            if let Some(x) = edge(
                diagram,
                sheet,
                bars,
                &message.to,
                anchor.receive.1,
                to,
                side,
            )? {
                anchor.receive.0 = x;
            }
        }
    }
    Ok(())
}
