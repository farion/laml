//! Visible general-ordering links between named occurrence endpoints.

use super::{
    SequenceDiagram,
    anchors::{MessageAnchor, position},
    arrows,
    executions::DrawnExecution,
    geometry::x_of,
    styles,
};
use crate::{
    sequence::{OrderingEnd, OrderingRule},
    styles::StyleSheet,
};
use std::fmt::Write;

pub(super) fn endpoint(
    end: &OrderingEnd,
    anchors: &[MessageAnchor],
    diagram: &SequenceDiagram,
    centers: &[f64],
    bars: &[DrawnExecution],
) -> Result<(f64, f64), String> {
    match end {
        OrderingEnd::Message(anchor) => position(anchors, &anchor.message, anchor.end),
        OrderingEnd::Execution {
            participant,
            id,
            finish,
        } => {
            let bar = bars.iter().find(|bar| {
                bar.opened.participant == *participant && bar.opened.id.as_deref() == Some(id)
            });
            let bar = bar.ok_or_else(|| format!("missing execution '{participant}.{id}'"))?;
            Ok((
                x_of(diagram, centers, participant)? + 5.0 * bar.opened.depth as f64,
                if *finish { bar.to } else { bar.opened.from },
            ))
        }
    }
}

pub(super) fn draw(
    svg: &mut String,
    rules: &[&OrderingRule],
    anchors: &[MessageAnchor],
    sheet: &StyleSheet,
    diagram: &SequenceDiagram,
    centers: &[f64],
    bars: &[DrawnExecution],
) -> Result<(), String> {
    let paint = styles::ordering(sheet);
    for (index, rule) in rules.iter().enumerate() {
        let (from_x, from_y) = endpoint(&rule.first, anchors, diagram, centers, bars)?;
        let (to_x, to_y) = endpoint(&rule.second, anchors, diagram, centers, bars)?;
        let lane = from_x.max(to_x) + 25.0 + index as f64 * 9.0;
        writeln!(svg, "<path d=\"M{from_x:.2},{from_y:.2} H{lane:.2} V{to_y:.2} H{to_x:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-dasharray=\"2 3\" data-general-ordering=\"true\" />",
            paint.color, paint.width).unwrap();
        arrows::draw(svg, (to_x, to_y), (to_x - lane, 0.0), true, &paint.color);
    }
    Ok(())
}
