use super::{ParseError, error};
use crate::lex::{split_once_top_level, split_top_level, valid_identifier, valid_reference};
use crate::syntax::{
    Alignment, EndPosition, EndSide, LabelPlacement, LayoutRule, Placement, PortLayoutRule,
    PortSide, RelationEndRule, RelationLabelRule,
};

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

pub(super) fn parse_port_side(text: &str, line: usize) -> Result<PortLayoutRule, ParseError> {
    let (target, attribute) = split_once_top_level(text, ':')
        .ok_or_else(|| error(line, "expected port[id] : side=left|right"))?;
    let id = target
        .trim()
        .strip_prefix("port[")
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| error(line, "invalid port layout target"))?;
    if !valid_reference(id) {
        return Err(error(line, "invalid port ID in layout"));
    }
    let (key, value) = split_once_top_level(attribute.trim(), '=')
        .ok_or_else(|| error(line, "expected port side=left|right"))?;
    if key.trim() != "side" {
        return Err(error(line, "unknown port layout attribute"));
    }
    let side = match value.trim() {
        "left" => PortSide::Left,
        "right" => PortSide::Right,
        _ => {
            return Err(error(
                line,
                "only left/right port sides are currently implemented",
            ));
        }
    };
    Ok(PortLayoutRule {
        id: id.into(),
        side,
        line,
    })
}

/// Mandatory per-association-end text side, kept separate from style and arrow direction.
pub(super) fn parse_relation_end(text: &str, line: usize) -> Result<RelationEndRule, ParseError> {
    let (target, attrs) = split_once_top_level(text, ':').ok_or_else(|| {
        error(
            line,
            "expected relation[id].source|target : multiplicity=above; role=below",
        )
    })?;
    let (name, end) = target
        .trim()
        .strip_prefix("relation[")
        .and_then(|rest| rest.split_once("]."))
        .ok_or_else(|| error(line, "expected relation[id].source or relation[id].target"))?;
    if !valid_identifier(name) {
        return Err(error(line, "invalid relation ID in end-label layout"));
    }
    let end = match end {
        "source" => EndPosition::Source,
        "target" => EndPosition::Target,
        _ => return Err(error(line, "expected .source or .target")),
    };
    let mut rule = RelationEndRule {
        name: name.into(),
        end,
        multiplicity: None,
        role: None,
        line,
    };
    for piece in split_top_level(attrs.trim(), ';') {
        let (key, value) = split_once_top_level(piece.trim(), '=')
            .ok_or_else(|| error(line, "expected association-end attribute=value"))?;
        let side = match value.trim() {
            "above" => EndSide::Above,
            "below" => EndSide::Below,
            "left" => EndSide::Left,
            "right" => EndSide::Right,
            _ => return Err(error(line, "invalid association-end text side")),
        };
        let slot = match key.trim() {
            "multiplicity" => &mut rule.multiplicity,
            "role" => &mut rule.role,
            _ => return Err(error(line, "unknown association-end layout attribute")),
        };
        if slot.replace(side).is_some() {
            return Err(error(line, "duplicate association-end layout attribute"));
        }
    }
    if rule.multiplicity.is_none() && rule.role.is_none() {
        return Err(error(
            line,
            "association-end rule requires multiplicity or role",
        ));
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
