//! Revision-bound compiler jobs, including preflight for visual transactions.
use laml_ui::{document::Edit, preview::Preview};
use std::path::PathBuf;
mod client;
pub use client::Client;

pub enum Expectation {
    Notation { line: Option<usize>, id: Option<String> },
    Settings(laml::syntax::Settings),
    CompartmentStyle {
        owner: String,
        kind: String,
        value: Option<String>,
    },
    MemberStyle {
        owner: String,
        address: String,
        name: String,
        value: Option<String>,
    },
    Styles {
        ids: Vec<String>,
        properties: Vec<(String, Option<String>)>,
    },
    Style {
        ids: Vec<String>,
        property: String,
        value: Option<String>,
    },
    Alignment {
        ids: Vec<String>,
        align: laml::syntax::FreeAlignment,
    },
    Node(String),
    Element {
        id: String,
        parent: Option<String>,
        kind: laml_ui::element_edit::ElementKind,
    },
    Membership {
        id: String,
        parent: Option<String>,
    },
    Port(String),
    Ports(Vec<String>),
    PortStyle {
        ids: Vec<String>,
        property: String,
        value: Option<String>,
    },
    Nodes(Vec<String>),
    Deleted {
        nodes: Vec<String>,
        ports: Vec<String>,
    },
    Relation(String),
    RelationEdit(Vec<usize>),
    RelationStyle {
        lines: Vec<usize>,
        property: String,
        value: Option<String>,
    },
}
pub struct Command {
    pub edits: Vec<Edit>,
    pub label: String,
    pub expectation: Expectation,
}
pub type Job = (u64, String, PathBuf, Option<Command>);
pub type BuildResult = (
    u64,
    Result<Preview, laml_ui::diagnostic::Diagnostic>,
    Option<Command>,
);

#[cfg(test)]
pub fn run(backend: &dyn laml_ui::backend::Backend, job: Job) -> BuildResult {
    let (revision, source, path, command) = job;
    let result = backend
        .compile(&source, &path, laml_ui::backend::Presentation::Document)
        .and_then(|preview| {
            if let Some(command) = &command {
                verify(&preview, &command.expectation, laml_ui::backend::Presentation::Document)?;
            }
            Ok(preview)
        });
    (revision, result, command)
}

