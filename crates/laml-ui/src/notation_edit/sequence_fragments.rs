//! Explicit structural operations; pointer position never silently changes operands.
use super::*;
pub fn wrap(
    source: &str,
    first: usize,
    last: usize,
    kind: &str,
    label: &str,
) -> Result<Vec<Edit>, String> {
    if ![
        "opt", "alt", "par", "loop", "critical", "break", "neg", "assert", "strict", "seq",
        "ignore", "consider",
    ]
    .contains(&kind)
    {
        return Err("Choose a supported fragment operator".into());
    }
    let first = record(source, first)?;
    let last = record(source, last)?;
    if first.section != "semantics"
        || last.section != "semantics"
        || first.parent != last.parent
        || last.block.end < first.block.end
    {
        return Err("Choose an ordered range in the same fragment operand".into());
    }
    let raw = &source[first.range.clone()];
    let indent = &raw[..raw.len() - raw.trim_start().len()];
    let nl = crate::element_edit::newline(source);
    let mut body = format!(
        "{indent}{kind} {} {{{nl}",
        crate::relation_edit::quote(label)
    );
    for line in source[first.block.start..last.block.end].lines() {
        body.push_str("    ");
        body.push_str(line);
        body.push_str(nl);
    }
    body.push_str(&format!("{indent}}}{nl}"));
    if kind == "alt" || kind == "par" {
        body.push_str(&format!(
            "{indent}{} \"\" {{{nl}{indent}}}{nl}",
            if kind == "alt" { "else" } else { "and" }
        ));
    }
    let edits = vec![Edit {
        range: first.block.start..last.block.end,
        replacement: body,
    }];
    let candidate = apply(source, &edits)?;
    laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    Ok(edits)
}
pub fn branch(source: &str, frame: usize, label: &str) -> Result<Vec<Edit>, String> {
    let r = record(source, frame)?;
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    let operator = doc
        .events
        .iter()
        .find_map(|e| match e {
            laml::sequence::Event::FrameStart { operator, line, .. } if *line == frame => {
                Some(operator.as_str())
            }
            _ => None,
        })
        .ok_or("Choose an alternative or parallel fragment")?;
    let kind = if operator.split_whitespace().any(|w| w == "alt") {
        "else"
    } else if operator.split_whitespace().any(|w| w == "par") {
        "and"
    } else {
        return Err("Only alternative and parallel fragments have additional operands".into());
    };
    let raw = &source[r.range.clone()];
    let indent = &raw[..raw.len() - raw.trim_start().len()];
    let nl = crate::element_edit::newline(source);
    let edits = vec![Edit {
        range: r.block.end..r.block.end,
        replacement: format!(
            "{indent}{kind} {} {{{nl}{indent}}}{nl}",
            crate::relation_edit::quote(label)
        ),
    }];
    laml::sequence::parse(&apply(source, &edits)?).map_err(|e| e.message)?;
    Ok(edits)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wraps_existing_events_without_inventing_messages_and_adds_operand() {
        let source="settings {\n type: sequence\n}\nsemantics {\n A\n B\n A ->> B: one // keep\n B ->> A: two\n}\n".replace('\n',"\r\n");
        let wrapped = apply(&source, &wrap(&source, 7, 8, "alt", "choice").unwrap()).unwrap();
        assert!(wrapped.contains("// keep\r\n"));
        assert!(!wrapped.replace("\r\n", "").contains('\n'));
        let result = apply(&wrapped, &branch(&wrapped, 7, "third").unwrap()).unwrap();
        let doc = laml::sequence::parse(&result).unwrap();
        assert_eq!(
            doc.events
                .iter()
                .filter(|e| matches!(e, laml::sequence::Event::Message(_)))
                .count(),
            2
        );
        assert_eq!(
            doc.events
                .iter()
                .filter(|e| matches!(e, laml::sequence::Event::FrameBranch { .. }))
                .count(),
            2
        );
    }
}
