//! Direct contextual graph insertion: one source transaction, no render in menus.
use crate::{
    document::Edit,
    element_edit::{self, ElementKind},
    graph_edit,
    notation::{self, Notation},
    notation_edit::{self as edit, catalog},
    placement_edit,
};
use laml::syntax::Placement;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub enum Item {
    Element(ElementKind),
    Recipe(catalog::Recipe),
}
impl Item {
    pub fn title(self) -> &'static str {
        match self {
            Self::Element(k) => k.title(),
            Self::Recipe(r) => r.title,
        }
    }
}
pub struct Insertion {
    pub id: Option<String>,
    pub edits: Vec<Edit>,
}

pub fn items(source: &str) -> Vec<Item> {
    let mut out: Vec<_> = ElementKind::available(source)
        .into_iter()
        .map(Item::Element)
        .collect();
    out.extend(
        catalog::recipes(Notation::of(source))
            .into_iter()
            .filter(|r| {
                if r.title == "Insert interaction on flow" {
                    return true;
                }
                r.section == "semantics"
                    && r.layout.is_empty()
                    && !catalog::parameters(*r).iter().any(|p| {
                        matches!(
                            p.as_str(),
                            "from" | "to" | "participant" | "relation" | "third"
                        )
                    })
                    && !r.body.lines().any(|line| {
                        line.contains("-->") || line.contains("--^") || line.contains("..>")
                    })
                    && !out_title_duplicate(r.title)
            })
            .map(Item::Recipe),
    );
    out
}
fn out_title_duplicate(title: &str) -> bool {
    matches!(title, "Part")
}
fn values(source: &str, from: &str, recipe: catalog::Recipe) -> BTreeMap<String, String> {
    let fresh = (1..)
        .map(|n| format!("Element{n}"))
        .find(|id| !source.contains(id))
        .unwrap();
    catalog::parameters(recipe)
        .into_iter()
        .map(|key| {
            let v = match key.as_str() {
                "id" => fresh.clone(),
                "label" => recipe.title.into(),
                "name" => "member".into(),
                "classifier" if recipe.title == "Metaclass" => [
                    "Element",
                    "Class",
                    "Component",
                    "Artifact",
                    "Activity",
                    "Interface",
                    "Package",
                    "Port",
                    "Property",
                    "State",
                    "UseCase",
                ]
                .into_iter()
                .find(|name| {
                    !source
                        .lines()
                        .any(|line| line.trim() == format!("metaclass {name}"))
                })
                .unwrap_or("Element")
                .into(),
                "classifier" => "String".into(),
                "parameter" => "T".into(),
                "value" => "Value".into(),
                "description" => "Description".into(),
                "technology" => "Technology".into(),
                "effect" => "effect".into(),
                "child" => format!("{fresh}_Child"),
                "start" => format!("{fresh}_Start"),
                _ => from.into(),
            };
            (key, v)
        })
        .collect()
}

