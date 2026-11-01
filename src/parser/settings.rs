//! Diagram-type and global settings parsing.
use super::{ParseError, error};
use crate::lex::split_once_top_level;
use crate::syntax::{Diagram, DiagramType, FlowDirection, LabelPlacement};

pub(super) fn parse_setting(
    text: &str,
    line: usize,
    diagram: &mut Diagram,
    seen: &mut [bool; 4],
) -> Result<(), ParseError> {
    let (key, value) =
        split_once_top_level(text, ':').ok_or_else(|| error(line, "expected setting: value"))?;
    let value = value.trim();
    let slot = match key.trim() {
        "direction" => 0,
        "naming-convention" => 1,
        "relation-label" => 2,
        "type" => 3,
        _ => return Err(error(line, format!("unknown setting '{}'", key.trim()))),
    };
    if seen[slot] {
        return Err(error(line, format!("duplicate setting '{}'", key.trim())));
    }
    seen[slot] = true;
    match key.trim() {
        "direction" => {
            diagram.settings.direction = match value {
                "left-to-right" => FlowDirection::LeftToRight,
                "right-to-left" => FlowDirection::RightToLeft,
                "top-to-bottom" => FlowDirection::TopToBottom,
                "bottom-to-top" => FlowDirection::BottomToTop,
                _ => return Err(error(line, "invalid diagram direction")),
            }
        }
        "naming-convention" => {
            if value != "uppercase-nodes-lowercase-relations" {
                return Err(error(line, "invalid naming convention"));
            }
            diagram.settings.strict_case = true;
        }
        "relation-label" => {
            diagram.settings.relation_label = match value {
                "near" => LabelPlacement::Near,
                "along" => LabelPlacement::Along,
                "inline" => LabelPlacement::Inline,
                _ => return Err(error(line, "invalid relation-label setting")),
            }
        }
        "type" => {
            diagram.settings.diagram_type = match value {
                "class" => DiagramType::Class,
                "object" => DiagramType::Object,
                "component" => DiagramType::Component,
                "composite" => DiagramType::Composite,
                "profile" => DiagramType::Profile,
                "deployment" => DiagramType::Deployment,
                "state" => DiagramType::State,
                "activity" => DiagramType::Activity,
                "usecase" => DiagramType::UseCase,
                "package" => DiagramType::Package,
                "communication" => DiagramType::Communication,
                "interaction-overview" => DiagramType::InteractionOverview,
                _ => return Err(error(line, "invalid graph diagram type")),
            }
        }
        _ => return Err(error(line, format!("unknown setting '{}'", key.trim()))),
    }
    Ok(())
}
