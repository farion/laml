use super::{ParseError, error};
use crate::lex::{split_once_top_level, split_top_level, valid_identifier};
use crate::syntax::{Alignment, LabelPlacement, LayoutRule, Placement, RelationLabelRule};

pub(super) fn parse_layout(text: &str, line: usize) -> Result<LayoutRule, ParseError> {
    let (placement, attrs) = match split_once_top_level(text, ':') {
        Some((placement, attrs)) => (placement.trim(), Some(attrs.trim())),
        None => (text, None),
    };
    let words: Vec<_> = placement.split_whitespace().collect();
    let (first, second, direction) = match words.as_slice() {
        [first, "<", second] | [first, "left", "of", second] => (*first, *second, Placement::Left),
        [first, ">", second] | [first, "right", "of", second] => {
            (*first, *second, Placement::Right)
        }
        [first, "^", second] | [first, "above", second] => (*first, *second, Placement::Above),
        [first, "v", second] | [first, "below", second] => (*first, *second, Placement::Below),
        _ => {
            return Err(error(
                line,
                "expected placement such as A < B or A left of B",
            ));
        }
    };
    if !valid_identifier(first) || !valid_identifier(second) {
        return Err(error(line, "invalid node identifier in layout"));
    }
    let mut rule = LayoutRule {
        first: first.into(),
        second: second.into(),
        placement: direction,
        align: None,
        gap: None,
        line,
    };
    if let Some(attrs) = attrs {
        let mut align_seen = false;
        for entry in split_top_level(attrs, ';') {
            let (key, value) = split_once_top_level(entry.trim(), '=')
                .ok_or_else(|| error(line, "expected layout attribute=value"))?;
            match key.trim() {
                "align" if !align_seen => {
                    align_seen = true;
                    rule.align = Some(match value.trim() {
                        "top" => Alignment::Top,
                        "bottom" => Alignment::Bottom,
                        "left" => Alignment::Left,
                        "right" => Alignment::Right,
                        "center" => Alignment::Center,
                        _ => return Err(error(line, "invalid alignment")),
                    });
                }
                "gap" if rule.gap.is_none() => {
                    let gap: f64 = value
                        .trim()
                        .parse()
                        .map_err(|_| error(line, "gap must be a number"))?;
                    if !gap.is_finite() || gap < 0.0 {
                        return Err(error(line, "gap must be finite and nonnegative"));
                    }
                    rule.gap = Some(gap);
                }
                "align" | "gap" => return Err(error(line, "duplicate layout attribute")),
                _ => {
                    return Err(error(
                        line,
                        format!("unknown layout attribute '{}'", key.trim()),
                    ));
                }
            }
        }
    }
    if let Some(align) = rule.align {
        let horizontal = matches!(direction, Placement::Left | Placement::Right);
        let invalid = if horizontal {
            matches!(align, Alignment::Left | Alignment::Right)
        } else {
            matches!(align, Alignment::Top | Alignment::Bottom)
        };
        if invalid {
            return Err(error(line, "alignment is incompatible with placement axis"));
        }
    }
    Ok(rule)
}

/// Per-relation routing/label placement is layout, not LSS styling.
pub(super) fn parse_relation_label(
    text: &str,
    line: usize,
) -> Result<RelationLabelRule, ParseError> {
    let (target, attrs) = split_once_top_level(text, ':')
        .ok_or_else(|| error(line, "expected relation[name] : label=near|along|inline"))?;
    let name = target
        .trim()
        .strip_prefix("relation[")
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| error(line, "expected relation[name] target"))?;
    if !valid_identifier(name) {
        return Err(error(line, "invalid relation name in layout"));
    }
    let pieces = split_top_level(attrs.trim(), ';');
    if pieces.len() != 1 {
        return Err(error(
            line,
            "expected exactly one label placement attribute",
        ));
    }
    let (key, value) = split_once_top_level(pieces[0].trim(), '=')
        .ok_or_else(|| error(line, "expected label=near|along|inline"))?;
    if key.trim() != "label" {
        return Err(error(line, "unknown relation layout attribute"));
    }
    let placement = match value.trim() {
        "near" => LabelPlacement::Near,
        "along" => LabelPlacement::Along,
        "inline" => LabelPlacement::Inline,
        _ => return Err(error(line, "invalid relation label placement")),
    };
    Ok(RelationLabelRule {
        name: name.into(),
        placement,
        line,
    })
}
