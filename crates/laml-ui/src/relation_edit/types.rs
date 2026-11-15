//! UI relation types and orientation, without introducing new LAML operators.
use super::*;

/// One representative per visual type; aliases and mirrored symbols are omitted.
pub const TYPES: &[&str] = &[
    "->", "<->", "--", "..", "..>", "--^>", "--|>", "..|>", "--*", "--o", "*-->", "o-->",
];

pub fn relation_type(operator: &str) -> &str {
    match operator {
        "-->" | "<-" | "<--" => "->",
        "<.." => "..>",
        "<|--" => "--|>",
        "<|.." => "..|>",
        "*--" => "--*",
        "o--" => "--o",
        other => other,
    }
}

fn mirrored(operator: &str) -> bool {
    matches!(
        operator,
        "<-" | "<--" | "<.." | "<|--" | "<|.." | "*--" | "o--"
    )
}

pub fn directional(kind: &str) -> bool {
    !matches!(kind, "--" | ".." | "<->")
}

/// Oriented lexical endpoints, including ports. For diamonds, points toward the marker.
pub fn direction(source: &str, line: usize) -> Result<(String, String), String> {
    let spans = endpoint_spans(source, line)?;
    let mut ends = (source[spans.from].to_owned(), source[spans.to].to_owned());
    if mirrored(&source[spans.operator]) {
        std::mem::swap(&mut ends.0, &mut ends.1);
    }
    Ok(ends)
}

fn set_type(source: &str, line: usize, kind: &str, reverse: bool) -> Result<Vec<Edit>, String> {
    if !TYPES.contains(&kind) {
        return Err("Unsupported relation type".into());
    }
    let opposite = match kind {
        "->" => Some("<-"),
        "..>" => Some("<.."),
        "--|>" => Some("<|--"),
        "..|>" => Some("<|.."),
        "--*" => Some("*--"),
        "--o" => Some("o--"),
        _ => None,
    };
    let mut edits = change_operator(
        source,
        line,
        if reverse {
            opposite.unwrap_or(kind)
        } else {
            kind
        },
    )?;
    if reverse && opposite.is_none() && directional(kind) {
        // Existing syntax has no mirrored token for these types. Exchange whole
        // endpoint expressions, keeping adornments with their endpoint, not the side.
        let spans = endpoint_spans(source, line)?;
        let left_end = source[..spans.operator.start].trim_end().len();
        let right_start = spans.operator.end + source[spans.operator.end..].len()
            - source[spans.operator.end..].trim_start().len();
        let left = spans.from.start..left_end;
        let right = right_start..spans.to.end;
        let left_node = source[spans.from.clone()].trim();
        let right_node = source[spans.to.clone()].trim();
        let left_adornments = source[spans.from.end..left_end].trim();
        let right_adornments = source[right_start..spans.to.start].trim();
        let join = |a: &str, b: &str| {
            if a.is_empty() {
                b.to_owned()
            } else if b.is_empty() {
                a.to_owned()
            } else {
                format!("{a} {b}")
            }
        };
        edits.push(Edit {
            range: left,
            replacement: join(right_node, right_adornments),
        });
        edits.push(Edit {
            range: right,
            replacement: join(left_adornments, left_node),
        });
    }
    Ok(edits)
}

/// Preserve orientation when choosing a different visual type.
pub fn change_type(source: &str, line: usize, kind: &str) -> Result<Vec<Edit>, String> {
    let op = operator(source, line)?;
    set_type(source, line, kind, mirrored(&op))
}

/// Choose direction relative to stable endpoint names, not screen position.
pub fn change_direction(source: &str, line: usize, ascending: bool) -> Result<Vec<Edit>, String> {
    let spans = endpoint_spans(source, line)?;
    let op = &source[spans.operator];
    let kind = relation_type(op);
    if !directional(kind) {
        return Ok(vec![]);
    }
    let lexical_ascending = source[spans.from] <= source[spans.to];
    set_type(source, line, kind, ascending != lexical_ascending)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patched(source: &str, edits: Vec<Edit>) -> String {
        crate::node_edit::candidate(source, &edits).unwrap()
    }

    #[test]
    fn aliases_and_mirrors_have_one_type() {
        for op in OPERATORS {
            // Assembly/delegation are notation-specific composite connectors,
            // not choices in the generic relation picker.
            if ["==", "=>"].contains(op) {
                assert!(
                    crate::notation::Notation::Composite
                        .operators()
                        .contains(op)
                );
            } else {
                assert!(TYPES.contains(&relation_type(op)), "{op}");
            }
        }
        assert_eq!(relation_type("->"), relation_type("-->"));
        assert_eq!(relation_type("<--"), relation_type("->"));
        assert_eq!(relation_type("*--"), relation_type("--*"));
        let mut unique = TYPES.to_vec();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), TYPES.len());
    }

    #[test]
    fn every_directional_type_roundtrips_both_directions() {
        for kind in TYPES.iter().copied().filter(|kind| directional(kind)) {
            let source = format!("semantics {{\n A\n B\n @r A {kind} B : label // keep\n}}\n");
            let reverse = patched(&source, change_direction(&source, 4, false).unwrap());
            assert_eq!(
                direction(&reverse, 4).unwrap(),
                ("B".into(), "A".into()),
                "{kind}"
            );
            assert!(reverse.contains(": label // keep"));
            assert_eq!(relation_type(&operator(&reverse, 4).unwrap()), kind);
            let forward = patched(&reverse, change_direction(&reverse, 4, true).unwrap());
            assert_eq!(
                direction(&forward, 4).unwrap(),
                ("A".into(), "B".into()),
                "{kind}"
            );
        }
    }

    #[test]
    fn type_changes_preserve_direction_and_endpoint_adornments() {
        let source =
            "semantics {\r\n A\r\n B\r\n @r A \"1\"/owner <- /items B : label // keep\r\n}\r\n";
        let updated = patched(source, change_type(source, 4, "--^>").unwrap());
        assert!(updated.contains("@r B /items --^> \"1\"/owner A : label // keep\r\n"));
        assert_eq!(direction(&updated, 4).unwrap(), ("B".into(), "A".into()));
    }
}
