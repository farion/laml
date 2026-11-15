//! Complete semantic relationship choices, not merely interchangeable arrowheads.
use super::Notation;
#[derive(Clone, Copy)]
pub struct RelationType { pub key: &'static str, pub title: &'static str, pub operator: &'static str, pub prefix: &'static str, pub stereotype: &'static str }
impl RelationType {
    const fn new(key: &'static str, title: &'static str, operator: &'static str, prefix: &'static str, stereotype: &'static str) -> Self {
        Self { key, title, operator, prefix, stereotype }
    }
}
const fn r(key: &'static str, title: &'static str, operator: &'static str) -> RelationType { RelationType::new(key, title, operator, "", "") }
const NOTE: RelationType = r("..", "Note attachment", "..");
impl Notation {
    pub fn relation_types(self) -> Vec<RelationType> {
        use Notation::*;
        match self {
            Gne => self.operators().iter().map(|&op| r(op, match op {
                "->" => "Arrow", "<->" => "Bidirectional", "--" => "Solid line", ".." => "Dashed line",
                "..>" => "Dashed arrow", "--^>" => "Filled triangle", "--|>" => "Hollow triangle",
                "..|>" => "Dashed triangle", "--*" => "Filled diamond", "--o" => "Hollow diamond",
                "*-->" => "Filled diamond + arrow", "o-->" => "Hollow diamond + arrow", _ => op,
            }, op)).collect(),
            Class | C4Code => vec![r("--", "Association", "--"), r("->", "Directed association", "->"),
                r("..>", "Dependency", "..>"), r("--|>", "Generalization", "--|>"), r("..|>", "Realization", "..|>"),
                r("--*", "Composition", "--*"), r("--o", "Aggregation", "--o"), NOTE],
            Object => vec![r("--", "Link", "--"), NOTE],
            Communication => vec![r("--", "Lifeline link", "--"), NOTE],
            Component => vec![r("..>", "Dependency / required interface", "..>"), r("..|>", "Provided interface", "..|>"),
                r("==", "Assembly connector", "=="), r("=>", "Delegation connector", "=>"), NOTE],
            Composite => vec![r("==", "Assembly connector", "=="), r("=>", "Delegation connector", "=>")],
            Deployment => vec![r("--", "Communication path", "--"), RelationType::new("deploy", "Deployment", "..>", "", "deploy"),
                RelationType::new("manifest", "Manifestation", "..>", "", "manifest"), NOTE],
            State => vec![r("-->", "Transition", "-->"), NOTE],
            Activity => vec![RelationType::new("control", "Control flow", "-->", "control ", ""), RelationType::new("object", "Object flow", "-->", "object ", ""), NOTE],
            InteractionOverview => vec![r("-->", "Control flow", "-->")],
            UseCase => vec![r("--", "Association", "--"), r("--|>", "Generalization", "--|>"),
                RelationType::new("include", "Include", "..>", "", "include"), RelationType::new("extend", "Extend", "..>", "", "extend"), NOTE],
            Package => vec![r("..>", "Dependency", "..>"), RelationType::new("import", "Public import", "..>", "", "import"),
                RelationType::new("access", "Private import", "..>", "", "access"), RelationType::new("merge", "Package merge", "..>", "", "merge"), NOTE],
            Profile => vec![r("--^>", "Stereotype extension", "--^>"), RelationType::new("reference", "Metaclass reference", "..>", "", "reference"),
                RelationType::new("apply", "Profile application", "..>", "", "apply"), RelationType::new("strict", "Strict profile application", "..>", "", "strict")],
            C4Context | C4Container | C4Component => vec![r("->", "Relationship", "->")],
            Bpmn => vec![r("-->", "Sequence flow", "-->"), r("~~>", "Message flow", "~~>"), r("..>", "Data association", "..>"), r("..", "Annotation association", "..")],
            Sequence => vec![r("->", "Synchronous message", "->"), r("->>", "Asynchronous message", "->>"), r("<--", "Reply", "<--")],
            Timing => vec![],
        }
    }
    pub fn relation(self, key: &str) -> Option<RelationType> {
        if let Some(r) = self.relation_types().into_iter().find(|r| r.key == key) { return Some(r); }
        let canonical = crate::relation_edit::relation_type(key);
        let mut relation = self.relation_types().into_iter().find(|r| r.key == canonical)?;
        let spelling = crate::relation_edit::OPERATORS.iter().copied().find(|op| *op == key)?;
        relation.key = spelling; relation.operator = spelling;
        Some(relation)
    }
}
