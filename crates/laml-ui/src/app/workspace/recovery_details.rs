//! Cache parsed recovery metadata, not its language-dependent presentation.
use crate::i18n::tr;
use laml_ui::notation::{self, Notation};

pub(super) struct Details {
    ty: Notation,
    count: Option<usize>,
    names: Vec<String>,
}
impl Details {
    pub fn new(source: &str) -> Self {
        let ty = Notation::of(source);
        let names: Option<Vec<String>> = match ty {
            Notation::Bpmn => laml::bpmn::parse(source)
                .ok()
                .map(|d| d.nodes.into_iter().map(|n| n.label).collect()),
            Notation::Sequence => laml::sequence::parse(source)
                .ok()
                .map(|d| d.participants.into_iter().map(|n| n.label).collect()),
            Notation::Timing => laml::timing::parse(source)
                .ok()
                .map(|d| d.lifelines.into_iter().map(|n| n.id).collect()),
            _ => {
                let parsed = if ty.c4() {
                    laml::c4::parse(source)
                } else {
                    notation::parse_graph(source)
                };
                parsed.ok().map(|d| {
                    d.nodes
                        .into_iter()
                        .map(|n| n.label.unwrap_or(n.id))
                        .collect()
                })
            }
        };
        let count = names.as_ref().map(Vec::len);
        let names = names
            .unwrap_or_default()
            .into_iter()
            .take(4)
            .map(|s| {
                let clean = s.split_whitespace().collect::<Vec<_>>().join(" ");
                if clean.chars().count() > 40 {
                    format!("{}…", clean.chars().take(40).collect::<String>())
                } else {
                    clean
                }
            })
            .collect();
        Self { ty, count, names }
    }
    pub fn kind(&self) -> String {
        match self.ty {
            Notation::Gne => tr("Generic node-edge diagram").into(),
            Notation::Bpmn => tr("BPMN - Process diagram").into(),
            _ => crate::i18n::format(
                "{family} - {title} diagram",
                &[
                    ("family", self.ty.family().into()),
                    ("title", tr(self.ty.title()).into()),
                ],
            ),
        }
    }
    pub fn count(&self) -> String {
        let Some(count) = self.count else {
            return tr("Incomplete source").into();
        };
        let unit = match (self.ty, count) {
            (Notation::Sequence, 1) => "participant",
            (Notation::Sequence, _) => "participants",
            (Notation::Timing, 1) => "lifeline",
            (Notation::Timing, _) => "lifelines",
            (_, 1) => "element",
            _ => "elements",
        };
        format!("{count} {}", tr(unit))
    }
    pub fn contents(&self) -> String {
        if self.count.is_none() {
            return tr("Can be restored and corrected in the editor.").into();
        }
        if self.names.is_empty() {
            return tr("No elements yet").into();
        }
        let mut contents = self.names.join(" · ");
        if let Some(count) = self.count.filter(|count| *count > 4) {
            contents.push_str(&crate::i18n::format(
                " · +{count} more",
                &[("count", (count - 4).to_string())],
            ));
        }
        contents
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_descriptions_follow_language_without_translating_document_labels() {
        let _language = crate::i18n::scoped_language(crate::i18n::Language::English);
        let details = Details::new("semantics {\n A(label=\"Source\")\n}\n");
        assert_eq!(details.count(), "1 element");
        assert_eq!(details.contents(), "Source");
        {
            let _german = crate::i18n::scoped_language(crate::i18n::Language::German);
            assert_eq!(details.kind(), "Generisches Knoten-Kanten-Diagramm");
            assert_eq!(details.count(), "1 Element");
            assert_eq!(details.contents(), "Source");
        }
        assert_eq!(details.count(), "1 element");
    }
}
