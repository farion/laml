use crate::lex::without_comment;
use crate::syntax::DiagramType;

pub(super) fn prescan_type(source: &str) -> DiagramType {
    let mut settings = false;
    for raw in source.lines() {
        let text = without_comment(raw).trim();
        if !settings {
            settings = text == "settings {";
        } else if text == "}" {
            settings = false;
        } else if let Some(value) = text.strip_prefix("type:") {
            return match value.trim() {
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
                _ => DiagramType::Generic,
            };
        }
    }
    DiagramType::Generic
}