/// Insert actual semantic content, without a relation to the enclosing frame.
pub fn plan_inside(source: &str, parent: &str, item: Item) -> Result<Insertion, String> {
    let before = notation::parse_graph(source).map_err(|e| e.message)?;
    let container = before
        .nodes
        .iter()
        .find(|n| n.id == parent)
        .ok_or("Missing container")?;
    if !crate::node_edit::blocks(source, &before)?.contains_key(&container.line) {
        let range = crate::source::SourceIndex::new(source)
            .line(container.line)
            .ok_or("Missing container line")?;
        let line = &source[range.clone()];
        let code = crate::style_edit::code(line).trim_end();
        let at = range.start + code.len();
        let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
        let nl = element_edit::newline(source);
        let end = range.end;
        let opened = edit::apply(
            source,
            &[
                Edit {
                    range: at..at,
                    replacement: " {".into(),
                },
                Edit {
                    range: end..end,
                    replacement: format!("{nl}{indent}}}"),
                },
            ],
        )?;
        let p = plan_inside(&opened, parent, item)?;
        let candidate = edit::apply(&opened, &p.edits)?;
        return Ok(Insertion {
            id: p.id,
            edits: graph_edit::difference(source, &candidate),
        });
    }
    let (id, edits) = match item {
        Item::Element(kind) => {
            let label = if matches!(
                Notation::of(source),
                Notation::Profile | Notation::InteractionOverview
            ) {
                ""
            } else {
                kind.title()
            };
            let (id, edits) = element_edit::add(source, kind, label, Some(parent), None)?;
            (Some(id), edits)
        }
        Item::Recipe(recipe) => {
            let edits = catalog::create(
                source,
                recipe,
                &values(source, parent, recipe),
                Some(container.line),
                None,
            )?;
            (None, edits)
        }
    };
    let candidate = edit::apply(source, &edits)?;
    notation::inspect(&candidate)?;
    let after = notation::parse_graph(&candidate).map_err(|e| e.message)?;
    let created: Vec<_> = after
        .nodes
        .iter()
        .filter(|n| !before.nodes.iter().any(|old| old.id == n.id))
        .collect();
    if created.is_empty()
        || created.iter().any(|n| {
            n.parent.as_deref() != Some(parent)
                && !created.iter().any(|p| n.parent.as_deref() == Some(&p.id))
        })
    {
        return Err("Choose an element supported inside this container".into());
    }
    Ok(Insertion {
        id: id.or_else(|| created.first().map(|n| n.id.clone())),
        edits,
    })
}