fn verify(preview: &Preview, expectation: &Expectation, presentation: laml_ui::backend::Presentation) -> Result<(), String> {
    if matches!(expectation, Expectation::Notation { .. }) { return Ok(()); }
    if let Expectation::Settings(expected) = expectation {
        let mut expected = expected.clone();
        // Match the selected preview presentation, while authored settings are
        // validated from the patched source before submitting the transaction.
        if let Some(theme) = presentation.theme_override() {
            expected.theme = theme;
            expected.transparent = true;
        }
        return if preview.diagram.settings == expected { Ok(()) }
            else { Err("Document setting did not match the compiled result".into()) };
    }
    if let Expectation::Styles { ids, properties } = expectation {
        for (property, value) in properties {
            verify(
                preview,
                &Expectation::Style {
                    ids: ids.clone(),
                    property: property.clone(),
                    value: value.clone(),
                },
                presentation,
            )?;
        }
        return Ok(());
    }
    if let Expectation::CompartmentStyle { owner, kind, value } = expectation {
        if value
            .as_ref()
            .is_some_and(|v| laml_ui::structure::compartment_value(preview, owner, kind) != *v)
        {
            return Err("Compartment override is shadowed; source unchanged".into());
        }
        return Ok(());
    }
    if let Expectation::MemberStyle {
        owner,
        address,
        name,
        value,
    } = expectation
    {
        let actual = laml_ui::structure::member_value(preview, owner, address, name);
        if value
            .as_ref()
            .is_some_and(|value| !laml_ui::properties::matches(actual.as_deref(), name, value))
        {
            return Err("Member override is shadowed or transformed; source unchanged".into());
        }
        return Ok(());
    }
    if let Expectation::PortStyle {
        ids,
        property,
        value,
    } = expectation
    {
        for id in ids {
            let actual = preview
                .port_properties
                .get(id)
                .and_then(|p| p.get(property));
            if let Some(value) = value
                && !laml_ui::properties::matches(
                    actual.and_then(|p| p.value.as_deref()),
                    property,
                    value,
                )
            {
                return Err(format!(
                    "The local {property} override for port {id} is shadowed or transformed. Source unchanged."
                ));
            }
        }
        return Ok(());
    }
    if let Expectation::RelationStyle {
        lines,
        property,
        value,
    } = expectation
    {
        for line in lines {
            let actual = preview
                .relation_properties
                .get(line)
                .and_then(|p| p.get(property));
            if let Some(value) = value {
                let matched = laml_ui::properties::matches(
                    actual.and_then(|p| p.value.as_deref()),
                    property,
                    value,
                );
                if !matched {
                    return Err(
                        "Local relation style is shadowed or transformed. Source unchanged.".into(),
                    );
                }
            }
        }
        return Ok(());
    }
    let Expectation::Style {
        ids,
        property,
        value,
    } = expectation
    else {
        let present =
            match expectation {
                Expectation::Alignment { ids, align } => ids.last().is_some_and(|reference| {
                    ids[..ids.len() - 1].iter().all(|id| {
                        preview.diagram.alignments.iter().any(|rule| {
                            laml_ui::alignment_edit::matches(rule, id, reference, *align)
                        })
                    })
                }),
                Expectation::RelationEdit(lines) => lines
                    .iter()
                    .all(|line| preview.diagram.relations.iter().any(|r| r.line == *line)),
                Expectation::Nodes(ids) => ids
                    .iter()
                    .all(|id| preview.diagram.nodes.iter().any(|node| &node.id == id)),
                Expectation::Deleted { nodes, ports } => {
                    nodes
                        .iter()
                        .all(|id| preview.diagram.nodes.iter().all(|n| &n.id != id))
                        && ports
                            .iter()
                            .all(|id| laml_ui::port_edit::find(&preview.diagram, id).is_none())
                }
                Expectation::Membership { id, parent } => preview
                    .diagram
                    .nodes
                    .iter()
                    .any(|n| &n.id == id && &n.parent == parent),
                Expectation::Port(id) => laml_ui::port_edit::find(&preview.diagram, id).is_some(),
                Expectation::Ports(ids) => ids
                    .iter()
                    .all(|id| laml_ui::port_edit::find(&preview.diagram, id).is_some()),
                Expectation::Element { id, parent, kind } => {
                    preview
                        .diagram
                        .nodes
                        .iter()
                        .any(|n| &n.id == id && &n.parent == parent && n.kind == kind.node_kind())
                        && preview.nodes.iter().any(|n| &n.id == id)
                        && kind.shape().is_none_or(|shape| {
                            preview
                                .node_properties
                                .get(id)
                                .and_then(|p| p.get("shape"))
                                .and_then(|p| p.value.as_deref())
                                == Some(shape)
                        })
                }
                Expectation::Node(id) => preview.diagram.nodes.iter().any(|node| &node.id == id),
                Expectation::Relation(id) => preview
                    .diagram
                    .relations
                    .iter()
                    .any(|relation| relation.name.as_ref() == Some(id)),
                _ => unreachable!(),
            };
        return if present {
            Ok(())
        } else {
            Err("Created element was not found in the compiled diagram. Source unchanged.".into())
        };
    };
    let Some(expected) = value else {
        return Ok(());
    };
    for id in ids {
        let property_value = preview
            .node_properties
            .get(id)
            .and_then(|values| values.get(property));
        let matches = laml_ui::properties::matches(
            property_value.and_then(|p| p.value.as_deref()),
            property,
            expected,
        );
        if !matches {
            return Err(format!(
                "The local {property} override for {id} is shadowed or transformed by another rule. Source unchanged."
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editor_jobs_respect_document_theme_by_default() {
        let source = "settings {\n theme: modern\n background: opaque\n}\nsemantics {\n A\n}\n";
        let document = laml_ui::document::Document::new(source.into());
        let (revision, preview, _) = run(
            &laml_ui::backend::LocalBackend,
            (0, source.into(), PathBuf::from("test.laml"), None),
        );
        let preview = preview.unwrap();
        assert_eq!(revision, 0);
        assert_eq!(
            preview.diagram.settings.theme,
            laml::theme::Theme::Modern
        );
        assert!(!preview.diagram.settings.transparent);
        assert!(preview.svg.contains("#ffffff"));
        assert_eq!(document.source(), source);
        assert_eq!(document.revision(), 0);
        assert!(!document.is_dirty());
    }

    #[test]
    fn rejected_visual_transaction_does_not_modify_document() {
        let source = "semantics {\n A\n}\n";
        let document = laml_ui::document::Document::new(source.into());
        let result = run(
            &laml_ui::backend::LocalBackend,
            (
                0,
                source.into(),
                PathBuf::from("test.laml"),
                Some(Command {
                    edits: vec![],
                    label: "Style".into(),
                    expectation: Expectation::Style {
                        ids: vec!["A".into()],
                        property: "fill".into(),
                        value: Some("#123456".into()),
                    },
                }),
            ),
        );
        assert!(result.1.is_err());
        assert_eq!(document.source(), source);
    }
}
