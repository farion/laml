//! Graph relation declarations, decorators, and endpoint annotations.
use super::*;

pub(super) fn parse_semantic(
    text: &str,
    line: usize,
    diagram: &mut Diagram,
    scope: Option<&str>,
) -> Result<(), ParseError> {
    // Prefixes end at a colon before the arrow; labels begin at a colon after it.
    let arrow_at = find_arrow(text).or_else(|| text.find(" .. ").map(|offset| offset + 1));
    if arrow_at.is_none() {
        return bodies::node(text, line, diagram, false, scope);
    }
    let arrow_at = arrow_at.expect("checked above");
    let (name, attrs, body) = if let Some(prefixed) = text.strip_prefix('@') {
        let (head, body) = prefixed
            .split_once(char::is_whitespace)
            .ok_or_else(|| error(line, "expected @relation-id followed by a relation"))?;
        let (id, attrs) = attributes(head, line)?;
        if id.is_empty() {
            return Err(error(line, "relation ID cannot be empty"));
        }
        (Some(id), attrs, body.trim())
    } else {
        match split_once_top_level(text, ':') {
            Some((prefix, rest)) if prefix.len() < arrow_at => {
                let (name, attrs) = attributes(prefix, line)?;
                (
                    if name.is_empty() { None } else { Some(name) },
                    attrs,
                    rest.trim(),
                )
            }
            _ => (None, Vec::new(), text),
        }
    };
    let mut relation_classes = Vec::new();
    for (key, value) in attrs {
        if key != "style-classes" {
            return Err(error(line, format!("unknown relation attribute '{key}'")));
        }
        relation_classes = classes(&value, line)?;
    }
    let (endpoints, label) = match split_once_top_level(body, ':') {
        Some((endpoints, label)) => {
            let label = label.trim();
            if label.is_empty() {
                return Err(error(line, "empty relation label"));
            }
            let label = if label.starts_with('"') {
                quoted_string(label).map_err(|msg| error(line, msg))?
            } else {
                label.into()
            };
            if label.trim().is_empty() {
                return Err(error(line, "empty relation label"));
            }
            (endpoints.trim(), Some(label))
        }
        None => (body.trim(), None),
    };
    let (endpoints, binding) = bindings::suffix(endpoints, line, diagram.settings.diagram_type)?;
    let (endpoints, stereotypes) =
        stereotypes::suffix(endpoints, line, diagram.settings.diagram_type)?;
    let tokens: Vec<_> = endpoints.split_whitespace().collect();
    let operator_index = tokens
        .iter()
        .position(|token| graph_operators::is_operator(token));
    let Some(operator_index) = operator_index else {
        return Err(error(line, "expected a relationship operator"));
    };
    if tokens.len() == 3 && tokens[2].starts_with("relation[") {
        let target = tokens[2]
            .strip_prefix("relation[")
            .and_then(|value| value.strip_suffix(']'))
            .ok_or_else(|| error(line, "invalid relation[id] attachment target"))?;
        if !valid_identifier(target)
            || !valid_reference(tokens[0])
            || tokens[1] != ".."
            || label.is_some()
            || !stereotypes.is_empty()
        {
            return Err(error(
                line,
                "edge attachments require Node .. relation[id] without end adornments or label",
            ));
        }
        diagram.attachments.push(crate::syntax::EdgeAttachment {
            name,
            classes: relation_classes,
            source: tokens[0].into(),
            target: target.into(),
            scope: scope.map(str::to_string),
            line,
        });
        return Ok(());
    }
    if !matches!(tokens.len(), 3..=5)
        || !matches!(operator_index, 1 | 2)
        || !matches!(tokens.len() - operator_index - 1, 1 | 2)
        || !valid_reference(tokens[0])
        || !valid_reference(tokens[tokens.len() - 1])
    {
        return Err(error(line, "expected relation: A [end] -- [end] B"));
    }
    let (kind, appearance, reversed) =
        graph_operators::parse(tokens[operator_index], diagram.settings.diagram_type, line)?;
    let first_end = if operator_index == 2 {
        ends::parse(tokens[1], line, diagram.settings.diagram_type)?
    } else {
        Default::default()
    };
    let second_end = if tokens.len() - operator_index == 3 {
        ends::parse(
            tokens[operator_index + 1],
            line,
            diagram.settings.diagram_type,
        )?
    } else {
        Default::default()
    };
    let (from, to, source_end, target_end) = if reversed {
        (tokens[tokens.len() - 1], tokens[0], second_end, first_end)
    } else {
        (tokens[0], tokens[tokens.len() - 1], first_end, second_end)
    };
    diagram.relations.push(RelationDecl {
        name,
        classes: relation_classes,
        stereotypes,
        binding,
        from: from.into(),
        to: to.into(),
        kind,
        appearance,
        source_end,
        target_end,
        label,
        scope: scope.map(str::to_string),
        line,
    });
    Ok(())
}
