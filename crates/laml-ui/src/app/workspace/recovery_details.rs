//! Cheap, cached recovery descriptions: parse source, never lay out or render it.
use laml_ui::notation::{self, Notation};

pub(super) struct Details {
    pub kind: String,
    pub count: String,
    pub contents: String,
}
impl Details {
    pub fn new(source: &str) -> Self {
        let ty = Notation::of(source);
        let kind = match ty {
            Notation::Gne => "Generic node-edge diagram".into(),
            Notation::Bpmn => "BPMN - Process diagram".into(),
            _ => format!("{} - {} diagram", ty.family(), ty.title()),
        };
        let names: Option<Vec<String>> = match ty {
            Notation::Bpmn => laml::bpmn::parse(source).ok().map(|d| d.nodes.into_iter().map(|n| n.label).collect()),
            Notation::Sequence => laml::sequence::parse(source).ok().map(|d| d.participants.into_iter().map(|n| n.label).collect()),
            Notation::Timing => laml::timing::parse(source).ok().map(|d| d.lifelines.into_iter().map(|n| n.id).collect()),
            _ => {
                let parsed = if ty.c4() { laml::c4::parse(source) } else { notation::parse_graph(source) };
                parsed.ok().map(|d| d.nodes.into_iter().map(|n| n.label.unwrap_or(n.id)).collect())
            }
        };
        let Some(names) = names else {
            return Self { kind, count: "Incomplete source".into(),
                contents: "Can be restored and corrected in the editor.".into() };
        };
        let unit = match ty {
            Notation::Sequence => if names.len() == 1 { "participant" } else { "participants" },
            Notation::Timing => if names.len() == 1 { "lifeline" } else { "lifelines" },
            _ => if names.len() == 1 { "element" } else { "elements" },
        };
        let count = format!("{} {unit}", names.len());
        let mut contents = names.iter().take(4).map(|s| {
            let clean = s.split_whitespace().collect::<Vec<_>>().join(" ");
            if clean.chars().count() > 40 { format!("{}…", clean.chars().take(40).collect::<String>()) } else { clean }
        }).collect::<Vec<_>>().join(" · ");
        if names.len() > 4 { contents.push_str(&format!(" · +{} more", names.len() - 4)); }
        if contents.is_empty() { contents = "No elements yet".into(); }
        Self { kind, count, contents }
    }
}
