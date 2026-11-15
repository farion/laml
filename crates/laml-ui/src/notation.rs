//! Concrete source types and shared compiler dispatch. No separate editor type state.
use laml::{parser::ParseError, syntax::Diagram};
mod relations;
mod templates;
pub use relations::RelationType;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Notation {
    #[default]
    Gne,
    Class,
    Object,
    Package,
    Component,
    Deployment,
    Composite,
    Profile,
    UseCase,
    State,
    Activity,
    Communication,
    InteractionOverview,
    Sequence,
    Timing,
    Bpmn,
    C4Context,
    C4Container,
    C4Component,
    C4Code,
}
impl Notation {
    pub const ALL: [Self; 20] = [
        Self::Gne,
        Self::Class,
        Self::Object,
        Self::Package,
        Self::Component,
        Self::Deployment,
        Self::Composite,
        Self::Profile,
        Self::UseCase,
        Self::State,
        Self::Activity,
        Self::Communication,
        Self::InteractionOverview,
        Self::Sequence,
        Self::Timing,
        Self::Bpmn,
        Self::C4Context,
        Self::C4Container,
        Self::C4Component,
        Self::C4Code,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Gne => "gne",
            Self::Class => "class",
            Self::Object => "object",
            Self::Package => "package",
            Self::Component => "component",
            Self::Deployment => "deployment",
            Self::Composite => "composite",
            Self::Profile => "profile",
            Self::UseCase => "usecase",
            Self::State => "state",
            Self::Activity => "activity",
            Self::Communication => "communication",
            Self::InteractionOverview => "interaction-overview",
            Self::Sequence => "sequence",
            Self::Timing => "timing",
            Self::Bpmn => "bpmn",
            Self::C4Context => "c4-context",
            Self::C4Container => "c4-container",
            Self::C4Component => "c4-component",
            Self::C4Code => "c4-code",
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Self::Gne => "Generic node-edge",
            Self::Class => "Class",
            Self::Object => "Object",
            Self::Package => "Package",
            Self::Component => "Component",
            Self::Deployment => "Deployment",
            Self::Composite => "Composite structure",
            Self::Profile => "Profile",
            Self::UseCase => "Use case",
            Self::State => "State machine",
            Self::Activity => "Activity",
            Self::Communication => "Communication",
            Self::InteractionOverview => "Interaction overview",
            Self::Sequence => "Sequence",
            Self::Timing => "Timing",
            Self::Bpmn => "BPMN",
            Self::C4Context => "Context",
            Self::C4Container => "Container",
            Self::C4Component => "Component",
            Self::C4Code => "Code",
        }
    }
    pub fn family(self) -> &'static str {
        match self {
            Self::Gne => "Generic",
            Self::Bpmn => "BPMN",
            Self::C4Context | Self::C4Container | Self::C4Component | Self::C4Code => "C4",
            _ => "UML",
        }
    }
    pub fn of(source: &str) -> Self {
        let mut settings = false;
        for raw in source.lines() {
            let text = crate::style_edit::code(raw).trim();
            if text == "settings {" {
                settings = true;
            } else if settings && text == "}" {
                settings = false;
            } else if settings && let Some(value) = text.strip_prefix("type:") {
                return Self::ALL
                    .into_iter()
                    .find(|ty| ty.key() == value.trim())
                    .unwrap_or_default();
            }
        }
        Self::Gne
    }
    pub fn operators(self) -> &'static [&'static str] {
        match self {
            Self::Gne => crate::relation_edit::TYPES,
            Self::Class | Self::C4Code => &["->", "--", "..", "..>", "--|>", "..|>", "--*", "--o"],
            Self::Object | Self::Communication => &["--", ".."],
            Self::State | Self::Activity | Self::InteractionOverview => &["-->", ".."],
            Self::UseCase => &["--", "--|>", "..>", ".."],
            Self::Package => &["..>", ".."],
            Self::Profile => &["--^>", "--|>", ".."],
            Self::Composite => &["==", "=>"],
            Self::Component => &["..>", "..|>", "--|>", "--", ".."],
            Self::Deployment => &["--", "..>", ".."],
            Self::C4Context | Self::C4Container | Self::C4Component => &["->"],
            Self::Bpmn => &["-->", "~~>", "..>", ".."],
            Self::Sequence => &["->", "->>", "<--", "-->"],
            Self::Timing => &[],
        }
    }
    pub fn graph(self) -> bool {
        !matches!(self, Self::Bpmn | Self::Sequence | Self::Timing)
    }
    pub fn c4(self) -> bool {
        self.family() == "C4"
    }
    pub fn starter(self, dark: bool) -> String {
        let example = match self {
            Self::C4Context => Some(include_str!("../../../examples/c4-context.laml")),
            Self::C4Container => Some(include_str!("../../../examples/c4-container.laml")),
            Self::C4Component => Some(include_str!("../../../examples/c4-component.laml")),
            Self::C4Code => Some(include_str!("../../../examples/c4-code.laml")),
            Self::UseCase => Some(include_str!("../../../examples/usecase-quickstart.laml")),
            Self::Composite => Some(include_str!("../../../examples/composite-basic.laml")),
            Self::Profile => Some(include_str!("../../../examples/profile-quickstart.laml")),
            Self::Communication => Some(include_str!(
                "../../../examples/communication-quickstart.laml"
            )),
            Self::InteractionOverview => Some(include_str!(
                "../../../examples/interaction-overview-basic.laml"
            )),
            _ => None,
        };
        if let Some(example) = example {
            return example.replacen(
                "settings {",
                &format!(
                    "settings {{\n    theme: {}\n    background: transparent",
                    if dark { "modern-dark" } else { "modern" }
                ),
                1,
            );
        }
        let settings = match self {
            Self::C4Context => "    scope: System\n",
            Self::C4Container => "    scope: System\n",
            Self::C4Component => "    scope: Backend\n",
            Self::C4Code => "    scope: Component\n",
            Self::Timing => "    interaction: Interaction\n",
            _ => "",
        };
        let semantics = match self {
            Self::Package => "    package Package {\n    }\n",
            Self::Profile => "    profile Profile {\n    }\n",
            Self::Composite => {
                "    class Class\n    structure Structure {\n        part Part(of=Class)\n    }\n"
            }
            Self::Deployment => "    node Node {\n    }\n",
            Self::UseCase => "    actor Actor\n",
            Self::State => "    state State\n",
            Self::Activity => "    action Action\n",
            Self::InteractionOverview => "    initial Start\n",
            Self::Communication => "    object Participant\n",
            Self::Bpmn => "    startEvent Start\n",
            Self::Sequence => "    Participant\n",
            Self::Timing => {
                "    lifeline Participant : Idle\n    state Participant at 5 : Active\n"
            }
            Self::C4Context => "    system System\n",
            Self::C4Container => "    system System {\n    }\n",
            Self::C4Component => "    container Backend {\n    }\n",
            Self::C4Code => "    component Component {\n    }\n",
            _ => "",
        };
        let layout = if self == Self::Timing {
            "layout {\n    end: 10\n    tick 0\n    tick 5\n    tick 8\n}\n"
        } else {
            ""
        };
        format!(
            "settings {{\n{}    theme: {}\n    background: transparent\n{settings}}}\nsemantics {{\n{semantics}}}\n{layout}",
            if self == Self::Gne {
                String::new()
            } else {
                format!("    type: {}\n", self.key())
            },
            if dark { "modern-dark" } else { "modern" }
        )
    }
}

