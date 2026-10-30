//! Relationship notation is independent of placement direction.
use super::{ParseError, error};
use crate::RelationKind;
use crate::syntax::{DiagramType, EdgeAppearance, EndSymbol};

pub(super) fn is_operator(token: &str) -> bool {
    matches!(
        token,
        "->" | "<-"
            | "<->"
            | "--"
            | ".."
            | "-->"
            | "<--"
            | "..>"
            | "<.."
            | "--|>"
            | "<|--"
            | "..|>"
            | "<|.."
            | "*--"
            | "--*"
            | "o--"
            | "--o"
            | "*-->"
            | "o-->"
    )
}

pub(super) fn parse(
    operator: &str,
    diagram_type: DiagramType,
    line: usize,
) -> Result<(RelationKind, EdgeAppearance, bool), ParseError> {
    use EndSymbol::{Arrow, FilledDiamond, None, OpenDiamond, Triangle};
    let solid = |start, end| EdgeAppearance {
        start,
        end,
        dashed: false,
    };
    let dashed = |end| EdgeAppearance {
        start: None,
        end,
        dashed: true,
    };
    let (kind, appearance, reversed) = match operator {
        "->" => (RelationKind::Directed, solid(None, Arrow), false),
        "<-" => (RelationKind::Directed, solid(None, Arrow), true),
        "<->" => (RelationKind::Bidirectional, solid(Arrow, Arrow), false),
        "--" => (RelationKind::Undirected, solid(None, None), false),
        ".." => (RelationKind::Undirected, dashed(None), false),
        "-->" => (RelationKind::Directed, solid(None, Arrow), false),
        "<--" => (RelationKind::Directed, solid(None, Arrow), true),
        "..>" => (RelationKind::Directed, dashed(Arrow), false),
        "<.." => (RelationKind::Directed, dashed(Arrow), true),
        "--|>" => (RelationKind::Directed, solid(None, Triangle), false),
        "<|--" => (RelationKind::Directed, solid(None, Triangle), true),
        "..|>" => (RelationKind::Directed, dashed(Triangle), false),
        "<|.." => (RelationKind::Directed, dashed(Triangle), true),
        "*--" => (RelationKind::Undirected, solid(FilledDiamond, None), false),
        "--*" => (RelationKind::Undirected, solid(None, FilledDiamond), false),
        "o--" => (RelationKind::Undirected, solid(OpenDiamond, None), false),
        "--o" => (RelationKind::Undirected, solid(None, OpenDiamond), false),
        "*-->" => (RelationKind::Directed, solid(FilledDiamond, Arrow), false),
        "o-->" => (RelationKind::Directed, solid(OpenDiamond, Arrow), false),
        _ => return Err(error(line, "unknown relation operator")),
    };
    let allowed = match diagram_type {
        DiagramType::Generic => true,
        DiagramType::Object => matches!(operator, "--" | ".."),
        DiagramType::Class => !matches!(operator, "->" | "<-" | "<->" | "<--" | "<.."),
    };
    if !allowed {
        return Err(error(
            line,
            "relationship operator is not valid for this diagram type",
        ));
    }
    Ok((kind, appearance, reversed))
}