pub fn plan(
    source: &str,
    from: &str,
    side: Placement,
    item: Item,
    operator: &str,
) -> Result<Insertion, String> {
    let before = notation::parse_graph(source).map_err(|e| e.message)?;
    let reference = before
        .nodes
        .iter()
        .find(|n| n.id == from)
        .ok_or("Missing insertion reference")?;
    let mut candidate;
    let mut id = None;
    match item {
        Item::Element(kind) => {
            let (created, edits) = element_edit::add(
                source,
                kind,
                if matches!(
                    Notation::of(source),
                    Notation::Profile | Notation::InteractionOverview
                ) {
                    ""
                } else {
                    kind.title()
                },
                reference.parent.as_deref(),
                Some((from, side)),
            )?;
            candidate = edit::apply(source, &edits)?;
            id = Some(created);
        }
        Item::Recipe(recipe) => {
            let member = !catalog::parameters(recipe).iter().any(|k| k == "id");
            let parent = if recipe.title == "Metaclass" {
                None
            } else if member {
                Some(reference.line)
            } else {
                reference
                    .parent
                    .as_ref()
                    .and_then(|p| before.nodes.iter().find(|n| &n.id == p))
                    .map(|n| n.line)
            };
            let mut parameters = values(source, from, recipe);
            if recipe.title == "Insert interaction on flow" {
                let mut flows = before
                    .relations
                    .iter()
                    .filter(|r| r.from == from)
                    .collect::<Vec<_>>();
                if flows.is_empty() {
                    flows = before.relations.iter().filter(|r| r.to == from).collect();
                }
                if flows.len() != 1 {
                    return Err("Choose an element with one unambiguous incident flow".into());
                }
                parameters.insert("from".into(), flows[0].from.clone());
                parameters.insert("to".into(), flows[0].to.clone());
                parameters.insert("classifier".into(), "Interaction".into());
            }
            let edits = catalog::create(source, recipe, &parameters, parent, None)?;
            candidate = edit::apply(source, &edits)?;
            let after = notation::parse_graph(&candidate).map_err(|e| e.message)?;
            if let Some(created) = after
                .nodes
                .iter()
                .find(|n| !before.nodes.iter().any(|old| old.id == n.id))
            {
                id = Some(created.id.clone());
                let placement = placement_edit::plan(&candidate, &created.id, from, side)?;
                candidate = placement.source;
            }
        }
    }
    // Invalid defaults never prevent element insertion and never choose a replacement.
    if let Some(id) = &id {
        let already_connected = notation::parse_graph(&candidate)
            .map_err(|e| e.message)?
            .relations
            .iter()
            .any(|r| r.from == from && r.to == *id);
        let is_note = notation::parse_graph(&candidate)
            .map_err(|e| e.message)?
            .nodes
            .iter()
            .any(|n| n.id == *id && n.kind == laml::syntax::NodeKind::Note);
        let op = if is_note { ".." } else { operator };
        let label = Notation::of(source).c4().then_some("Uses");
        if !already_connected
            && let Ok((_, edits)) =
                graph_edit::add_relation_labeled(&candidate, from, id, op, label, None, None)
            && let Ok(connected) = edit::apply(&candidate, &edits)
            && notation::inspect(&connected).is_ok()
        {
            candidate = connected;
        }
    }
    notation::inspect(&candidate)?;
    Ok(Insertion {
        id,
        edits: graph_edit::difference(source, &candidate),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_container_content_is_nested_without_relation_and_undoable() {
        for source in [
            "semantics {\r\n package P {\r\n }\r\n}\r\n",
            "settings {\n type: package\n}\nsemantics {\n package P // keep\n}\n",
        ] {
            let kind = if Notation::of(source) == Notation::Package {
                ElementKind::Package
            } else {
                ElementKind::Rectangle
            };
            let p = plan_inside(source, "P", Item::Element(kind)).unwrap();
            let mut doc = crate::document::Document::new(source.into());
            doc.apply(0, "Insert content", p.edits).unwrap();
            let graph = notation::parse_graph(doc.source()).unwrap();
            assert_eq!(graph.nodes.len(), 2);
            assert_eq!(graph.nodes[1].parent.as_deref(), Some("P"));
            assert!(graph.relations.is_empty());
            if source.contains("// keep") {
                assert!(doc.source().contains("// keep"));
            }
            doc.undo();
            assert_eq!(doc.source(), source);
        }
    }
    #[test]
    fn add_and_connect_preserves_source_and_is_one_undo() {
        let source = "semantics {\r\n A // äö\r\n}\r\n";
        let p = plan(
            source,
            "A",
            Placement::Left,
            Item::Element(ElementKind::Rectangle),
            "->",
        )
        .unwrap();
        let mut doc = crate::document::Document::new(source.into());
        doc.apply(0, "Insert", p.edits).unwrap();
        let g = notation::parse_graph(doc.source()).unwrap();
        assert_eq!(g.relations.len(), 1);
        assert_eq!(g.relations[0].from, "A");
        assert!(doc.source().contains(" A // äö\r\n"));
        assert!(doc.source().contains("Node1 < A"));
        doc.undo();
        assert_eq!(doc.source(), source);
    }
    #[test]
    fn unsupported_default_leaves_new_element_unconnected() {
        let source = "settings {\n type: package\n}\nsemantics {\n package A {\n }\n}\n";
        let p = plan(
            source,
            "A",
            Placement::Right,
            Item::Element(ElementKind::Package),
            "->",
        )
        .unwrap();
        let changed = edit::apply(source, &p.edits).unwrap();
        let g = notation::parse_graph(&changed).unwrap();
        assert_eq!(g.nodes.len(), 2);
        assert!(g.relations.is_empty());
    }
    #[test]
    fn insertion_keeps_membership_and_notes_use_associations() {
        let source = "semantics {\n package P {\n A\n }\n}\n";
        let p = plan(
            source,
            "P.A",
            Placement::Below,
            Item::Element(ElementKind::Note),
            "->",
        )
        .unwrap();
        let changed = edit::apply(source, &p.edits).unwrap();
        let g = notation::parse_graph(&changed).unwrap();
        assert_eq!(g.nodes.last().unwrap().parent.as_deref(), Some("P"));
        assert_eq!(g.relations.len(), 1);
    }
    #[test]
    fn every_graph_notation_has_locally_valid_contextual_insertions() {
        for notation in Notation::ALL.into_iter().filter(|n| n.graph()) {
            let source = notation.starter(false);
            let graph = notation::parse_graph(&source).unwrap();
            if graph.nodes.is_empty() {
                continue;
            }
            let choices = items(&source);
            let operator = notation.relation_types().first().map_or("->", |r| r.key);
            assert!(
                graph
                    .nodes
                    .iter()
                    .any(|node| choices.iter().any(|item| plan(
                        &source,
                        &node.id,
                        Placement::Right,
                        *item,
                        operator
                    )
                    .is_ok())),
                "{} must have applicable insertion controls",
                notation.key()
            );
        }
    }
}