pub fn parse_graph(source: &str) -> Result<Diagram, ParseError> {
    match Notation::of(source) {
        Notation::Communication => laml::communication::parse(source).map(|d| d.graph),
        Notation::InteractionOverview => laml::interaction_overview::parse(source).map(|d| d.graph),
        _ => laml::parser::parse(source),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn every_new_document_builds() {
        for ty in Notation::ALL {
            let source = ty.starter(false);
            let preview = crate::preview::build(&source, std::path::Path::new("new.laml"))
                .unwrap_or_else(|e| panic!("{}: {e}\n{source}", ty.key()));
            assert_eq!(preview.notation, ty);
        }
    }
    #[test]
    fn source_type_ignores_comments_and_content() {
        assert_eq!(
            Notation::of("// type: sequence\nsemantics {\n note N \"type: bpmn\"\n}\n"),
            Notation::Gne
        );
        assert_eq!(
            Notation::of("settings {\n type: c4-component // comment\n}\n"),
            Notation::C4Component
        );
    }
}

/// Validate before dependency IO, including formats with independent ASTs.
pub fn inspect(source: &str) -> Result<(Diagram, usize, usize), String> {
    inspect_at(source, std::path::Path::new("untitled.laml")).map_err(|e| e.to_string())
}
pub fn inspect_at(
    source: &str,
    path: &std::path::Path,
) -> Result<(Diagram, usize, usize), crate::diagnostic::Diagnostic> {
    let location =
        |e: laml::parser::ParseError| crate::diagnostic::Diagnostic::at(path, e.line, e.message);
    if Notation::of(source).graph() {
        let d = parse_graph(source).map_err(location)?;
        let (n, r) = (d.nodes.len(), d.relations.len());
        return Ok((d, n, r));
    }
    let (_, theme, transparent, font, italic) = laml::theme::preprocess_settings(source)?;
    let mut d = Diagram::default();
    d.settings.theme = theme;
    d.settings.transparent = transparent;
    d.settings.font_file = font;
    d.settings.font_italic_file = italic;
    let (nodes, relations) = match Notation::of(source) {
        Notation::Sequence => {
            let doc = laml::sequence::parse(source).map_err(location)?;
            d.styling = doc.styling;
            (doc.participants.len(), doc.events.len())
        }
        Notation::Timing => {
            let doc = laml::timing::parse(source).map_err(location)?;
            d.styling = doc.styling;
            (
                doc.lifelines.len(),
                doc.lifelines
                    .iter()
                    .map(|l| l.changes.len() + l.events.len())
                    .sum(),
            )
        }
        Notation::Bpmn => {
            let doc = laml::bpmn::parse(source)?;
            d.settings.direction = doc.direction;
            d.styling = doc.styling;
            (doc.nodes.len(), doc.edges.len())
        }
        _ => unreachable!(),
    };
    Ok((d, nodes, relations))
}

#[cfg(test)]
mod authoring_tests;

pub fn container_kind(kind: laml::syntax::NodeKind) -> bool {
    use laml::syntax::NodeKind::*;
    matches!(
        kind,
        Package
            | Profile
            | UseCaseSystem
            | Component
            | DeploymentNode
            | Device
            | ExecutionEnvironment
            | State
            | CompositeStructure
    )
}
