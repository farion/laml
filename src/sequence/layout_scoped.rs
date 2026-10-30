//! Resolve local layout IDs of inline decompositions after parsing semantics.

use super::{
    SequenceDiagram, annotations, continuations, coregions, decompositions, error, frame_layout,
    layout, notes,
};
use crate::lex::split_once_top_level;
use crate::parser::ParseError;

fn strip_scope(text: &str, owner: &str, line: usize) -> Result<String, ParseError> {
    let (placement, attrs) = match split_once_top_level(text, ':') {
        Some((placement, attrs)) => (placement, Some(attrs)),
        None => (text, None),
    };
    let prefix = format!("{owner}.");
    let words = placement
        .split_whitespace()
        .map(|word| {
            if matches!(
                word,
                "<" | ">" | "left" | "right" | "of" | "over" | "above" | "spans" | "covers"
            ) {
                return Ok(word.to_string());
            }
            word.split(',')
                .map(|id| {
                    id.strip_prefix(&prefix).map(str::to_string).ok_or_else(|| {
                        error(
                            line,
                            format!("scoped layout reference '{id}' must start with '{prefix}'"),
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()
                .map(|ids| ids.join(","))
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(" ");
    Ok(match attrs {
        Some(attrs) => format!("{words} : {attrs}"),
        None => words,
    })
}

pub(super) fn add(
    diagram: &mut SequenceDiagram,
    text: &str,
    line: usize,
) -> Result<(), ParseError> {
    let first = text.split_whitespace().next().unwrap_or("");
    if let Some((owner, _)) = first.split_once('.')
        && let Some(index) = diagram
            .decompositions
            .iter()
            .position(|d| d.parent == owner)
    {
        let local = strip_scope(text, owner, line)?;
        return add(&mut diagram.decompositions[index].diagram, &local, line);
    }
    if let Some(coverage) = frame_layout::parse(text, line)? {
        diagram.frame_layout.push(coverage);
    } else if let Some(place) = notes::parse_placement(text, line)? {
        diagram.note_layout.push(place);
    } else if let Some(place) = continuations::parse_placement(text, line)? {
        diagram.continuation_layout.push(place);
    } else if let Some(rule) = coregions::parse_rule(text, line)? {
        diagram.vertical_layout.push(rule);
    } else {
        diagram.layout.push(layout::parse_rule(text, line)?);
    }
    Ok(())
}

pub(super) fn validate(diagram: &mut SequenceDiagram) -> Result<(), ParseError> {
    layout::route_decompositions(diagram)?;
    for nested in &mut diagram.decompositions {
        validate(&mut nested.diagram)?;
    }
    layout::validate_and_order(diagram)?;
    decompositions::validate(diagram)?;
    frame_layout::validate(diagram)?;
    notes::validate(diagram)?;
    continuations::validate(diagram)?;
    annotations::validate(diagram)?;
    coregions::validate(diagram)?;
    Ok(())
}
