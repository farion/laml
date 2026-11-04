//! Diagram-type and global settings parsing.
use super::{ParseError, error};
use crate::lex::split_once_top_level;
use crate::syntax::{Diagram, DiagramType, FlowDirection, LabelPlacement};

pub(super) fn parse_setting(
    text: &str,
    line: usize,
    diagram: &mut Diagram,
    seen: &mut [bool; 9],
) -> Result<(), ParseError> {
    let (key, value) =
        split_once_top_level(text, ':').ok_or_else(|| error(line, "expected setting: value"))?;
    let value = value.trim();
    let slot = match key.trim() {
        "direction" => 0,
        "naming-convention" => 1,
        "relation-label" => 2,
        "type" => 3,
        "theme" => 4,
        "background" => 5,
        "font-file" => 6,
        "font-italic-file" => 7,
        "allow-diagonal" => 8,
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
        "allow-diagonal" => {
            diagram.settings.allow_diagonal = match value {
                "true" => true,
                "false" => false,
                _ => return Err(error(line, "allow-diagonal must be true or false")),
            };
        }
        "relation-label" => {
            diagram.settings.relation_label = match value {
                "near" => LabelPlacement::Near,
                "along" => LabelPlacement::Along,
                "inline" => LabelPlacement::Inline,
                _ => return Err(error(line, "invalid relation-label setting")),
            }
        }
        "font-file" | "font-italic-file" => {
            let name = crate::lex::quoted_string(value).map_err(|message| error(line, message))?;
            if name.is_empty() {
                return Err(error(line, "font file path cannot be empty"));
            }
            if key.trim() == "font-file" {
                diagram.settings.font_file = Some(name);
            } else {
                diagram.settings.font_italic_file = Some(name);
            }
        }
        "theme" => {
            diagram.settings.theme = crate::theme::Theme::parse(value).ok_or_else(|| {
                error(
                    line,
                    "invalid theme (expected plain, modern, modern-mono or a -dark variant)",
                )
            })?;
        }
        "background" => {
            diagram.settings.transparent = match value {
                "transparent" => true,
                "opaque" => false,
                _ => return Err(error(line, "background must be opaque or transparent")),
            };
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
